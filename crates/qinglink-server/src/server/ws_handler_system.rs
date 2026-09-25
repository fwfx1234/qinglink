use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::*;

fn allow_secure_attention() -> bool {
    static LAST_SAS: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    let mut last = crate::lock_unpoisoned(LAST_SAS.get_or_init(|| Mutex::new(None)));
    if last.is_some_and(|instant| instant.elapsed() < Duration::from_secs(3)) {
        return false;
    }
    *last = Some(Instant::now());
    true
}

pub(super) fn handle(state: &AppState, req: &WsClientRequest, _session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === System ===
        WsClientRequest::GetSystemStatus => {
            let status = state.system_service.get_status();
            WsServerMessage::Response(WsServerResponse::SystemStatus(status))
        }
        WsClientRequest::GetMacAddress => match state.system_service.get_mac_address() {
            Some(mac) => {
                WsServerMessage::Response(WsServerResponse::MacAddress { mac_address: mac })
            }
            None => WsServerMessage::Response(WsServerResponse::error(
                "MAC_ERROR",
                "Unable to get MAC address",
            )),
        },
        WsClientRequest::GetSubnetPrefix => match state.system_service.get_subnet_prefix() {
            Some(prefix) => WsServerMessage::Response(WsServerResponse::SubnetPrefix { prefix }),
            None => WsServerMessage::Response(WsServerResponse::error(
                "SUBNET_ERROR",
                "Unable to get subnet prefix",
            )),
        },
        WsClientRequest::GetWakeNetworks => WsServerMessage::Response(
            WsServerResponse::WakeNetworks { networks: state.system_service.get_wake_networks() }
        ),
        WsClientRequest::Shutdown { force, delay_secs } => {
            match state.system_service.shutdown(*force, *delay_secs) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "SHUTDOWN_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::Sleep { hibernate } => match state.system_service.sleep(*hibernate) {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("SLEEP_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::Restart { force, delay_secs } => {
            match state.system_service.restart(*force, *delay_secs) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "RESTART_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::Logoff => match state.system_service.logoff() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("LOGOFF_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::Lock => match state.system_service.lock() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("LOCK_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::GetUnlockStatus => match state.system_service.get_unlock_status() {
            Ok(status) => WsServerMessage::Response(WsServerResponse::UnlockStatus(status)),
            Err(e) => WsServerMessage::Response(WsServerResponse::error(
                "UNLOCK_STATUS_FAILED",
                &e.to_string(),
            )),
        },
        WsClientRequest::AutoUnlock { pin } => match state.system_service.auto_unlock(pin.as_deref()) {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(error) => {
                let message = error.to_string();
                let code = message
                    .split_once(':')
                    .map(|(code, _)| code)
                    .unwrap_or("AUTO_UNLOCK_FAILED");
                WsServerMessage::Response(WsServerResponse::error(code, &message))
            }
        },
        WsClientRequest::GetLockInputStatus => match crate::platform::get_lock_input_status() {
            Ok(status) => WsServerMessage::Response(WsServerResponse::LockInputStatus(status)),
            Err(error) => WsServerMessage::Response(WsServerResponse::error(
                "LOCK_INPUT_STATUS_FAILED",
                &error.to_string(),
            )),
        },
        WsClientRequest::SubmitLockPin { pin } => {
            tracing::info!(length = pin.len(), "submit_lock_pin: submitting PIN for Windows login");
            match crate::platform::submit_lock_pin(pin) {
                Ok(result) => {
                    tracing::info!(%result.message, "submit_lock_pin: PIN submitted");
                    WsServerMessage::Response(WsServerResponse::LockInputResult(result))
                }
                Err(error) => {
                    tracing::error!(%error, "submit_lock_pin: failed to submit PIN");
                    let message = error.to_string();
                    let code = message
                        .split_once(':')
                        .map(|(code, _)| code)
                        .unwrap_or("LOCK_INPUT_FAILED");
                    WsServerMessage::Response(WsServerResponse::error(code, &message))
                }
            }
        }
        WsClientRequest::GetSecureDesktopStatus => match crate::platform::get_secure_desktop_status() {
            Ok(status) => {
                tracing::debug!(
                    desktop = %status.desktop,
                    locked = status.locked,
                    uac_detected = status.uac_detected,
                    input_ready = status.input_ready,
                    "secure desktop status requested"
                );
                WsServerMessage::Response(WsServerResponse::SecureDesktopStatus(status))
            }
            Err(error) => {
                tracing::error!(%error, "failed to get secure desktop status");
                WsServerMessage::Response(WsServerResponse::error(
                    "SECURE_DESKTOP_STATUS_FAILED",
                    &error.to_string(),
                ))
            }
        },
        WsClientRequest::SendSecureAscii { text } => {
            if text.is_empty() || text.len() > 256 || !text.bytes().all(|byte| (0x20..=0x7e).contains(&byte)) {
                tracing::warn!(length = text.len(), "rejecting invalid secure ASCII input");
                WsServerMessage::Response(WsServerResponse::error(
                    "INVALID_SECURE_TEXT",
                    "安全输入仅支持 1-256 个 ASCII 可打印字符",
                ))
            } else {
                match crate::platform::send_secure_ascii(text) {
                    Ok(()) => {
                        tracing::info!(length = text.len(), "secure ASCII input accepted and delivered");
                        WsServerMessage::Response(WsServerResponse::success())
                    }
                    Err(error) => {
                        tracing::error!(%error, length = text.len(), "secure ASCII input failed");
                        WsServerMessage::Response(WsServerResponse::error(
                            "SECURE_TEXT_FAILED",
                            &error.to_string(),
                        ))
                    }
                }
            }
        }
        WsClientRequest::SecureAttention => {
            if !allow_secure_attention() {
                tracing::warn!("secure attention sequence rate limited");
                WsServerMessage::Response(WsServerResponse::error(
                    "SECURE_ATTENTION_RATE_LIMITED",
                    "安全注意序列请求过于频繁，请稍后再试",
                ))
            } else {
                match crate::platform::send_secure_attention() {
                    Ok(()) => {
                        tracing::info!("secure attention sequence (Ctrl+Alt+Del) accepted and sent");
                        WsServerMessage::Response(WsServerResponse::success())
                    }
                    Err(error) => {
                        tracing::error!(%error, "secure attention sequence failed");
                        WsServerMessage::Response(WsServerResponse::error(
                            "SECURE_ATTENTION_FAILED",
                            &error.to_string(),
                        ))
                    }
                }
            }
        }

        _ => unreachable!("request routed to the wrong handler"),
    }
}


