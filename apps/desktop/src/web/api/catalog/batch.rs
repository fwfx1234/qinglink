//! 批量操作：一次请求对多个条目执行同一动作。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use qinglink_protocol::service_ipc::DesktopCommand;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::web::error::{ApiError, ok};
use crate::web::state::{ServerEvent, WebState};

use super::support::required_id;

/// Upper bound for one batch request, matching the service-side page budget.
const MAX_BATCH_IDS: usize = 100;

/// Upper bound for a batch icon refresh. Each entry costs a shell icon lookup,
/// so a smaller cap keeps the request inside the local transport budget.
const MAX_ICON_REFRESH: usize = 50;

/// Actions `POST /apps/batch` understands.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchAction {
    Hide,
    Show,
    Delete,
    /// Clears the display-name override, restoring the scanned name.
    RestoreName,
    /// Re-resolves each selected entry's icon.
    RefreshIcon,
}

/// Body of `POST /apps/batch`.
#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    action: BatchAction,
    ids: Vec<String>,
}

/// `POST /apps/batch`
///
/// Runs one catalog action over many ids, reporting every outcome. A partial
/// failure never aborts the batch: the caller gets per-id results and can retry
/// only the entries that failed.
pub async fn batch(
    State(state): State<Arc<WebState>>,
    Json(request): Json<BatchRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.ids.len() > MAX_BATCH_IDS {
        return Err(ApiError::bad_request(format!(
            "一次最多操作 {MAX_BATCH_IDS} 个条目"
        )));
    }
    if request.ids.is_empty() {
        return Err(ApiError::bad_request("缺少 id"));
    }
    // Icon resolution shells out per entry; keep one request inside the local
    // transport timeout instead of letting the whole batch look like a hang.
    if request.action == BatchAction::RefreshIcon && request.ids.len() > MAX_ICON_REFRESH {
        return Err(ApiError::bad_request(format!(
            "一次最多重扫 {MAX_ICON_REFRESH} 个图标"
        )));
    }

    let mut results = Vec::with_capacity(request.ids.len());
    let mut succeeded = 0_usize;
    for raw in &request.ids {
        let Ok(id) = required_id(raw) else {
            results.push(
                json!({ "id": raw, "ok": false, "code": "BAD_REQUEST", "message": "缺少 id" }),
            );
            continue;
        };
        let command = match request.action {
            BatchAction::Hide => DesktopCommand::SetAppHidden {
                id: id.clone(),
                hidden: true,
            },
            BatchAction::Show => DesktopCommand::SetAppHidden {
                id: id.clone(),
                hidden: false,
            },
            BatchAction::Delete => DesktopCommand::DeleteApp { id: id.clone() },
            BatchAction::RestoreName => DesktopCommand::RenameApp {
                id: id.clone(),
                name: String::new(),
            },
            BatchAction::RefreshIcon => DesktopCommand::RefreshAppIcon {
                id: id.clone(),
                icon_path: None,
            },
        };
        match state.service.call(command).await {
            Ok(_) => {
                succeeded += 1;
                results.push(json!({ "id": id, "ok": true }));
            }
            Err(error) => {
                let api_error = ApiError::from(error);
                results.push(json!({
                    "id": id,
                    "ok": false,
                    "code": api_error.code(),
                    "message": api_error.message(),
                }));
            }
        }
    }

    // The service already announced its own changes; this keeps a browser whose
    // event socket is behind in step even when every item failed.
    state.publish(ServerEvent::CatalogChanged(match request.action {
        BatchAction::Hide | BatchAction::Show => "hidden",
        BatchAction::Delete => "delete",
        BatchAction::RestoreName => "rename",
        BatchAction::RefreshIcon => "icon",
    }));

    let failed = results.len() - succeeded;
    Ok(ok(&json!({
        "results": results,
        "succeeded": succeeded,
        "failed": failed,
    })))
}
