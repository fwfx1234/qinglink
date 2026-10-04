//! 请求/响应信封：编解码、协议版本校验与能力协商。

use std::sync::Arc;
use std::sync::atomic::Ordering;

use qinglink_protocol::service_ipc::{
    Capabilities, DesktopCommand, DesktopRequest, SERVICE_IPC_VERSION,
};
use serde::de::DeserializeOwned;

use super::ServiceLinkState;
use super::inner::Inner;
use super::transport::Transport;
use crate::service::{ServiceError, UPGRADE_HINT};

/// Negotiate command support once per connection.
///
/// The cache lives in the shared state (`capabilities`), not in a worker-local
/// flag: `mark_disconnected` clears it, so a service restart drops the stale
/// answer and the next command re-negotiates instead of failing as
/// "unsupported" forever.
pub(super) fn ensure_negotiated(
    transport: &mut impl Transport,
    inner: &Arc<Inner>,
) -> Result<(), ServiceError> {
    if inner
        .state
        .lock()
        .map(|state| state.capabilities.is_some())
        .unwrap_or(false)
    {
        return Ok(());
    }
    let capabilities: Capabilities =
        exchange(transport, DesktopCommand::Capabilities, inner).and_then(decode)?;
    if !capabilities.management {
        return Err(ServiceError::Unsupported(UPGRADE_HINT.into()));
    }
    if let Ok(mut state) = inner.state.lock() {
        state.capabilities = Some(capabilities);
    }
    Ok(())
}

/// One request/response exchange with correlation and version checks.
///
/// The checks run in the order that produces the most actionable answer: a
/// protocol version the desktop cannot use is reported as "upgrade the service",
/// not as a transport failure, because the pipe itself worked.
pub(super) fn exchange(
    transport: &mut impl Transport,
    command: DesktopCommand,
    inner: &Arc<Inner>,
) -> Result<serde_json::Value, ServiceError> {
    let request = DesktopRequest {
        version: SERVICE_IPC_VERSION,
        request_id: format!(
            "desktop-{}-{}",
            std::process::id(),
            inner.next_request.fetch_add(1, Ordering::Relaxed)
        ),
        command,
    };
    let response = transport.request(&request, &inner.stop)?;
    note_service_version(inner, Some(response.version));
    if response.version != SERVICE_IPC_VERSION {
        return Err(ServiceError::Unsupported(format!(
            "服务协议版本不匹配（服务 {} / 桌面端 {}），请在设置中点击「修复服务（升级）」",
            response.version, SERVICE_IPC_VERSION
        )));
    }
    if response.request_id != request.request_id {
        // An empty id means a service that predates the correlated envelope, so
        // it is a version problem rather than a corrupt reply.
        if response.request_id.is_empty() {
            return Err(ServiceError::Unsupported(UPGRADE_HINT.into()));
        }
        return Err(ServiceError::Internal("服务响应 request_id 不匹配".into()));
    }
    if !response.ok {
        return Err(classify_refusal(response.code, response.message));
    }
    Ok(response.payload)
}

/// Classify a refusal code the service sent in a well-formed envelope.
fn classify_refusal(code: String, message: String) -> ServiceError {
    match code.as_str() {
        "UNKNOWN_COMMAND" => ServiceError::Unsupported(if message.trim().is_empty() {
            UPGRADE_HINT.to_string()
        } else {
            message
        }),
        "UNAUTHORIZED_DESKTOP" => ServiceError::Unauthorized(if message.trim().is_empty() {
            "桌面端身份未获授权，请在设置中修复服务安装".to_string()
        } else {
            message
        }),
        _ => ServiceError::Rejected { code, message },
    }
}

/// Remember the protocol version a peer reported, for the admin diagnostics.
fn note_service_version(inner: &Arc<Inner>, version: Option<u32>) {
    if let Ok(mut state) = inner.state.lock()
        && state.service_version != version
    {
        state.service_version = version;
    }
}

/// 把信封里的 payload 解码为调用方期望的类型。
pub(super) fn decode<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, ServiceError> {
    serde_json::from_value(value)
        .map_err(|_| ServiceError::Internal("服务响应格式错误，请升级或修复服务".into()))
}

/// Message shown to a caller while the link is not usable.
pub(super) fn disconnected_message(state: ServiceLinkState) -> String {
    match state {
        ServiceLinkState::ProtocolMismatch => {
            "服务协议版本过旧，请在设置中点击「修复服务（升级）」".into()
        }
        ServiceLinkState::Unauthorized => {
            "服务未授权当前桌面端，请在设置中点击「修复服务」重新登记".into()
        }
        ServiceLinkState::Connecting => "正在连接服务，请稍后重试".into(),
        ServiceLinkState::Connected | ServiceLinkState::Unreachable => {
            "服务未连接，请先安装并启动 QingLinkService".into()
        }
    }
}
