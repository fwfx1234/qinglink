//! 在本机启动一个目录条目。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use qinglink_protocol::service_ipc::DesktopCommand;
use serde::Deserialize;
use serde_json::Value;

use crate::web::error::{ApiError, ok};
use crate::web::state::WebState;

use super::support::required_id;

/// Body of `POST /apps/launch`.
#[derive(Debug, Deserialize)]
pub struct LaunchRequest {
    id: String,
}

/// `POST /apps/launch`
///
/// Starts the entry on this PC. The service forwards the launch to the
/// interactive session, so it is visible to the user even though the service
/// itself runs in Session 0.
pub async fn launch(
    State(state): State<Arc<WebState>>,
    Json(request): Json<LaunchRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = required_id(&request.id)?;
    let result = state.service.call(DesktopCommand::LaunchApp { id }).await?;
    Ok(ok(&result))
}
