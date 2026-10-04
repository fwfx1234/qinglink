//! 所有管理端点共用的错误类型。
//!
//! 处理函数返回 `Result<Json<Value>, ApiError>`。失败带有稳定错误码，前端无需解析
//! 消息即可据此分支；同时带有有意义的 HTTP 状态码，便于日志与代理。JSON 信封仍是
//! 权威：客户端必须始终检查 `ok`。

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::service::ServiceError;

/// A failed admin request.
#[derive(Clone, Debug)]
pub struct ApiError {
    code: String,
    message: String,
    status: StatusCode,
}

impl ApiError {
    /// Build an error with an explicit code and status.
    pub fn new(code: impl Into<String>, status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            status,
        }
    }

    /// Stable machine-readable code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// `400 BAD_REQUEST`.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", StatusCode::BAD_REQUEST, message)
    }

    /// `404 NOT_FOUND`.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", StatusCode::NOT_FOUND, message)
    }

    /// `409 CONFLICT`.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("CONFLICT", StatusCode::CONFLICT, message)
    }

    /// `403 UNAUTHORIZED_DESKTOP`: the service refused this desktop identity.
    ///
    /// Repairing the service installation re-registers the desktop executable,
    /// so the interface treats this exactly like an upgrade: a repair action.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new("UNAUTHORIZED_DESKTOP", StatusCode::FORBIDDEN, message)
    }

    /// `503 SERVICE_UNAVAILABLE`: the desktop cannot reach the service.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(
            "SERVICE_UNAVAILABLE",
            StatusCode::SERVICE_UNAVAILABLE,
            message,
        )
    }

    /// `501 NOT_IMPLEMENTED`: the installed service is too old.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new("UNSUPPORTED", StatusCode::NOT_IMPLEMENTED, message)
    }

    /// `504 GATEWAY_TIMEOUT`.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new("TIMEOUT", StatusCode::GATEWAY_TIMEOUT, message)
    }

    /// `500 INTERNAL`.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL", StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// `500 IO_ERROR` for local filesystem failures.
    pub fn io(error: impl std::fmt::Display) -> Self {
        Self::new(
            "IO_ERROR",
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl From<ServiceError> for ApiError {
    fn from(error: ServiceError) -> Self {
        // Routing through the named constructors keeps the code and the status
        // paired in exactly one place.
        let message = error.to_string();
        match &error {
            ServiceError::Unavailable(_) => Self::unavailable(message),
            ServiceError::Unsupported(_) => Self::unsupported(message),
            ServiceError::Unauthorized(_) => Self::forbidden(message),
            ServiceError::Timeout => Self::timeout(message),
            ServiceError::Rejected { code, .. } => match code.as_str() {
                "NOT_FOUND" => Self::not_found(message),
                "CONFLICT" | "GAME_ACTIVE" => Self::conflict(message),
                "BAD_REQUEST" => Self::bad_request(message),
                "UNAUTHORIZED_DESKTOP" => Self::forbidden(message),
                _ => Self::internal(message),
            },
            ServiceError::Internal(_) => Self::internal(message),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::internal(format!("{error:#}"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "ok": false,
            "error": { "code": self.code, "message": self.message },
        });
        (self.status, Json(body)).into_response()
    }
}

/// Wrap a successful payload in the standard envelope.
pub fn ok(data: &serde_json::Value) -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "data": data }))
}
