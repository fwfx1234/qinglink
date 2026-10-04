//! worker 线程主循环：排空调用队列、按退避重连、驱动快照刷新。

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use qinglink_protocol::service_ipc::DesktopCommand;

use super::envelope::{ensure_negotiated, exchange};
use super::inner::{Inner, Job};
use super::snapshot::{mark_disconnected, refresh_snapshot};
use super::transport::Transport;
use crate::service::ServiceError;

pub(super) fn worker_loop(inner: &Arc<Inner>, jobs: &Receiver<Job>, mut transport: impl Transport) {
    let mut failures = 0_u32;
    let mut retry_at = Instant::now();
    let mut next_snapshot = Instant::now();
    let mut seen_wake = inner.wake.load(Ordering::Acquire);

    while !inner.stop.load(Ordering::Acquire) {
        // Always drain the queue, even during an outage, so no caller is left
        // waiting for an answer that will never come.
        while let Ok(job) = jobs.try_recv() {
            let outcome = if Instant::now() < retry_at {
                Err(ServiceError::Unavailable(
                    "服务连接已中断，操作未执行，请重试".into(),
                ))
            } else {
                execute(&mut transport, &job.command, inner)
            };
            if let Err(error) = &outcome {
                tracing::debug!(command = ?job.command, %error, "service command failed");
            }
            // A dropped receiver just means the caller timed out.
            let _ = job.reply.send(outcome);
            // Re-read authoritative state instead of trusting the ack.
            next_snapshot = Instant::now();
        }

        // An explicit reconnect request drops the backoff and re-negotiates now.
        let wake = inner.wake.load(Ordering::Acquire);
        if wake != seen_wake {
            seen_wake = wake;
            failures = 0;
            retry_at = Instant::now();
            next_snapshot = Instant::now();
        }

        if Instant::now() >= retry_at {
            match refresh_snapshot(&mut transport, inner, &mut next_snapshot) {
                Ok(()) => failures = 0,
                Err(error) => {
                    failures = failures.saturating_add(1);
                    retry_at = Instant::now() + backoff(failures);
                    mark_disconnected(inner, &error);
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
    }

    // Release anything still queued so callers fail fast instead of hanging.
    while let Ok(job) = jobs.try_recv() {
        let _ = job
            .reply
            .send(Err(ServiceError::Unavailable("桌面端正在退出".into())));
    }
}

fn execute(
    transport: &mut impl Transport,
    command: &DesktopCommand,
    inner: &Arc<Inner>,
) -> Result<serde_json::Value, ServiceError> {
    ensure_negotiated(transport, inner)?;
    exchange(transport, command.clone(), inner)
}

/// Backoff between reconnect attempts once the service is unreachable.
fn backoff(failures: u32) -> Duration {
    Duration::from_secs(match failures {
        0 | 1 => 2,
        2 => 4,
        3 => 8,
        _ => 15,
    })
}
