pub mod auth;
pub mod connection_registry;
pub mod input_protocol;
pub mod input_ws_handler;
pub mod pairing;
pub mod ws_common;
pub mod ws_handler;
pub mod ws_request_handler;

use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{Extension, Router};
use qinglink_domain::PairingManager;
use qinglink_storage::database::DatabaseService;
use tokio::sync::broadcast;

use crate::protocol::requests::WsEvent;
use crate::service::RemoteControlService;
use crate::service::app_scanner::AppScanner;
use crate::service::process::ProcessManager;
use crate::service::system::SystemService;
use crate::service::window_monitor::WindowMonitor;

pub type EventSender = broadcast::Sender<WsEvent>;
type WindowListCache = Option<(Instant, Vec<crate::protocol::requests::WindowInfo>)>;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<RemoteControlService>,
    pub system_service: Arc<SystemService>,
    pub process_manager: Arc<std::sync::Mutex<ProcessManager>>,
    pub token_store: Arc<auth::TokenStore>,
    pub events: EventSender,
    pub app_scanner: Arc<AppScanner>,
    pub window_monitor: Arc<std::sync::Mutex<WindowMonitor>>,
    pub connection_registry: Arc<connection_registry::ConnectionRegistry>,
    pub input_connection_registry: Arc<connection_registry::ConnectionRegistry>,
    pub input_move_tx: tokio::sync::watch::Sender<(i32, i32)>,
    input_worker_started: Arc<AtomicBool>,
    pub app_db: Arc<std::sync::Mutex<crate::service::app_db::AppDatabase>>,
    /// Serializes explicit catalog scans.  The WebSocket handlers run these
    /// filesystem-heavy jobs off the async reactor, but they must not compete
    /// with one another for disk bandwidth.
    pub scan_gate: Arc<Mutex<()>>,
    pub window_list_cache: Arc<Mutex<WindowListCache>>,
    pub pairing: Arc<Mutex<PairingManager>>,
    pub pairing_events: broadcast::Sender<pairing::PairingNotification>,
}

impl AppState {
    pub fn new(service: RemoteControlService, database: Arc<DatabaseService>) -> Self {
        let (tx, _) = broadcast::channel(256);
        let (pairing_events, _) = broadcast::channel(32);
        let scanner_config_path = service.paths().config("scanner_custom_dirs.json");
        let mut window_monitor = WindowMonitor::new();
        window_monitor.set_event_sender(tx.clone());
        let token_store = Arc::new(auth::TokenStore::new(database.clone()));
        let (input_move_tx, _) = tokio::sync::watch::channel((0_i32, 0_i32));
        // Clean up expired tokens.
        token_store.cleanup();
        let app_db = Arc::new(std::sync::Mutex::new(
            crate::service::app_db::AppDatabase::new(database.clone()),
        ));
        // Restore the persisted directory before accepting requests.  A fresh
        // process must not make the first mobile list request scan the disk.
        let cached_apps = app_db
            .lock()
            .ok()
            .and_then(|db| db.get_visible_apps().ok())
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect();
        let game_scan_completed = app_db
            .lock()
            .ok()
            .and_then(|db| db.scan_completed("game").ok())
            .unwrap_or(false);
        let cached_steam_games = if game_scan_completed {
            app_db
                .lock()
                .ok()
                .and_then(|db| db.get_steam_games().ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let app_scanner = Arc::new(AppScanner::with_cached_apps(
            scanner_config_path,
            cached_apps,
        ));
        // An empty Steam table is not a useful cache marker: it can mean the
        // previous scan ran before Steam was detected. Let the scanner probe
        // again so installed Steam libraries are not permanently skipped.
        if game_scan_completed && !cached_steam_games.is_empty() {
            app_scanner.replace_steam_games(cached_steam_games);
        }
        Self {
            service: Arc::new(service),
            system_service: Arc::new(SystemService::new()),
            process_manager: Arc::new(std::sync::Mutex::new(ProcessManager::new())),
            token_store,
            events: tx,
            app_scanner,
            window_monitor: Arc::new(std::sync::Mutex::new(window_monitor)),
            connection_registry: Arc::new(connection_registry::ConnectionRegistry::new()),
            input_connection_registry: Arc::new(connection_registry::ConnectionRegistry::new()),
            input_move_tx,
            input_worker_started: Arc::new(AtomicBool::new(false)),
            app_db,
            scan_gate: Arc::new(Mutex::new(())),
            window_list_cache: Arc::new(Mutex::new(None)),
            pairing: Arc::new(Mutex::new(PairingManager::default())),
            pairing_events,
        }
    }

    fn start_input_worker(&self) {
        if self.input_worker_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut input_move_rx = self.input_move_tx.subscribe();
        tokio::spawn(async move {
            while input_move_rx.changed().await.is_ok() {
                let (dx, dy) = *input_move_rx.borrow_and_update();
                if dx != 0 || dy != 0 {
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Err(error) = crate::platform::mouse_move(dx, dy) {
                            tracing::warn!(dx, dy, %error, "binary mouse move failed");
                        }
                    })
                    .await;
                }
            }
        });
    }

