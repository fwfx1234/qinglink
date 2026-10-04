//! 图标读取、候选列表与刷新/设置。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use qinglink_protocol::service_ipc::{AppIcon, AppIconCandidates, AppIconUpdate, DesktopCommand};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::web::error::{ApiError, ok};
use crate::web::state::{ServerEvent, WebState};

use super::support::{MAX_APP_PATH_CHARS, required_id};

/// Query parameters of `GET /apps/icon`.
#[derive(Debug, Deserialize)]
pub struct IconQuery {
    id: String,
    /// Cache-buster carrying the entry's `icon_updated_at`. Rendering uses it so
    /// a refreshed icon is not served from the browser cache.
    #[serde(default)]
    #[allow(dead_code)]
    v: Option<String>,
}

/// `GET /apps/icon`
///
/// Returns the raw image so the browser can use it directly in an `<img>` tag.
/// The payload is resolved on demand through every icon source, so an entry
/// without a stored icon still renders one.
pub async fn icon(
    State(state): State<Arc<WebState>>,
    Query(query): Query<IconQuery>,
) -> Result<Response, ApiError> {
    if query.id.trim().is_empty() {
        return Err(ApiError::bad_request("缺少 id"));
    }
    let icon = state
        .service
        .call_typed::<AppIcon>(DesktopCommand::GetAppIcon {
            id: query.id.trim().to_string(),
        })
        .await?;
    let Some(encoded) = icon.icon_base64 else {
        return Err(ApiError::not_found("该条目没有可用图标"));
    };
    let bytes = BASE64
        .decode(encoded.as_bytes())
        .map_err(|error| ApiError::internal(format!("图标数据损坏：{error}")))?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, image_content_type(&bytes)),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        bytes,
    )
        .into_response())
}

/// Query parameters of `GET /apps/icon/candidates`.
#[derive(Debug, Deserialize)]
pub struct IconCandidatesQuery {
    id: String,
}

/// `GET /apps/icon/candidates`
///
/// Lists the icon files the user can pick for one entry: the executable itself,
/// a shortcut's icon location, and any artwork shipped next to the program.
pub async fn icon_candidates(
    State(state): State<Arc<WebState>>,
    Query(query): Query<IconCandidatesQuery>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&query.id)?;
    let result = state
        .service
        .call_typed::<AppIconCandidates>(DesktopCommand::ListAppIcons { id })
        .await?;
    Ok(ok(&json!(result)))
}

/// Body of `POST /apps/icon/refresh`.
#[derive(Debug, Deserialize)]
pub struct IconRefreshRequest {
    id: String,
    /// Explicit icon file; omitted re-runs automatic discovery.
    #[serde(default)]
    icon_path: Option<String>,
}

/// `POST /apps/icon/refresh`
///
/// Re-resolves one entry's icon and persists it, so the fix reaches every
/// client of this PC instead of only the browser that performed it.
pub async fn refresh_icon(
    State(state): State<Arc<WebState>>,
    Json(request): Json<IconRefreshRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let icon_path = normalize_icon_path(request.icon_path.as_deref())?;
    let result = state
        .service
        .call_typed::<AppIconUpdate>(DesktopCommand::RefreshAppIcon { id, icon_path })
        .await?;
    state.publish(ServerEvent::CatalogChanged("icon"));
    Ok(ok(&json!(result)))
}

/// Body of `POST /apps/icon/set`.
#[derive(Debug, Deserialize)]
pub struct IconSetRequest {
    id: String,
    /// Icon file to use. An empty or missing value clears the override and
    /// re-runs automatic discovery.
    #[serde(default)]
    icon_path: Option<String>,
}

/// `POST /apps/icon/set`
pub async fn set_icon(
    State(state): State<Arc<WebState>>,
    Json(request): Json<IconSetRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let icon_path = normalize_icon_path(request.icon_path.as_deref())?;
    let result = state
        .service
        .call_typed::<AppIconUpdate>(DesktopCommand::RefreshAppIcon { id, icon_path })
        .await?;
    state.publish(ServerEvent::CatalogChanged("icon"));
    Ok(ok(&json!(result)))
}

/// Reject an implausible icon path before a round trip.
fn normalize_icon_path(icon_path: Option<&str>) -> Result<Option<String>, ApiError> {
    let Some(icon_path) = icon_path.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if icon_path.chars().count() > MAX_APP_PATH_CHARS || icon_path.chars().any(char::is_control) {
        return Err(ApiError::bad_request("图标路径不合法"));
    }
    Ok(Some(icon_path.to_string()))
}

/// Sniff the image container from its magic bytes.
fn image_content_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(b"BM") {
        "image/bmp"
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}
