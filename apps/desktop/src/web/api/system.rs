//! 进程级端点：健康检查、仪表盘聚合、设置，以及三个桌面端生命周期动作。

use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use qinglink_protocol::service_ipc::{CatalogStats, DesktopCommand, DeviceSummary, MAX_PAGE_SIZE};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::oneshot;

use super::super::error::{ApiError, ok};
use super::super::state::{HostCommand, WebState};
use super::AdminPage;

/// How long an endpoint waits for the main thread to acknowledge an action.
const HOST_TIMEOUT: Duration = Duration::from_secs(10);

/// `GET /health`
pub async fn health(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&health_payload(&state))
}

/// `GET /overview`
///
/// The dashboard needs service status, settings, install state and counters at
/// once. Doing it in one request keeps the first paint to a single round trip;
/// afterwards everything arrives over the event socket.
pub async fn overview(State(state): State<Arc<WebState>>) -> Json<Value> {
    let service = state.service.status();
    let mut apps = 0_usize;
    let mut hidden_apps = 0_usize;
    let mut devices = 0_usize;

    if service.connected {
        if let Ok(stats) = state
            .service
            .call_typed::<CatalogStats>(DesktopCommand::GetCatalogStats)
            .await
        {
            apps = stats.app.total + stats.game.total;
            hidden_apps = stats.app.hidden + stats.game.hidden;
        }
        if let Ok(page) = state
            .service
            .call_typed::<AdminPage<DeviceSummary>>(DesktopCommand::ListDevices {
                offset: 0,
                limit: MAX_PAGE_SIZE,
            })
            .await
        {
            // Prefer the service's true total; fall back to the page length for
            // a service that predates the total field.
            devices = page.total.unwrap_or(page.items.len());
        }
    }

    let log_stats = state.logs.stats();
    let errors = log_stats.by_level.get("ERROR").copied().unwrap_or(0);

    ok(&json!({
        "health": health_payload(&state),
        "service": service,
        "settings": state.settings.view(),
        "install_state": state.install_view(),
        "tray": state.tray_report(),
        "stats": {
            "apps": apps,
            "hidden_apps": hidden_apps,
            "devices": devices,
            "logs": log_stats.total,
            "errors": errors,
        },
        "pairing": state.service.pending_pairing(),
    }))
}

/// `GET /settings`
pub async fn get_settings(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&json!(state.settings.view()))
}

/// Fields accepted by `POST /settings`. Every field is optional so a client can
/// change one setting without restating the others.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateSettings {
    autostart: Option<bool>,
    keep_awake: Option<bool>,
    debug_log: Option<bool>,
}

/// `POST /settings`
pub async fn update_settings(
    State(state): State<Arc<WebState>>,
    Json(request): Json<UpdateSettings>,
) -> Result<Json<Value>, ApiError> {
    if let Some(enabled) = request.autostart {
        // The registry write has no thread affinity, so it is done in place.
        let result = if enabled {
            qinglink_windows::autostart::enable()
        } else {
            qinglink_windows::autostart::disable()
        };
        result.map_err(|error| ApiError::internal(format!("设置开机启动失败：{error}")))?;
        state.settings.set(
            enabled,
            state.settings.keep_awake(),
            state.settings.debug_log(),
        );
    }

    if let Some(enabled) = request.keep_awake {
        // `SetThreadExecutionState` is per-thread, so this must run on the main
        // thread, which outlives any single request.
        let (reply, receiver) = oneshot::channel();
        state
            .host
            .send(HostCommand::SetKeepAwake { enabled, reply })
            .map_err(|_| ApiError::internal("桌面端主线程不可用"))?;
        await_host(receiver).await?;
        state.settings.set(
            state.settings.autostart(),
            enabled,
            state.settings.debug_log(),
        );
    }

    if let Some(enabled) = request.debug_log {
        state
            .preferences
            .set_debug_logging(enabled)
            .map_err(|error| ApiError::internal(format!("保存日志设置失败：{error}")))?;
        state.logs.set_debug(enabled);
        state.settings.set(
            state.settings.autostart(),
            state.settings.keep_awake(),
            enabled,
        );
        state.publish(super::super::state::ServerEvent::Toast {
            level: "info",
            message: if enabled {
                "调试日志已开启".into()
            } else {
                "调试日志已关闭".into()
            },
        });
    }

    // Other tabs (and the tray) share this state, so the change is announced
    // rather than left for the next reload.
    state.publish(super::super::state::ServerEvent::Settings(Box::new(
        state.settings.view(),
    )));
    Ok(ok(&json!(state.settings.view())))
}

/// `POST /desktop/open`
pub async fn open_ui(State(state): State<Arc<WebState>>) -> Result<Json<Value>, ApiError> {
    let (reply, receiver) = oneshot::channel();
    state
        .host
        .send(HostCommand::OpenUi { reply })
        .map_err(|_| ApiError::internal("桌面端主线程不可用"))?;
    let result = await_host(receiver).await?;
    Ok(ok(&result))
}

/// `POST /desktop/restart`
pub async fn restart(State(state): State<Arc<WebState>>) -> Result<Json<Value>, ApiError> {
    dispatch_lifecycle(state, false).await
}

/// `POST /desktop/exit`
pub async fn exit(State(state): State<Arc<WebState>>) -> Result<Json<Value>, ApiError> {
    dispatch_lifecycle(state, true).await
}

async fn dispatch_lifecycle(state: Arc<WebState>, exiting: bool) -> Result<Json<Value>, ApiError> {
    let (reply, receiver) = oneshot::channel();
    let command = if exiting {
        HostCommand::Exit { reply }
    } else {
        HostCommand::Restart { reply }
    };
    state
        .host
        .send(command)
        .map_err(|_| ApiError::internal("桌面端主线程不可用"))?;
    await_host(receiver).await?;
    Ok(ok(&json!({ "accepted": true })))
}

fn health_payload(state: &WebState) -> Value {
    json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "pid": std::process::id(),
        "uptime_secs": state.uptime_secs(),
        "web_port": state.port(),
        "log_level": if state.logs.is_debug() { "debug" } else { "info" },
    })
}

async fn await_host(
    receiver: oneshot::Receiver<Result<Value, ApiError>>,
) -> Result<Value, ApiError> {
    match tokio::time::timeout(HOST_TIMEOUT, receiver).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(ApiError::internal("桌面端未返回结果")),
        Err(_) => Err(ApiError::timeout("桌面端响应超时")),
    }
}
