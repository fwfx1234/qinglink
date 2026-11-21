//! 退出序列：通知浏览器 → 停 IPC worker → 服务租约 → 数据库 → 日志 flush（最后）。
//!
//! 顺序是契约，见 `docs/architecture.md` §4 与 §9.1；调整顺序必须同步该文档。
//! `logs` 必须最后 drop，它负责 flush 并停掉写线程。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::bootstrap::Desktop;
use crate::web::state::{ServerEvent, WebState};

/// Grace period granted to an in-flight HTTP response before the process exits.
///
/// Quitting is triggered by an API call, so that call's response has to leave
/// the socket before the process tears the server down.
const RESPONSE_FLUSH: Duration = Duration::from_millis(400);

/// Why the main loop is ending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitRequest {
    /// Restart the desktop process.
    Restart,
    /// Quit, and ask the service to stop with it.
    Quit,
}

impl ExitRequest {
    /// 日志与浏览器事件里使用的稳定原因标记。
    fn reason(self) -> &'static str {
        match self {
            Self::Restart => "restart",
            Self::Quit => "exit",
        }
    }
}

/// 离开主循环前的收尾：先通知浏览器，再按需拉起替代进程。
///
/// 先回包再退出是刻意的：退出是由 API 调用触发的，那个响应必须先离开 socket。
/// 等待窗口就是 [`RESPONSE_FLUSH`]。
pub fn prepare(request: ExitRequest, web_state: &Arc<WebState>) {
    let reason = request.reason();
    tracing::info!(reason, "准备退出主循环");
    // Tell browsers first, then give the in-flight response time to
    // reach the client before the listener disappears.
    web_state.publish(ServerEvent::Shutdown(reason));
    std::thread::sleep(RESPONSE_FLUSH);
    if matches!(request, ExitRequest::Restart)
        && let Err(error) = spawn_restart()
    {
        tracing::error!(%error, "重启失败");
    }
}

/// 主循环结束后的关闭动作；顺序不可调整。
pub fn shutdown(desktop: Desktop, request: ExitRequest) {
    let quit = matches!(request, ExitRequest::Quit);
    tracing::info!(quit, "桌面端正在关闭");

    let Desktop {
        logs,
        web_state,
        runtime,
        lifecycle,
        database,
        ..
    } = desktop;

    runtime.shutdown();
    if let Err(error) = lifecycle.shutdown(quit) {
        tracing::warn!(%error, "关闭服务租约失败");
    }
    database.shutdown();
    // `logs` drops here, which flushes and stops the writer thread.
    drop(logs);
    drop(web_state);
}

/// Launch a replacement instance and let this one exit.
fn spawn_restart() -> Result<()> {
    let executable = std::env::current_exe().context("cannot resolve the executable path")?;
    let parent = std::process::id();
    std::process::Command::new(executable)
        .arg("--restart-parent")
        .arg(parent.to_string())
        .spawn()
        .context("cannot spawn the replacement process")?;
    Ok(())
}
