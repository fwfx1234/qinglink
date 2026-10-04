//! 已配对设备与配对审批。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use qinglink_protocol::service_ipc::{DesktopCommand, DeviceSummary, MAX_PAGE_SIZE};
use serde::Deserialize;
use serde_json::{Value, json};

use super::super::error::{ApiError, ok};
use super::super::state::{ServerEvent, WebState};
use super::{AdminPage, clamp_limit};

/// Query parameters of `GET /devices`.
#[derive(Debug, Default, Deserialize)]
pub struct DeviceQuery {
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

/// `GET /devices`
pub async fn list(
    State(state): State<Arc<WebState>>,
    Query(query): Query<DeviceQuery>,
) -> Result<Json<Value>, ApiError> {
    let offset = query.offset.unwrap_or(0);
    let limit = clamp_limit(query.limit).min(MAX_PAGE_SIZE);
    let page = state
        .service
        .call_typed::<AdminPage<DeviceSummary>>(DesktopCommand::ListDevices { offset, limit })
        .await?;
    let total = page.total.unwrap_or(page.items.len());
    Ok(ok(&json!({
        "items": page.items,
        "offset": page.offset.unwrap_or(offset),
        "limit": limit,
        "total": total,
        "next_offset": page.next_offset,
    })))
}

/// Body of `POST /devices/revoke`.
#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    device_id: String,
}

/// `POST /devices/revoke`
///
/// Revoking closes the device's control and input connections immediately, so
/// the response reports that explicitly rather than leaving the UI to guess.
pub async fn revoke(
    State(state): State<Arc<WebState>>,
    Json(request): Json<RevokeRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.device_id.trim().is_empty() {
        return Err(ApiError::bad_request("缺少 device_id"));
    }
    state
        .service
        .call(DesktopCommand::RevokeDevice {
            device_id: request.device_id.trim().to_string(),
        })
        .await?;
    state.publish(ServerEvent::Toast {
        level: "info",
        message: "设备已撤销，其对这台电脑的访问已断开".into(),
    });
    Ok(ok(&json!({ "revoked": true })))
}

/// `GET /pairing`
pub async fn pairing(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&json!({ "pairing": state.service.pending_pairing() }))
}

/// Body of `POST /pairing/cancel`.
#[derive(Debug, Deserialize)]
pub struct CancelRequest {
    request_id: String,
}

/// `POST /pairing/cancel`
pub async fn cancel_pairing(
    State(state): State<Arc<WebState>>,
    Json(request): Json<CancelRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.request_id.trim().is_empty() {
        return Err(ApiError::bad_request("缺少 request_id"));
    }
    state
        .service
        .call(DesktopCommand::CancelPairing {
            pairing_id: request.request_id.trim().to_string(),
        })
        .await?;
    // The cancellation is authoritative: announce the disappearance now instead
    // of waiting for the next snapshot poll to notice it.
    state.publish(ServerEvent::Pairing(Box::new(None)));
    Ok(ok(&json!({ "cancelled": true })))
}
