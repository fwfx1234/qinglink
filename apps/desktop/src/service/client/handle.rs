//! 交给 Web 层的服务调用句柄。

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::TrySendError;
use std::time::{Duration, Instant};

use qinglink_protocol::service_ipc::{DesktopCommand, SERVICE_IPC_VERSION};
use serde::de::DeserializeOwned;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::{broadcast, oneshot};

use super::envelope::disconnected_message;
use super::inner::{Inner, Job};
use super::{ServiceEvent, ServiceLinkState, ServiceStatus};
use crate::service::{CALL_TIMEOUT, ServiceError, UPGRADE_HINT};

/// How often the blocking caller re-checks its reply channel.
///
/// Small enough that the answer is not delayed noticeably, large enough that a
/// thread waiting out a slow service costs almost nothing.
const CALL_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Handle used by the web layer to talk to the service.
#[derive(Clone, Debug)]
pub struct ServiceClient {
    inner: Arc<Inner>,
}

impl ServiceClient {
    /// 用共享内核构造句柄；内核由 [`ServiceRuntime`](super::runtime::ServiceRuntime) 持有。
    pub(super) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// Current service status.
    pub fn status(&self) -> ServiceStatus {
        match self.inner.state.lock() {
            Ok(state) => state.view(),
            Err(_) => ServiceStatus {
                connected: false,
                state: ServiceLinkState::Unreachable,
                expected_version: SERVICE_IPC_VERSION,
                service_version: None,
                paused: false,
                network_ready: false,
                address: None,
                last_error: Some("服务状态不可用".into()),
                capabilities: None,
            },
        }
    }

    /// Ask the worker to re-negotiate right now instead of waiting out its
    /// reconnect backoff.
    ///
    /// The capability cache is dropped as well, so the worker performs a fresh
    /// `capabilities` handshake rather than trusting an answer from a service
    /// that may have been repaired or restarted since.
    pub fn reconnect(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.capabilities = None;
            state.last_error = None;
            state.connected = false;
            state.link_state = ServiceLinkState::Connecting;
        }
        self.inner.wake.fetch_add(1, Ordering::Release);
    }

    /// The current pending pairing request, if one is live and the desktop is
    /// allowed to display it.
    pub fn pending_pairing(&self) -> Option<qinglink_protocol::service_ipc::PendingPairing> {
        let state = self.inner.state.lock().ok()?;
        if !state.connected {
            return None;
        }
        let snapshot = state.snapshot.pending_pairing.clone()?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        (snapshot.expires_at_unix > now).then_some(snapshot)
    }

    /// Active device count from the most recent snapshot, when one was received.
    ///
    /// Used to seed a browser that has to rebuild its view (for example after
    /// the event socket fell behind).
    pub fn device_count(&self) -> Option<usize> {
        let state = self.inner.state.lock().ok()?;
        if !state.connected {
            return None;
        }
        state.snapshot.device_count
    }

    /// Subscribe to status changes.
    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.inner.events.subscribe()
    }

    /// Send a command and await its answer.
    ///
    /// Fails immediately when the command is unsupported or the queue is full;
    /// the caller never waits on a service that is known to be down.
    ///
    /// # Errors
    /// [`ServiceError::Unavailable`] while the link is down or the queue is
    /// full, [`ServiceError::Unsupported`] for a command the service does not
    /// advertise, [`ServiceError::Timeout`] when the service misses the budget,
    /// and whatever the service itself refused with.
    pub async fn call(&self, command: DesktopCommand) -> Result<serde_json::Value, ServiceError> {
        self.ensure_callable(&command)?;
        let receiver = self.enqueue(command)?;
        match tokio::time::timeout(CALL_TIMEOUT, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(ServiceError::Internal("服务 worker 未返回结果".into())),
            Err(_) => Err(ServiceError::Timeout),
        }
    }

    /// Send a command and wait for its answer **without a Tokio runtime**.
    ///
    /// The main thread owns the Windows message pump and cannot await, but a
    /// prompt button still has to reach the same worker over the same bounded
    /// queue as every other caller — so the pre-checks, the reply channel and the
    /// answer semantics are identical to [`Self::call`], and only the waiting
    /// differs. Callers must not run this on a thread that has to stay
    /// responsive; the pairing prompt runs it on a worker thread of its own.
    ///
    /// # Errors
    /// Same classification as [`Self::call`], with `timeout` replacing the
    /// shared per-call budget.
    pub fn call_blocking(
        &self,
        command: DesktopCommand,
        timeout: Duration,
    ) -> Result<serde_json::Value, ServiceError> {
        self.ensure_callable(&command)?;
        let mut receiver = self.enqueue(command)?;
        let deadline = Instant::now() + timeout;
        loop {
            match receiver.try_recv() {
                Ok(result) => return result,
                Err(TryRecvError::Closed) => {
                    return Err(ServiceError::Internal("服务 worker 未返回结果".into()));
                }
                Err(TryRecvError::Empty) => {}
            }
            if Instant::now() >= deadline {
                return Err(ServiceError::Timeout);
            }
            std::thread::sleep(CALL_POLL_INTERVAL);
        }
    }

    /// Whether a command may be attempted right now.
    fn ensure_callable(&self, command: &DesktopCommand) -> Result<(), ServiceError> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| ServiceError::Internal("服务状态不可用".into()))?;
        if !state.connected {
            return Err(ServiceError::Unavailable(disconnected_message(
                state.link_state,
            )));
        }
        if !state.supports(command) {
            return Err(ServiceError::Unsupported(UPGRADE_HINT.into()));
        }
        Ok(())
    }

    /// Put one command on the worker queue and return its private reply channel.
    fn enqueue(
        &self,
        command: DesktopCommand,
    ) -> Result<oneshot::Receiver<Result<serde_json::Value, ServiceError>>, ServiceError> {
        let (reply, receiver) = oneshot::channel();
        self.inner
            .jobs
            .try_send(Job { command, reply })
            .map_err(|error| match error {
                TrySendError::Full(_) => {
                    ServiceError::Unavailable("服务命令队列已满，请稍后重试".into())
                }
                TrySendError::Disconnected(_) => {
                    ServiceError::Unavailable("服务命令通道已关闭".into())
                }
            })?;
        Ok(receiver)
    }

    /// Send a command and decode its typed payload.
    pub async fn call_typed<T: DeserializeOwned>(
        &self,
        command: DesktopCommand,
    ) -> Result<T, ServiceError> {
        let value = self.call(command).await?;
        serde_json::from_value(value)
            .map_err(|error| ServiceError::Internal(format!("服务响应格式错误：{error}")))
    }
}
