//! 提供管理界面及其 API 的本地 Web 服务。
//!
//! 服务器刻意绑定在 `127.0.0.1`。它不设认证就暴露完整的管理面，是因为它代理的
//! Windows 命名管道本就只对已授权的桌面端对端开放；绑定到可路由的接口等于把这份
//! 权限交给网络。

pub mod api;
pub mod assets;
pub mod error;
pub mod state;
pub mod ws;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::json;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;

use crate::logging::LogEntry;
use crate::service::ServiceEvent;
use state::{ServerEvent, WebState};

/// How often accumulated log records are flushed to subscribers.
const LOG_BATCH_INTERVAL: Duration = Duration::from_millis(200);

/// Maximum records in one log frame. Older records beyond this are dropped and
/// counted, so one chatty second cannot stall a browser.
const LOG_BATCH_MAX: usize = 200;

/// Path prefix of the admin API.
const API_PREFIX: &str = "/api/admin/v1";

/// Start the web server on a dedicated thread.
///
/// The router is built here, on the calling thread, before the worker starts.
/// A route-configuration panic is then a hard startup failure with a clear
/// stack trace, rather than a dead worker thread that leaves the interface
/// unreachable while the process still looks healthy.
///
/// Returns only once the listener is bound, so a port conflict is reported to
/// the caller instead of disappearing into a background thread.
pub fn run_on_thread(port: u16, state: Arc<WebState>) -> Result<std::thread::JoinHandle<()>> {
    let app = router(Arc::clone(&state));
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
    let handle = std::thread::Builder::new()
        .name("qinglink-web".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = ready_tx.send(Err(anyhow::anyhow!(
                        "cannot create the web runtime: {error}"
                    )));
                    return;
                }
            };
            runtime.block_on(async move {
                let listener = match bind(port).await {
                    Ok(listener) => {
                        let _ = ready_tx.send(Ok(()));
                        listener
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                // The pumps share the lifetime of the server, so they are started
                // here rather than in the router builder.
                tokio::spawn(pump_service_events(Arc::clone(&state)));
                tokio::spawn(pump_logs(Arc::clone(&state)));

                tracing::info!("Web 管理界面已监听 http://127.0.0.1:{port}");
                if let Err(error) = axum::serve(listener, app).await {
                    tracing::error!(%error, "Web 管理服务异常退出");
                }
            });
        })
        .context("cannot start the web server thread")?;

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(anyhow::anyhow!(
            "the web server thread exited during startup"
        )),
    }
}

async fn bind(port: u16) -> Result<tokio::net::TcpListener> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("无法绑定 {address}，端口可能已被占用"))
}

/// Build the complete router.
fn router(state: Arc<WebState>) -> Router {
    let api = api::router().route("/events", get(ws::handler));
    Router::new()
        .nest(API_PREFIX, api)
        .route("/", get(serve_index))
        // axum 0.7 catch-all syntax: the parameter must be the final segment and
        // is written as `/*name`. The `{*name}` form is axum 0.8 and panics here.
        .route("/assets/*path", get(serve_asset))
        .fallback(serve_fallback)
        .with_state(state)
}

async fn serve_index(State(_state): State<Arc<WebState>>) -> Response {
    index_response()
}

/// Serve a hashed build asset from the `/assets` directory.
///
/// Vite emits content-hashed file names here, and the subprocess-free fallback
/// build hashes the bundled stylesheet too, so these responses are immutable.
/// Any other embedded file is handled by the fallback, which applies a shorter
/// cache.
async fn serve_asset(Path(path): Path<String>) -> Response {
    let url = format!("/assets/{path}");
    match assets::lookup(&url) {
        Some((content_type, bytes)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            bytes,
        )
            .into_response(),
        None => not_found_response("资源不存在"),
    }
}

/// Serve any other path.
///
/// Three cases, in order: an API path is a hard 404 so a mistyped call never
/// receives HTML; an embedded frontend file is served as-is; anything else is a
/// deep link into the single-page application, which is why the shell is
/// returned rather than a 404.
async fn serve_fallback(uri: Uri) -> Response {
    let path = uri.path();
    if is_api_path(path) {
        return not_found_response("未知接口");
    }
    if let Some((content_type, bytes)) = assets::lookup(path) {
        // Only `index.html` is served this way in a default build; other files
        // here have stable names and so must not be cached aggressively.
        if path == "/index.html" {
            return index_response();
        }
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            bytes,
        )
            .into_response();
    }
    index_response()
}

