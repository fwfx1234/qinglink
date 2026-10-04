//! 目录的列表与统计查询（只读接口）。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use qinglink_protocol::service_ipc::{CatalogApp, CatalogStats, DesktopCommand, MAX_PAGE_SIZE};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::web::api::{AdminPage, clamp_limit};
use crate::web::error::{ApiError, ok};
use crate::web::state::WebState;

use super::support::normalize_kind;

/// Query parameters of `GET /apps`.
#[derive(Debug, Default, Deserialize)]
pub struct AppsQuery {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    include_hidden: Option<bool>,
    /// Only entries in 常用（"我的库"）.
    #[serde(default)]
    favorite: Option<bool>,
}

/// `GET /apps`
pub async fn list(
    State(state): State<Arc<WebState>>,
    Query(query): Query<AppsQuery>,
) -> Result<Json<Value>, ApiError> {
    let offset = query.offset.unwrap_or(0);
    let limit = clamp_limit(query.limit).min(MAX_PAGE_SIZE);
    let kind = normalize_kind(query.kind.as_deref());
    let page = state
        .service
        .call_typed::<AdminPage<CatalogApp>>(DesktopCommand::ListApps {
            kind: kind.to_string(),
            offset,
            limit,
            query: query.query.clone(),
            include_hidden: query.include_hidden.unwrap_or(true),
            favorite_only: query.favorite.unwrap_or(false),
        })
        .await?;
    let total = page.total.unwrap_or(page.items.len());
    Ok(ok(&json!({
        "items": page.items,
        "offset": page.offset.unwrap_or(offset),
        "limit": limit,
        "total": total,
        "next_offset": page.next_offset,
        "kind": page.kind.unwrap_or_else(|| kind.to_string()),
    })))
}

/// `GET /apps/stats`
pub async fn stats(State(state): State<Arc<WebState>>) -> Result<Json<Value>, ApiError> {
    let stats = state
        .service
        .call_typed::<CatalogStats>(DesktopCommand::GetCatalogStats)
        .await?;
    Ok(ok(&json!(stats)))
}
