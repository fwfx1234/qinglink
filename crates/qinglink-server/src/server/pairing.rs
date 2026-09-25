use axum::{Extension, Json, extract::Path};
use qinglink_domain::PairingError;
use qinglink_protocol::{HealthResponse, PairingConfirm, PairingRequest, PairingTokenResponse};
use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::AppState;

#[derive(Clone, Debug, Serialize)]
pub struct PairingNotification {
    pub request_id: Uuid,
    pub device_id: String,
    pub device_name: String,
    pub pin: String,
    pub expires_at_unix: i64,
}

pub async fn health(Extension(state): Extension<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        pc_id: state.service.pc_id().to_string(),
        name: state.service.pc_name(),
        version: 2,
        port: state.service.server_port(),
    })
}

pub async fn create_request(
    Extension(state): Extension<AppState>,
    Json(input): Json<PairingRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    #[cfg(target_os = "windows")]
    let secure_desktop = crate::platform::is_secure_desktop().unwrap_or(false)
        && !qinglink_windows::secure_desktop::running_in_session_zero();
    #[cfg(not(target_os = "windows"))]
    let secure_desktop = crate::platform::is_secure_desktop().unwrap_or(false);
    if secure_desktop {
        return Err(error(
            axum::http::StatusCode::LOCKED,
            "PAIRING_REQUIRES_DESKTOP",
            "首次配对需要在已登录桌面确认",
        ));
    }
    let request = state
        .pairing
        .lock()
        .map_err(|_| {
            error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "PAIRING_UNAVAILABLE",
                "配对服务不可用",
            )
        })?
        .request(input.device_id, input.device_name);
    let expires_at_unix = time::OffsetDateTime::now_utc().unix_timestamp() + 60;
    let pin = request.pin.clone();
    let _ = state.pairing_events.send(PairingNotification {
        request_id: request.request_id,
        device_id: request.device_id,
        device_name: request.device_name,
        pin: request.pin,
        expires_at_unix,
    });
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        let result = state
            .pairing
            .lock()
            .map_err(|_| {
                error(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "PAIRING_UNAVAILABLE",
                    "配对服务不可用",
                )
            })?
            .confirm(request.request_id, &pin)
            .map_err(pairing_error)?;
        let token = state
            .token_store
            .create_paired_token(&result.device_id, &result.device_name)
            .map_err(|_| {
                error(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "TOKEN_CREATE_FAILED",
                    "无法创建令牌",
                )
            })?;
        let mac_address = state.system_service.get_mac_address();
        let wake_networks = state.system_service.get_wake_networks();
        return Ok(Json(serde_json::json!({
            "ok": true,
            "data": {
                "token": token,
                "pc_id": state.service.pc_id().to_string(),
                "pc_name": state.service.pc_name(),
                "mac_address": mac_address,
                "wake_networks": wake_networks,
            }
        })));
    }
    Ok(Json(serde_json::json!({
        "ok": true,
        "data": {
            "request_id": request.request_id,
            "expires_at_unix": expires_at_unix,
        }
    })))
}

pub async fn confirm_request(
    Extension(state): Extension<AppState>,
    Json(input): Json<PairingConfirm>,
) -> Result<Json<PairingTokenResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    #[cfg(target_os = "windows")]
    let secure_desktop = crate::platform::is_secure_desktop().unwrap_or(false)
        && !qinglink_windows::secure_desktop::running_in_session_zero();
    #[cfg(not(target_os = "windows"))]
    let secure_desktop = crate::platform::is_secure_desktop().unwrap_or(false);
    if secure_desktop {
        return Err(error(
            axum::http::StatusCode::LOCKED,
            "PAIRING_REQUIRES_DESKTOP",
            "首次配对需要在已登录桌面确认",
        ));
    }
    let request_id = input.request_id;
    let result = state
        .pairing
        .lock()
        .map_err(|_| {
            error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "PAIRING_UNAVAILABLE",
                "配对服务不可用",
            )
        })?
        .confirm(request_id, &input.pin)
        .map_err(pairing_error)?;
    if result.device_id != input.device_id {
        return Err(error(
            axum::http::StatusCode::BAD_REQUEST,
            "DEVICE_MISMATCH",
            "设备标识不匹配",
        ));
    }
    let token = state
        .token_store
        .create_paired_token(&result.device_id, &result.device_name)
        .map_err(|_| {
            error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "TOKEN_CREATE_FAILED",
                "无法创建令牌",
            )
        })?;
    let mac_address = state.system_service.get_mac_address();
    let wake_networks = state.system_service.get_wake_networks();
    let _ = state.pairing_events.send(PairingNotification {
        request_id,
        device_id: String::new(),
        device_name: String::new(),
        pin: String::new(),
        expires_at_unix: 0,
    });
    Ok(Json(PairingTokenResponse {
        token,
        pc_id: state.service.pc_id().to_string(),
        pc_name: state.service.pc_name(),
        mac_address,
        wake_networks: Some(wake_networks),
    }))
}

pub async fn cancel_request(
    Path(request_id): Path<Uuid>,
    Extension(state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cancelled = state
        .pairing
        .lock()
        .map_err(|_| {
            error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "PAIRING_UNAVAILABLE",
                "配对服务不可用",
            )
        })?
        .cancel(request_id);
    if !cancelled {
        return Err(error(
            axum::http::StatusCode::NOT_FOUND,
            "PAIRING_NOT_FOUND",
            "配对请求不存在或已失效",
        ));
    }
    let _ = state.pairing_events.send(PairingNotification {
        request_id,
        device_id: String::new(),
        device_name: String::new(),
        pin: String::new(),
        expires_at_unix: 0,
    });
    Ok(Json(serde_json::json!({ "ok": true, "data": {} })))
}

pub fn subscribe(state: &AppState) -> broadcast::Receiver<PairingNotification> {
    state.pairing_events.subscribe()
}

fn pairing_error(pairing: PairingError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let message = pairing.to_string();
    let (status, code) = match pairing {
        PairingError::NotFound => (axum::http::StatusCode::NOT_FOUND, "PAIRING_NOT_FOUND"),
        PairingError::Expired => (axum::http::StatusCode::GONE, "PAIRING_EXPIRED"),
        PairingError::InvalidPin => (axum::http::StatusCode::BAD_REQUEST, "INVALID_PIN"),
        PairingError::RateLimited => (axum::http::StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
    };
    error(status, code, &message)
}

fn error(
    status: axum::http::StatusCode,
    code: &str,
    message: &str,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "ok": false, "error": { "code": code, "message": message } })),
    )
}
