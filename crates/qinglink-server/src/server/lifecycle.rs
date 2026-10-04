//! 服务器装配与启动：axum 路由、端口绑定、优雅退出与首次目录扫描。

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Extension, Router};

use crate::server::AppState;
use crate::server::catalog::scan_initial_catalog;
use crate::server::input_ws_handler;
use crate::server::pairing;
use crate::server::ws_handler;

/// 远程控制 HTTP/WebSocket 服务的入口。
#[derive(Debug)]
pub struct RemoteServer;

impl RemoteServer {
    /// 构建挂载健康检查、配对、事件与输入路由的 axum Router。
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

    /// 绑定端口并启动服务，直到进程结束。
    ///
    /// # Errors
    /// 服务处于暂停状态、绑定 `port` 失败、取本地监听地址失败，或窗口监视器锁
    /// 中毒时返回 `Err`。
    pub async fn run(
        state: AppState,
        port: u16,
    ) -> anyhow::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
        Self::run_with_shutdown(state, port, std::future::pending()).await
    }

    /// 绑定端口并启动服务，`shutdown` 完成后优雅退出。
    ///
    /// # Errors
    /// 服务处于暂停状态、绑定 `port` 失败、取本地监听地址失败，或窗口监视器锁
    /// 中毒时返回 `Err`。
    pub async fn run_with_shutdown(
        state: AppState,
        port: u16,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> anyhow::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
        anyhow::ensure!(!state.control.is_paused(), "service paused");
        tracing::info!("[remote-control] Attempting to bind port {}...", port);
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        let addr = listener.local_addr()?;
        state.start_input_worker();
        state.service.set_server_running(true, addr.port());
        {
            let mut monitor = state
                .window_monitor
                .lock()
                .map_err(|_| anyhow::anyhow!("window monitor lock is poisoned"))?;
            monitor.start();
        }
        let service = Arc::clone(&state.service);
        let control = Arc::clone(&state.control);
        let initial_state = state.clone();
        let app = Self::create_router(state);
        tokio::task::spawn_blocking(move || {
            scan_initial_catalog(&initial_state);
            initial_state.backfill_catalog_icons();
        });
        tracing::info!(
            "[remote-control] WebSocket server bound: {} (0.0.0.0:{})",
            addr,
            port
        );
        let handle = tokio::spawn(async move {
            tracing::info!("[remote-control] WebSocket service listening...");
            let serve = axum::serve(listener, app).with_graceful_shutdown(shutdown);
            if let Err(error) = serve.into_future().await {
                control.set_network_error(Some(error.to_string()));
                tracing::error!(%error, "HTTP/WebSocket listener failed");
            }
            service.set_server_running(false, addr.port());
            tracing::warn!("[remote-control] WebSocket service stopped");
        });
        Ok((addr, handle))
    }
}