/// Whether a path belongs to the admin API and must therefore never be answered
/// with the SPA shell.
///
/// Returning HTML for a mistyped API call would make `response.json()` throw in
/// the client and hide the real problem.
fn is_api_path(path: &str) -> bool {
    path.starts_with("/api/")
}

fn index_response() -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            // The shell references hashed assets, so it must not be cached or a
            // deployment would keep serving the previous build.
            (header::CACHE_CONTROL, "no-cache"),
        ],
        assets::INDEX_HTML,
    )
        .into_response()
}

fn not_found_response(message: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        axum::Json(json!({
            "ok": false,
            "error": { "code": "NOT_FOUND", "message": message },
        })),
    )
        .into_response()
}

/// Forward service events to the browser fan-out.
async fn pump_service_events(state: Arc<WebState>) {
    let mut receiver = state.service.subscribe();
    let mut was_connected = state.service.status().connected;
    loop {
        match receiver.recv().await {
            Ok(ServiceEvent::Status) => {
                let connected = state.service.status().connected;
                state.publish(ServerEvent::Service(Box::new(state.service.status())));
                // A connect or disconnect transition also changes what the
                // SCM-side health check would report, and nothing else re-probes
                // it. Doing it only on the transition keeps it off the poll path.
                if connected != was_connected {
                    was_connected = connected;
                    crate::input_install::refresh(&state.install_state);
                    state.publish_install_state();
                }
            }
            Ok(ServiceEvent::Pairing(pairing)) => {
                state.publish(ServerEvent::Pairing(Box::new(pairing)));
            }
            Ok(ServiceEvent::Devices { total }) => {
                state.publish(ServerEvent::Devices { total });
            }
            // The service finished a background scan; browsers only know that
            // lists and counters are now stale.
            Ok(ServiceEvent::CatalogScanFinished) => {
                state.publish(ServerEvent::CatalogChanged("scan_finished"));
            }
            // A lagging subscriber missed at least one change; the current state
            // is what matters, so publish it instead of the missed deltas.
            Err(RecvError::Lagged(_)) => {
                state.publish(ServerEvent::Service(Box::new(state.service.status())));
                state.publish(ServerEvent::Pairing(Box::new(
                    state.service.pending_pairing(),
                )));
                if let Some(total) = state.service.device_count() {
                    state.publish(ServerEvent::Devices { total });
                }
            }
            Err(RecvError::Closed) => return,
        }
    }
}

/// Coalesce captured log records into periodic frames.
///
/// One batcher serves every socket. Sending per record would let a chatty
/// subsystem dominate the connection and starve the state events that share it.
async fn pump_logs(state: Arc<WebState>) {
    let mut receiver = state.logs.subscribe();
    let mut batch: Vec<Arc<LogEntry>> = Vec::new();
    let mut dropped: u64 = 0;
    let mut ticker = tokio::time::interval(LOG_BATCH_INTERVAL);

    loop {
        tokio::select! {
            event = receiver.recv() => match event {
                Ok(entry) => batch.push(entry),
                // The collector's channel overflowed; it already reported the
                // exact gap size, so accumulate it for the next flush.
                Err(RecvError::Lagged(count)) => dropped = dropped.saturating_add(count),
                Err(RecvError::Closed) => return,
            },
            _ = ticker.tick() => {
                if batch.len() > LOG_BATCH_MAX {
                    let overflow = batch.len() - LOG_BATCH_MAX;
                    batch.drain(0..overflow);
                    dropped = dropped.saturating_add(overflow as u64);
                }
                if !batch.is_empty() {
                    // Materialize owned entries for serialization; the shared
                    // handles stay with the collector.
                    let items: Vec<LogEntry> =
                        batch.iter().map(|entry| entry.as_ref().clone()).collect();
                    batch.clear();
                    state.publish(ServerEvent::Logs(Arc::new(items)));
                }
                if dropped > 0 {
                    state.publish(ServerEvent::LogsDropped(dropped));
                    dropped = 0;
                }
            }
        }
    }
}

/// Channel pair used by the main thread to receive host commands.
pub fn host_channel() -> (
    mpsc::UnboundedSender<state::HostCommand>,
    mpsc::UnboundedReceiver<state::HostCommand>,
) {
    mpsc::unbounded_channel()
}
