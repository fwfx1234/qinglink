//! 控制 WebSocket 通道：认证、事件推送与客户端请求分发。

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::protocol::requests::{WsClientRequest, WsEvent, WsServerMessage, WsServerResponse};
use crate::server::AppState;
use crate::server::connection_registry::{ConnectionId, ConnectionRegistry};
use crate::server::ws_common::{AUTH_TIMEOUT_SECS, Heartbeat};
use crate::server::ws_request_handler;

struct ConnState {
    authenticated: bool,
    token: Option<String>,
}

/// 控制通道单个会话的共享上下文：被拆出的处理函数通过它读取状态与注册表。
struct ControlSession {
    state: AppState,
    registry: Arc<ConnectionRegistry>,
    conn_state: Arc<Mutex<ConnState>>,
    conn_id: ConnectionId,
    connection_generation: u64,
}

fn update_pressed_button_state(
    pressed_buttons: &mut HashSet<String>,
    button: &str,
    pressed: bool,
    succeeded: bool,
) {
    if !succeeded || !matches!(button, "left" | "right") {
        return;
    }
    if pressed {
        pressed_buttons.insert(button.to_string());
    } else {
        pressed_buttons.remove(button);
    }
}

/// 升级为 WebSocket 并处理控制通道会话，直到认证超时、被撤销或连接关闭。
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::Extension(state): axum::extract::Extension<AppState>,
) -> impl IntoResponse {
    let registry = state.connection_registry.clone();
    let conn_state = Arc::new(Mutex::new(ConnState {
        authenticated: false,
        token: None,
    }));
    ws.max_message_size(128 * 1024)
        .max_frame_size(128 * 1024)
        .on_upgrade(|socket| handle_socket(socket, state, registry, conn_state))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    registry: Arc<ConnectionRegistry>,
    conn_state: Arc<Mutex<ConnState>>,
) {
    let (conn_id, mut close_rx) = registry.register();
    let connection_generation = state.input_generation();
    tracing::info!("[远程控制] WebSocket 已连接, conn_id={}", conn_id);

    let session = ControlSession {
        state,
        registry,
        conn_state,
        conn_id,
        connection_generation,
    };

    let mut events = session.state.events.subscribe();
    let mut heartbeat = Heartbeat::start().await;
    let mut auth_timeout = Box::pin(sleep(Duration::from_secs(AUTH_TIMEOUT_SECS)));
    let mut pressed_buttons = HashSet::<String>::new();

    loop {
        tokio::select! {
            Some(reason) = close_rx.recv() => {
                session.close_with_reason(&mut socket, reason).await;
                break;
            }
            _ = &mut auth_timeout, if !session.conn_state.lock().await.authenticated => {
                tracing::warn!("[远程控制] WebSocket 认证超时, conn_id={}", session.conn_id);
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            Ok(event) = events.recv(), if session.conn_state.lock().await.authenticated => {
                if session.forward_event(&mut socket, event).await {
                    break;
                }
            }
            result = socket.recv() => {
                if session.handle_recv_result(result, &mut socket, &mut heartbeat, &mut pressed_buttons).await {
                    break;
                }
            }
            _ = heartbeat.tick() => {
                if session.handle_heartbeat_tick(&mut socket, &mut heartbeat).await {
                    break;
                }
            }
        }
    }

    session.release_pressed_buttons(&mut pressed_buttons).await;
    session.registry.unregister(session.conn_id);
    tracing::info!(
        "[远程控制] WebSocket 连接已关闭, conn_id={}",
        session.conn_id
    );
}

impl ControlSession {
    /// 处理注册表下发的关闭通知：补发 `session_closed` 后以 1000 正常关闭。
    async fn close_with_reason(&self, socket: &mut WebSocket, reason: String) {
        tracing::info!(
            "[远程控制] WebSocket 会话被关闭, conn_id={}, reason={}",
            self.conn_id,
            reason
        );
        let _ = socket
            .send(Message::Text(
                serde_json::json!({
                    "type": "session_closed",
                    "data": { "code": reason.clone() },
                })
                .to_string(),
            ))
            .await;
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: 1000,
                reason: reason.into(),
            })))
            .await;
    }

    /// 处理一次 `socket.recv()` 的结果，返回 true 表示会话循环必须结束。
    async fn handle_recv_result(
        &self,
        result: Option<Result<Message, axum::Error>>,
        socket: &mut WebSocket,
        heartbeat: &mut Heartbeat,
        pressed_buttons: &mut HashSet<String>,
    ) -> bool {
        match result {
            Some(Ok(msg)) => {
                self.registry.activity(self.conn_id);
                match msg {
                    Message::Close(_) => true,
                    Message::Pong(_) => {
                        heartbeat.on_pong();
                        false
                    }
                    Message::Ping(data) => socket.send(Message::Pong(data)).await.is_err(),
                    Message::Text(text) => {
                        self.handle_text_frame(&text, socket, pressed_buttons).await
                    }
                    _ => false,
                }
            }
            Some(Err(error)) => {
                tracing::debug!(
                    "[远程控制] WebSocket 接收失败, conn_id={}, error={}",
                    self.conn_id,
                    error
                );
                true
            }
            None => true,
        }
    }

    /// 处理控制通道的文本帧：解析请求、鉴权、分发、回包与撤销关闭。
    async fn handle_text_frame(
        &self,
        text: &str,
        socket: &mut WebSocket,
        pressed_buttons: &mut HashSet<String>,
    ) -> bool {
        let request_id = serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|value| value.get("request_id")?.as_str().map(str::to_owned))
            .filter(|id| !id.is_empty() && id.len() <= 128);
        let Ok(req) = serde_json::from_str::<WsClientRequest>(text) else {
            let response = WsServerMessage::Response(WsServerResponse::error(
                "INVALID_REQUEST",
                "请求格式无效",
            ));
            return ws_request_handler::send_ws_message(socket, response, request_id.as_deref())
                .await
                .is_err();
        };

        let is_auth = self.conn_state.lock().await.authenticated;
        let is_auth_request = matches!(
            &req,
            WsClientRequest::Authenticate { .. } | WsClientRequest::Ping
        );
        let session_token = self.conn_state.lock().await.token.clone();
        let resp = if !is_auth && !is_auth_request {
            WsServerMessage::Response(WsServerResponse::error(
                "UNAUTHORIZED",
                "Authentication required",
            ))
        } else {
            let token_for_request = if is_auth { session_token.clone() } else { None };
            self.dispatch_client_request(&req, token_for_request.as_deref())
                .await
        };
        self.apply_authenticated_response(&resp).await;

        if let WsClientRequest::MouseButtonState { button, pressed } = &req {
            let succeeded = matches!(&resp, WsServerMessage::Response(WsServerResponse::Success));
            update_pressed_button_state(pressed_buttons, button, *pressed, succeeded);
        }

        let should_close_after_revoke = matches!(&req, WsClientRequest::RevokeToken)
            && matches!(&resp, WsServerMessage::Response(WsServerResponse::Revoked));
        let should_reply = !matches!(
            &req,
            WsClientRequest::MouseMove { .. } | WsClientRequest::MouseMoveAbsolute { .. }
        ) || !is_auth;
        if should_reply
            && ws_request_handler::send_ws_message(socket, resp, request_id.as_deref())
                .await
                .is_err()
        {
            return true;
        }
        if should_close_after_revoke {
            if let Some(token) = session_token.as_deref() {
                self.state.connection_registry.close_token(token, "REVOKED");
                self.state
                    .input_connection_registry
                    .close_token(token, "REVOKED");
            }
            return true;
        }
        false
    }

    /// 认证成功时把连接标记为已认证，并把令牌绑定到注册表。
    async fn apply_authenticated_response(&self, resp: &WsServerMessage) {
        if let WsServerMessage::Response(WsServerResponse::Authenticated(auth)) = resp {
            let mut state_guard = self.conn_state.lock().await;
            state_guard.authenticated = true;
            state_guard.token = Some(auth.token.clone());
            self.registry.bind_token(self.conn_id, &auth.token);
        }
    }

    /// 在阻塞线程池中执行客户端请求，返回待回包的服务端消息。
    async fn dispatch_client_request(
        &self,
        request: &WsClientRequest,
        token: Option<&str>,
    ) -> WsServerMessage {
        let state_clone = self.state.clone();
        let request_clone = request.clone();
        let token_for_request = token.map(str::to_owned);
        let generation = self.connection_generation;
        tokio::task::spawn_blocking(move || {
            ws_request_handler::handle_client_request_in_generation(
                &state_clone,
                &request_clone,
                token_for_request.as_deref(),
                generation,
            )
        })
        .await
        .unwrap_or_else(|_| {
            WsServerMessage::Response(WsServerResponse::error(
                "INTERNAL_ERROR",
                "请求处理任务失败",
            ))
        })
    }

    /// 向已认证客户端推送一条服务端事件，返回 true 表示推送失败需结束会话。
    async fn forward_event(&self, socket: &mut WebSocket, event: WsEvent) -> bool {
        self.registry.activity(self.conn_id);
        let msg = WsServerMessage::Event(event);
        ws_request_handler::send_ws_message(socket, msg, None)
            .await
            .is_err()
    }

    /// 处理一次心跳 tick：令牌失效、心跳超时或补发 Ping。
    async fn handle_heartbeat_tick(
        &self,
        socket: &mut WebSocket,
        heartbeat: &mut Heartbeat,
    ) -> bool {
        let token = self.conn_state.lock().await.token.clone();
        if let Some(token) = token
            && !self.state.token_store.validate(&token)
        {
            tracing::info!(
                "[远程控制] WebSocket Token 已失效, conn_id={}",
                self.conn_id
            );
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "session_closed",
                        "data": { "code": "REVOKED" },
                    })
                    .to_string(),
                ))
                .await;
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1008,
                    reason: "REVOKED".into(),
                })))
                .await;
            return true;
        }
        if heartbeat.note_tick() {
            tracing::warn!("[远程控制] WebSocket 心跳超时, conn_id={}", self.conn_id);
            return true;
        }
        socket
            .send(Message::Ping(
                format!("hb-{}", heartbeat.missed_pongs()).into_bytes(),
            ))
            .await
            .is_err()
    }

    /// 会话结束时释放本连接成功按下的所有鼠标键，避免客户端消失后按键卡住。
    async fn release_pressed_buttons(&self, pressed_buttons: &mut HashSet<String>) {
        // A client can disappear between any two messages. Release every button
        // that this connection successfully held through the same platform input
        // path used for normal button-state requests.
        for button in pressed_buttons.drain() {
            let button_for_release = button.clone();
            let cleanup_state = self.state.clone();
            let connection_generation = self.connection_generation;
            match tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let _guard = cleanup_state
                    .control
                    .input_gate
                    .lock()
                    .map_err(|_| anyhow::anyhow!("input gate poisoned"))?;
                // Pause/restart cleanup owns all old inputs after the epoch changes.
                if cleanup_state.input_generation() != connection_generation {
                    return Ok(());
                }
                crate::platform::mouse_button_state(&button_for_release, false)?;
                if let Ok(mut held) = cleanup_state.held_input.lock() {
                    held.button(&button_for_release, false);
                }
                Ok(())
            })
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%button, %error, "failed to release mouse button on WebSocket close")
                }
                Err(error) => tracing::warn!(%button, %error, "mouse button cleanup task failed"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::update_pressed_button_state;
    use std::collections::HashSet;

    #[test]
    fn only_successful_left_and_right_state_changes_are_tracked() {
        let mut buttons = HashSet::new();
        update_pressed_button_state(&mut buttons, "left", true, false);
        assert!(buttons.is_empty());
        update_pressed_button_state(&mut buttons, "left", true, true);
        assert!(buttons.contains("left"));
        update_pressed_button_state(&mut buttons, "left", false, false);
        assert!(buttons.contains("left"));
        update_pressed_button_state(&mut buttons, "left", false, true);
        assert!(buttons.is_empty());
    }

    #[test]
    fn repeated_release_and_clicks_are_safe() {
        let mut buttons = HashSet::new();
        update_pressed_button_state(&mut buttons, "right", false, true);
        update_pressed_button_state(&mut buttons, "right", false, true);
        update_pressed_button_state(&mut buttons, "middle", true, true);
        assert!(buttons.is_empty());
    }
}
