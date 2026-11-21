//! 管理管道命令的解码与分发。
//!
//! 这里是桌面管理协议的唯一入口：先按命令名白名单判定归属，再把 `extra`
//! 还原成 `DesktopCommand` 负载，最后统一按 `{ok, code, message, payload}`
//! 应答。分发失败与参数解码失败都走同一套错误编码。

use super::apps;
use super::catalog::{self, ListFilter};
use super::catalog_edit;
use super::devices;
use super::icons;
use super::pairing;
use super::record::{CommandError, CommandResult, value};
use super::snapshot;
use crate::windows_service_impl::authorization::DesktopPeer;
use crate::windows_service_impl::dispatch::Request;
use crate::windows_service_impl::pipe::Reply;
use anyhow::Result;
use qinglink_protocol::service_ipc::{Capabilities, DesktopCommand};
use qinglink_server::AppState;

/// Answer one desktop management command, or report that it is not ours.
///
/// Returns `Ok(false)` when `request.command` belongs to another dispatcher, so
/// the caller can keep matching on the same request.
pub(in crate::windows_service_impl) fn dispatch(
    reply: &mut Reply,
    request: &Request,
    state: &AppState,
    peer: DesktopPeer,
) -> Result<bool> {
    if !matches!(
        request.command.as_str(),
        "capabilities"
            | "get_snapshot"
            | "set_paused"
            | "set_keep_awake"
            | "cancel_pairing"
            | "list_devices"
            | "revoke_device"
            | "get_catalog"
            | "list_apps"
            | "get_app"
            | "rename_app"
            | "set_app_hidden"
            | "set_app_favorite"
            | "set_app_pinned"
            | "delete_app"
            | "rescan_apps"
            | "get_app_icon"
            | "refresh_app_icon"
            | "list_app_icons"
            | "get_catalog_stats"
            | "add_app"
            | "launch_app"
    ) {
        return Ok(false);
    }
    let command = decode_desktop_command(request);
    match command.map(|command| execute(state, peer, command)) {
        Ok(Ok(payload)) => reply.send(
            &serde_json::json!({"ok":true,"code":"OK","message":"操作完成","payload":payload}),
        )?,
        Ok(Err(error)) => reply.send(&serde_json::json!({
            "ok": false,
            "code": error.code,
            "message": error.message,
        }))?,
        Err(error) => reply.send(
            &serde_json::json!({"ok":false,"code":"BAD_REQUEST","message":error.to_string()}),
        )?,
    }
    Ok(true)
}

/// Rebuild the typed command from the request envelope.
///
/// `Capabilities` and `GetCatalogStats` are unit variants: the tagged enum
/// expects no `payload` key at all for them, so an empty payload object makes
/// serde fail with "invalid type: map, expected unit variant". Attach the
/// payload only when the request actually carried fields.
fn decode_desktop_command(request: &Request) -> Result<DesktopCommand> {
    if request.command == "capabilities" {
        return Ok(DesktopCommand::Capabilities);
    }
    let mut value = serde_json::json!({ "command": request.command });
    if !request.extra.is_empty() {
        value["payload"] = serde_json::Value::Object(request.extra.clone().into_iter().collect());
    }
    serde_json::from_value::<DesktopCommand>(value).map_err(anyhow::Error::from)
}

fn execute(state: &AppState, peer: DesktopPeer, command: DesktopCommand) -> CommandResult {
    match command {
        DesktopCommand::Capabilities => value(Capabilities {
            management: true,
            pairing: true,
            pause: true,
            devices: true,
            catalog: true,
            catalog_management: true,
            app_add: true,
            app_launch: true,
            app_icon_manage: true,
            keep_awake: true,
        }),
        DesktopCommand::GetSnapshot { desktop_unlocked } => {
            state
                .control
                .observe_desktop(peer.session, desktop_unlocked);
            snapshot::snapshot(state)
        }
        DesktopCommand::SetPaused { paused } => {
            state.set_paused(paused).map_err(CommandError::internal)?;
            snapshot::snapshot(state)
        }
        DesktopCommand::SetKeepAwake { enabled } => {
            state
                .set_keep_awake(enabled)
                .map_err(CommandError::internal)?;
            value(state.keep_awake())
        }
        DesktopCommand::CancelPairing { pairing_id } => pairing::cancel_pairing(state, &pairing_id),
        DesktopCommand::ListDevices { offset, limit } => {
            devices::list_devices(state, offset, limit)
        }
        DesktopCommand::RevokeDevice { device_id } => devices::revoke_device(state, &device_id),
        DesktopCommand::GetCatalog { offset, limit } => catalog::get_catalog(state, offset, limit),
        DesktopCommand::ListApps {
            kind,
            offset,
            limit,
            query,
            include_hidden,
            favorite_only,
        } => catalog::list_apps(
            state,
            ListFilter {
                kind,
                offset,
                limit,
                query,
                include_hidden,
                favorite_only,
            },
        ),
        DesktopCommand::GetApp { id } => catalog::get_app(state, &id),
        DesktopCommand::RenameApp { id, name } => catalog_edit::rename_app(state, &id, &name),
        DesktopCommand::SetAppHidden { id, hidden } => {
            catalog_edit::set_app_hidden(state, &id, hidden)
        }
        DesktopCommand::SetAppFavorite { id, favorite } => {
            catalog_edit::set_app_favorite(state, &id, favorite)
        }
        DesktopCommand::SetAppPinned { id, pinned } => {
            catalog_edit::set_app_pinned(state, &id, pinned)
        }
        DesktopCommand::DeleteApp { id } => catalog_edit::delete_app(state, &id),
        DesktopCommand::RescanApps { kind } => apps::rescan_apps(state, &kind),
        DesktopCommand::GetAppIcon { id } => icons::get_app_icon(state, &id),
        DesktopCommand::RefreshAppIcon { id, icon_path } => {
            icons::refresh_app_icon(state, &id, icon_path.as_deref())
        }
        DesktopCommand::ListAppIcons { id } => icons::list_app_icons(state, &id),
        DesktopCommand::GetCatalogStats => catalog::get_catalog_stats(state),
        DesktopCommand::AddApp {
            path,
            name,
            kind,
            args,
        } => apps::add_app(state, &path, &name, &kind, &args),
        DesktopCommand::LaunchApp { id } => apps::launch_app(state, &id),
    }
}
