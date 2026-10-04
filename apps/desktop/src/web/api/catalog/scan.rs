//! 扫描触发与手动新增条目。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use qinglink_protocol::service_ipc::DesktopCommand;
use serde::Deserialize;
use serde_json::Value;

use crate::web::error::{ApiError, ok};
use crate::web::state::{ServerEvent, WebState};

use super::support::{MAX_APP_PATH_CHARS, MAX_DISPLAY_NAME_CHARS, normalize_kind};

/// Body of `POST /apps/scan`.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ScanRequest {
    kind: Option<String>,
}

/// `POST /apps/scan`
///
/// Scanning runs in the background on the service. The response acknowledges
/// acceptance; completion arrives as an `apps/catalog_changed` event.
pub async fn scan(
    State(state): State<Arc<WebState>>,
    Json(request): Json<ScanRequest>,
) -> Result<Json<Value>, ApiError> {
    let kind = normalize_kind(request.kind.as_deref());
    let result = state
        .service
        .call(DesktopCommand::RescanApps {
            kind: kind.to_string(),
        })
        .await?;
    state.publish(ServerEvent::ScanStarted(kind));
    Ok(ok(&result))
}

/// Body of `POST /apps/add`.
#[derive(Debug, Deserialize)]
pub struct AddRequest {
    path: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

/// `POST /apps/add`
///
/// Adds a user-supplied executable to the catalog. The service validates that
/// the path exists and derives a stable id, so a repeated add is a conflict
/// rather than a duplicate row.
pub async fn add(
    State(state): State<Arc<WebState>>,
    Json(request): Json<AddRequest>,
) -> Result<Json<Value>, ApiError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(ApiError::bad_request("缺少可执行文件路径"));
    }
    if path.chars().count() > MAX_APP_PATH_CHARS || path.chars().any(char::is_control) {
        return Err(ApiError::bad_request("可执行文件路径不合法"));
    }
    let name = request.name.trim();
    if name.chars().count() > MAX_DISPLAY_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(ApiError::bad_request("名称不合法"));
    }
    if request.args.len() > 64 {
        return Err(ApiError::bad_request("启动参数过多"));
    }
    let kind = normalize_kind(request.kind.as_deref());
    let result = state
        .service
        .call(DesktopCommand::AddApp {
            path: path.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            args: request.args,
        })
        .await?;
    state.publish(ServerEvent::CatalogChanged("add"));
    Ok(ok(&result))
}
