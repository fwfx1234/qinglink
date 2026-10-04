//! 单个条目的编辑：重命名、隐藏、常用、置顶与删除。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use qinglink_protocol::service_ipc::DesktopCommand;
use serde::Deserialize;
use serde_json::Value;

use crate::web::error::{ApiError, ok};
use crate::web::state::{ServerEvent, WebState};

use super::support::{MAX_DISPLAY_NAME_CHARS, required_id};

/// Body of `POST /apps/rename`.
#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    id: String,
    #[serde(default)]
    name: String,
}

/// `POST /apps/rename`
///
/// An empty `name` restores the scanned name, which is how the UI implements
/// "reset to original".
pub async fn rename(
    State(state): State<Arc<WebState>>,
    Json(request): Json<RenameRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let name = request.name.trim();
    if name.chars().count() > MAX_DISPLAY_NAME_CHARS {
        return Err(ApiError::bad_request(format!(
            "名称不能超过 {MAX_DISPLAY_NAME_CHARS} 个字符"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(ApiError::bad_request("名称不能包含控制字符"));
    }
    let result = state
        .service
        .call(DesktopCommand::RenameApp {
            id,
            name: name.to_string(),
        })
        .await?;
    state.publish(ServerEvent::CatalogChanged("rename"));
    Ok(ok(&result))
}

/// Body of `POST /apps/hidden`.
#[derive(Debug, Deserialize)]
pub struct HiddenRequest {
    id: String,
    hidden: bool,
}

/// `POST /apps/hidden`
pub async fn set_hidden(
    State(state): State<Arc<WebState>>,
    Json(request): Json<HiddenRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let result = state
        .service
        .call(DesktopCommand::SetAppHidden {
            id,
            hidden: request.hidden,
        })
        .await?;
    state.publish(ServerEvent::CatalogChanged("hidden"));
    Ok(ok(&result))
}

/// Body of `POST /apps/favorite`.
#[derive(Debug, Deserialize)]
pub struct FavoriteRequest {
    id: String,
    favorite: bool,
}

/// `POST /apps/favorite`
///
/// 常用 is stored on the service, so this is the same list the phone shows and
/// the management UI can curate it on the user's behalf.
pub async fn set_favorite(
    State(state): State<Arc<WebState>>,
    Json(request): Json<FavoriteRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let result = state
        .service
        .call(DesktopCommand::SetAppFavorite {
            id,
            favorite: request.favorite,
        })
        .await?;
    state.publish(ServerEvent::CatalogChanged("favorite"));
    Ok(ok(&result))
}

/// Body of `POST /apps/pinned`.
#[derive(Debug, Deserialize)]
pub struct PinnedRequest {
    id: String,
    pinned: bool,
}

/// `POST /apps/pinned`
pub async fn set_pinned(
    State(state): State<Arc<WebState>>,
    Json(request): Json<PinnedRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let result = state
        .service
        .call(DesktopCommand::SetAppPinned {
            id,
            pinned: request.pinned,
        })
        .await?;
    state.publish(ServerEvent::CatalogChanged("favorite"));
    Ok(ok(&result))
}

/// Body of `POST /apps/delete`.
#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    id: String,
}

/// `POST /apps/delete`
pub async fn delete_app(
    State(state): State<Arc<WebState>>,
    Json(request): Json<DeleteRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let result = state.service.call(DesktopCommand::DeleteApp { id }).await?;
    state.publish(ServerEvent::CatalogChanged("delete"));
    Ok(ok(&result))
}
