//! 配对 HTTP 接口：健康检查，以及配对请求的发起、确认与取消。

use axum::{Extension, Json, extract::Path};
use qinglink_domain::PairingError;
use qinglink_protocol::{HealthResponse, PairingConfirm, PairingRequest, PairingTokenResponse};
use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::AppState;

/// 广播给桌面端的配对状态变化通知。
#[derive(Clone, Serialize)]
pub struct PairingNotification {
    /// 配对请求标识。
    pub request_id: Uuid,
    /// 发起配对的设备标识。
    pub device_id: String,
    /// 发起配对的设备名称。
    pub device_name: String,
    /// 供用户在桌面端输入的 PIN；终态通知中为空串。
    pub pin: String,
    /// 请求过期时间（Unix 秒）；终态通知中为 0。
    pub expires_at_unix: i64,
}

impl std::fmt::Debug for PairingNotification {
    /// 手写 `Debug` 以剔除 PIN：该字段是要展示给用户的秘密，
    /// 不能随 `{:?}` 进入日志（约定见 docs/development.md 第 3 条）。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PairingNotification")
            .field("request_id", &self.request_id)
            .field("device_id", &self.device_id)
            .field("device_name", &self.device_name)
            .field("pin", &"<redacted>")
            .field("expires_at_unix", &self.expires_at_unix)
            .finish()
    }
}

/// 健康检查：返回本机标识、名称、协议版本、产品版本与监听端口。
pub async fn health(Extension(state): Extension<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        pc_id: state.service.pc_id().to_string(),
        name: state.service.pc_name(),
        version: 2,
        port: state.service.server_port(),
        app_version: Some(qinglink_version::VERSION.to_string()),
    })
}

/// 发起配对请求并向桌面端广播 PIN，返回请求标识与过期时间。
///
/// # Errors
/// 桌面端配对不可用时返回 `403 LOCKED / PAIRING_REQUIRES_DESKTOP`；配对状态机
/// 加锁失败时返回 `500 PAIRING_UNAVAILABLE`；状态机拒绝请求时按
/// `pairing_error` 映射为 `PAIRING_NOT_FOUND` / `PAIRING_EXPIRED` /
/// `INVALID_PIN` / `RATE_LIMITED` / `DEVICE_MISMATCH` / `INVALID_REQUEST` /
/// `PAIRING_BUSY`。
pub async fn create_request(
    Extension(state): Extension<AppState>,
    Json(input): Json<PairingRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if !state.control.pairing_available() {
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
        .try_request(input.device_id, input.device_name)
        .map_err(|error| pairing_error(&error))?;
    let expires_at_unix = time::OffsetDateTime::now_utc().unix_timestamp() + 60;
    let _ = state.pairing_events.send(PairingNotification {
        request_id: request.request_id,
        device_id: request.device_id,
        device_name: request.device_name,
        pin: request.pin,
        expires_at_unix,
    });
    Ok(Json(serde_json::json!({
        "ok": true,
        "data": {
            "request_id": request.request_id,
            "expires_at_unix": expires_at_unix,
        }
    })))
}

/// 校验 PIN 完成配对，签发设备令牌并返回连接所需的网络信息。
///
/// # Errors
/// 桌面端配对不可用时返回 `403 LOCKED / PAIRING_REQUIRES_DESKTOP`；配对状态机
/// 加锁失败时返回 `500 PAIRING_UNAVAILABLE`；确认失败时按 `pairing_error` 映射
/// 为对应的 `PAIRING_*` / `INVALID_PIN` 等错误；签发或持久化设备令牌失败时返回
/// `500 TOKEN_CREATE_FAILED`。
pub async fn confirm_request(
    Extension(state): Extension<AppState>,
    Json(input): Json<PairingConfirm>,
) -> Result<Json<PairingTokenResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if !state.control.pairing_available() {
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
        .confirm_for_device(request_id, &input.device_id, &input.pin)
        .map_err(|error| pairing_error(&error))?;
    let token = state
        .token_store
        .create_paired_token(&result.device_id, &result.device_name)
        .map_err(|cause| {
            // 原始错误必须留下：磁盘满、库被锁、唯一约束冲突在这里都会
            // 呈现为同一句"无法创建令牌"，没有日志就无法区分。
            tracing::error!(error = %cause, "配对确认后创建令牌失败");
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

/// 取消尚未确认的配对请求，并通知桌面端撤下 PIN。
///
/// # Errors
/// 配对状态机加锁失败时返回 `500 PAIRING_UNAVAILABLE`；请求不存在或已失效时返回
/// `404 PAIRING_NOT_FOUND`。
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

/// 订阅配对通知流，供桌面端展示 PIN 与终态。
pub fn subscribe(state: &AppState) -> broadcast::Receiver<PairingNotification> {
    state.pairing_events.subscribe()
}

fn pairing_error(pairing: &PairingError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let message = pairing.to_string();
    let (status, code) = match pairing {
        PairingError::NotFound => (axum::http::StatusCode::NOT_FOUND, "PAIRING_NOT_FOUND"),
        PairingError::Expired => (axum::http::StatusCode::GONE, "PAIRING_EXPIRED"),
        PairingError::InvalidPin => (axum::http::StatusCode::BAD_REQUEST, "INVALID_PIN"),
        PairingError::RateLimited => (axum::http::StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
        PairingError::DeviceMismatch => (axum::http::StatusCode::BAD_REQUEST, "DEVICE_MISMATCH"),
        PairingError::InvalidRequest => (axum::http::StatusCode::BAD_REQUEST, "INVALID_REQUEST"),
        PairingError::TooManyPending => (axum::http::StatusCode::TOO_MANY_REQUESTS, "PAIRING_BUSY"),
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
