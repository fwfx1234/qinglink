//! 配对请求的转发与取消。
//!
//! 快照里展示的待配对请求只能由桌面端取消；配对状态机仍归服务进程所有，
//! 这里不复制任何 PIN 或设备信息。

use crate::windows_service_impl::management::record::{CommandError, CommandResult, value};
use anyhow::Context;
use qinglink_server::AppState;

/// Cancel a pending pairing request the desktop UI is showing.
pub(super) fn cancel_pairing(state: &AppState, pairing_id: &str) -> CommandResult {
    let id = pairing_id
        .parse()
        .context("invalid pairing request identifier")
        .map_err(CommandError::internal)?;
    let cancelled = state
        .pairing
        .lock()
        .map_err(|_| CommandError::internal("pairing state unavailable"))?
        .cancel(id);
    if !cancelled {
        return Err(CommandError::new(
            "NOT_FOUND",
            "配对请求已完成、过期或已取消",
        ));
    }
    value(serde_json::json!({"cancelled":true}))
}
