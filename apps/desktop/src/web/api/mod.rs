//! 管理 REST 接口面，挂载在 `/api/admin/v1`。
//!
//! 路由布局遵循 `docs/web-api.md` 的契约。按对象寻址的写操作使用带 JSON 体的
//! `POST`，而不是路径参数，因为目录 id 就是 Windows 路径，放进 URL 段会需要脆弱的
//! 百分号编码。

pub mod catalog;
pub mod devices;
pub mod diagnostics;
pub mod dialog;
pub mod logs;
pub mod service;
pub mod system;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use super::state::WebState;

/// Build the admin API router.
pub fn router() -> Router<Arc<WebState>> {
    Router::new()
        .route("/health", get(system::health))
        .route("/overview", get(system::overview))
        .route(
            "/settings",
            get(system::get_settings).post(system::update_settings),
        )
        .route("/desktop/open", post(system::open_ui))
        .route("/desktop/restart", post(system::restart))
        .route("/desktop/exit", post(system::exit))
        .route("/service", get(service::status))
        .route("/service/pause", post(service::pause))
        .route("/service/reconnect", post(service::reconnect))
        .route("/service/install", post(service::install))
        .route("/service/uninstall", post(service::uninstall))
        .route("/service/start", post(service::start))
        .route("/service/stop", post(service::stop))
        .route("/service/restart", post(service::restart))
        .route("/service/install-state", get(service::install_state))
        .route("/diagnostics", get(diagnostics::report))
        .route("/diagnostics/bundle", get(diagnostics::bundle))
        .route("/dialog/file", post(dialog::pick))
        .route("/devices", get(devices::list))
        .route("/devices/revoke", post(devices::revoke))
        .route("/pairing", get(devices::pairing))
        .route("/pairing/cancel", post(devices::cancel_pairing))
        .route("/apps", get(catalog::list))
        .route("/apps/stats", get(catalog::stats))
        .route("/apps/icon", get(catalog::icon))
        .route("/apps/icon/candidates", get(catalog::icon_candidates))
        .route("/apps/icon/refresh", post(catalog::refresh_icon))
        .route("/apps/icon/set", post(catalog::set_icon))
        .route("/apps/rename", post(catalog::rename))
        .route("/apps/hidden", post(catalog::set_hidden))
        .route("/apps/favorite", post(catalog::set_favorite))
        .route("/apps/pinned", post(catalog::set_pinned))
        .route("/apps/delete", post(catalog::delete_app))
        .route("/apps/scan", post(catalog::scan))
        .route("/apps/add", post(catalog::add))
        .route("/apps/launch", post(catalog::launch))
        .route("/apps/batch", post(catalog::batch))
        .route("/logs", get(logs::query).delete(logs::clear))
        .route("/logs/stats", get(logs::stats))
        .route("/logs/files", get(logs::files))
        .route("/logs/service", get(logs::service))
        .route("/logs/download", get(logs::download))
}

/// Maximum page size any endpoint will honour.
pub const MAX_LIMIT: usize = 100;

/// A page as returned by the service for admin listings.
///
/// `total` and `kind` are additive metadata that a service predating this
/// desktop build will not send, so both are optional.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct AdminPage<T> {
    pub items: Vec<T>,
    #[serde(default)]
    pub total: Option<usize>,
    #[serde(default)]
    pub next_offset: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub kind: Option<String>,
}

/// Clamp a caller-supplied page size.
pub fn clamp_limit(limit: Option<usize>) -> usize {
    limit.unwrap_or(50).clamp(1, MAX_LIMIT)
}

/// Clamp a caller-supplied page size for endpoints with a larger contract.
pub fn clamp_limit_with(limit: Option<usize>, default: usize, max: usize) -> usize {
    limit.unwrap_or(default).clamp(1, max)
}
