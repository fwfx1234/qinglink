//! 服务连接状态、暂停控制，以及需要提权的 Windows 服务操作。
//!
//! 提权动作在请求被接受后立即返回。真正的结果通过 `service/install_state` 事件
//! 异步送达，因为 UAC 提示可能耗时任意长，而一个 HTTP 请求不能为此一直挂着连接。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use qinglink_protocol::service_ipc::DesktopCommand;
use serde::Deserialize;
use serde_json::{Value, json};

use super::super::error::{ApiError, ok};
use super::super::state::{ServerEvent, WebState};
use crate::input_install::{self, InputOperation};

/// `GET /service`
pub async fn status(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&json!(state.service.status()))
}

/// Body of `POST /service/pause`.
#[derive(Debug, Deserialize)]
pub struct PauseRequest {
    paused: bool,
}

/// `POST /service/pause`
pub async fn pause(
    State(state): State<Arc<WebState>>,
    Json(request): Json<PauseRequest>,
) -> Result<Json<Value>, ApiError> {
    state
        .service
        .call(DesktopCommand::SetPaused {
            paused: request.paused,
        })
        .await?;
    // The authoritative snapshot arrives over the event socket; report the
    // requested value so the caller can render an optimistic spinner only.
    Ok(ok(&json!({ "accepted": true, "paused": request.paused })))
}

/// `GET /service/install-state`
pub async fn install_state(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&json!(state.install_view()))
}

/// `POST /service/install`
pub async fn install(State(state): State<Arc<WebState>>) -> Json<Value> {
    input_install::start_elevated(&state.install_state, InputOperation::Install);
    state.publish_install_state();
    accepted()
}

/// `POST /service/uninstall`
pub async fn uninstall(State(state): State<Arc<WebState>>) -> Json<Value> {
    input_install::start_elevated(&state.install_state, InputOperation::Uninstall);
    state.publish_install_state();
    accepted()
}

/// `POST /service/start`
pub async fn start(State(state): State<Arc<WebState>>) -> Json<Value> {
    run_elevated(&state, input_install::start_service_elevated);
    accepted()
}

/// `POST /service/stop`
pub async fn stop(State(state): State<Arc<WebState>>) -> Json<Value> {
    run_elevated(&state, input_install::stop_service_elevated);
    accepted()
}

/// `POST /service/restart`
pub async fn restart(State(state): State<Arc<WebState>>) -> Json<Value> {
    run_elevated(&state, input_install::restart_service_elevated);
    accepted()
}

/// `POST /service/reconnect`
///
/// Cuts the reconnect backoff short and re-negotiates the management handshake
/// immediately. The authoritative result arrives over the event socket, exactly
/// like [`pause`].
pub async fn reconnect(State(state): State<Arc<WebState>>) -> Json<Value> {
    state.service.reconnect();
    accepted()
}

fn accepted() -> Json<Value> {
    ok(&json!({ "accepted": true }))
}

/// Run one elevated service operation off the request path.
///
/// The operation blocks on the UAC helper process, so it needs its own thread.
/// Progress and the final message are published as install-state events, which
/// is what the UI converges on.
fn run_elevated(state: &Arc<WebState>, operation: fn() -> anyhow::Result<String>) {
    {
        let Ok(mut shared) = state.install_state.lock() else {
            return;
        };
        if shared.busy {
            return;
        }
        shared.busy = true;
        shared.result_message = Some("正在请求管理员权限…".to_string());
    }
    state.publish_install_state();

    let web_state = Arc::clone(state);
    let shared = Arc::clone(&state.install_state);
    std::thread::spawn(move || {
        let result = operation();
        input_install::refresh(&shared);
        if let Ok(mut shared) = shared.lock() {
            shared.busy = false;
            shared.result_message = Some(match result {
                Ok(message) => message,
                Err(error) => format!("操作失败：{error:#}"),
            });
        }
        // Built through the state so the management-link signal is filled in
        // just like it is on `GET /service/install-state`.
        let view = web_state.install_view();
        let _ = web_state
            .events
            .send(ServerEvent::InstallState(Box::new(view)));
    });
}
