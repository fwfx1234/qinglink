//! 设备列表与撤销。
//!
//! 撤销会同时关闭该设备在普通连接与输入连接两个注册表中的会话，语义必须与
//! 移动端自助撤销一致，不得只关闭其中一个。

use crate::windows_service_impl::management::page::{bounded_page_with_total, check_page};
use crate::windows_service_impl::management::record::{CommandError, CommandResult, value};
use qinglink_protocol::service_ipc::DeviceSummary;
use qinglink_server::AppState;

/// List active devices with the composite id the desktop sends back on revoke.
pub(super) fn list_devices(state: &AppState, offset: usize, limit: usize) -> CommandResult {
    check_page(offset, limit)?;
    let total = state.token_store.list_active().len();
    let devices = state.token_store.list_devices_offset(offset, limit);
    let more = devices.len() == limit;
    let items = devices
        .into_iter()
        .map(|(_, device)| DeviceSummary {
            id: format!(
                "{}:{}",
                if device.device_id.is_some() {
                    "paired"
                } else {
                    "legacy"
                },
                device.id
            ),
            device_name: device.device_name.chars().take(256).collect(),
        })
        .collect();
    bounded_page_with_total(items, offset, more, Some(total), None)
}

/// Revoke one device by its `paired:`/`legacy:` prefixed identifier.
pub(super) fn revoke_device(state: &AppState, device_id: &str) -> CommandResult {
    let hashes = if let Some(id) = device_id.strip_prefix("paired:") {
        state.token_store.revoke_device(id)
    } else if let Some(id) = device_id.strip_prefix("legacy:") {
        state.token_store.revoke_legacy(id)
    } else {
        return Err(CommandError::new(
            "BAD_REQUEST",
            "invalid device identifier",
        ));
    };
    if hashes.is_empty() {
        return Err(CommandError::new(
            "NOT_FOUND",
            "设备不存在、已撤销或存储操作失败",
        ));
    }
    state
        .connection_registry
        .close_hashes(hashes.iter(), "REVOKED");
    state
        .input_connection_registry
        .close_hashes(hashes.iter(), "REVOKED");
    value(serde_json::json!({"revoked":true}))
}