    pub fn list_windows(&self, force_refresh: bool) -> Vec<crate::protocol::requests::WindowInfo> {
        const WINDOW_CACHE_TTL: Duration = Duration::from_millis(500);
        if !force_refresh
            && let Ok(cache) = self.window_list_cache.lock()
            && let Some((created_at, windows)) = cache.as_ref()
            && created_at.elapsed() < WINDOW_CACHE_TTL
        {
            return windows.clone();
        }
        let windows = crate::platform::enum_windows_list();
        if let Ok(mut cache) = self.window_list_cache.lock() {
            *cache = Some((Instant::now(), windows.clone()));
        }
        windows
    }

    pub fn invalidate_window_list(&self) {
        if let Ok(mut cache) = self.window_list_cache.lock() {
            *cache = None;
        }
    }
}

pub(crate) fn reload_visible_catalog(state: &AppState) {
    tracing::info!("[catalog] reload_visible_catalog: 等待 app_db 锁");
    let apps: Vec<_> = state
        .app_db
        .lock()
        .ok()
        .and_then(|db| db.get_visible_apps().ok())
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect();
    tracing::info!(
        count = apps.len(),
        "[catalog] reload_visible_catalog: 查询完成，替换扫描器缓存"
    );
    state.app_scanner.replace_cache(apps);
    tracing::info!("[catalog] reload_visible_catalog: 完成");
}

pub(crate) fn scan_catalog_kind(state: &AppState, kind: &str) -> Result<usize, String> {
    tracing::info!(
        kind,
        "[remote-control] Starting background application catalog scan"
    );
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| "database lock is poisoned".to_string())?;
        db.initialize()?;
    }
    let apps = if kind == "game" {
        state.app_scanner.scan_games()
    } else {
        state.app_scanner.scan_apps()
    };
    let db = state
        .app_db
        .lock()
        .map_err(|_| "database lock is poisoned".to_string())?;
    db.sync_scanned_kind(kind, &apps)?;
    if kind == "game" {
        db.sync_steam_games(&state.app_scanner.get_steam_games())?;
    }
    db.mark_scan_completed(kind)?;
    drop(db);
    reload_visible_catalog(state);
    let _ = state
        .events
        .send(crate::protocol::requests::WsEvent::LauncherCatalogReady {
            kind: kind.to_string(),
        });
    tracing::info!(
        kind,
        count = apps.len(),
        "[remote-control] Application catalog scan complete"
    );
    Ok(apps.len())
}

fn scan_initial_catalog(state: &AppState) {
    let _scan_permit = crate::lock_unpoisoned(&state.scan_gate);
    for kind in ["app", "game"] {
        let already_scanned = state
            .app_db
            .lock()
            .ok()
            .and_then(|db| db.scan_completed(kind).ok())
            .unwrap_or(false);
        let steam_catalog_needs_probe = kind == "game" && !state.app_scanner.steam_cache_loaded();
        if already_scanned && !steam_catalog_needs_probe {
            continue;
        }
        if let Err(error) = scan_catalog_kind(state, kind) {
            tracing::error!(kind, %error, "[remote-control] Application catalog scan failed");
        }
    }
}

pub use auth::TokenStore;

pub struct RemoteServer;

impl RemoteServer {
    pub fn create_router(state: AppState) -> Router {
        Router::new()
            .route("/api/v2/health", axum::routing::get(pairing::health))
            .route(
                "/api/v2/pairing/requests",
                axum::routing::post(pairing::create_request),
            )
            .route(
                "/api/v2/pairing/confirm",
                axum::routing::post(pairing::confirm_request),
            )
            .route(
                "/api/v2/pairing/requests/:request_id",
                axum::routing::delete(pairing::cancel_request),
            )
            .route("/api/v2/events", axum::routing::get(ws_handler::ws_handler))
            .route(
                "/api/v2/input",
                axum::routing::get(input_ws_handler::input_ws_handler),
            )
            .layer(Extension(state))
    }

    pub async fn run(
        state: AppState,
        port: u16,
    ) -> anyhow::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
        Self::run_with_shutdown(state, port, std::future::pending()).await
    }

    pub async fn run_with_shutdown(
        state: AppState,
        port: u16,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> anyhow::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
        state.start_input_worker();
        tracing::info!("[remote-control] Attempting to bind port {}...", port);
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        let addr = listener.local_addr()?;
        state.service.set_server_running(true, addr.port());
        {
            let mut monitor = state
                .window_monitor
                .lock()
                .map_err(|_| anyhow::anyhow!("window monitor lock is poisoned"))?;
            monitor.start();
        }
        let service = Arc::clone(&state.service);
        let initial_state = state.clone();
        let app = Self::create_router(state);
        tokio::task::spawn_blocking(move || scan_initial_catalog(&initial_state));
        tracing::info!(
            "[remote-control] WebSocket server bound: {} (0.0.0.0:{})",
            addr,
            port
        );
        let handle = tokio::spawn(async move {
            tracing::info!("[remote-control] WebSocket service listening...");
            let serve = axum::serve(listener, app).with_graceful_shutdown(shutdown);
            let _ = serve.into_future().await;
            service.set_server_running(false, addr.port());
            tracing::warn!("[remote-control] WebSocket service stopped");
        });
        Ok((addr, handle))
    }
}
