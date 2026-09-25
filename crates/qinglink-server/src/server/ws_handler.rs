use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::server::AppState;
use crate::server::ws_common::{AUTH_TIMEOUT_SECS, Heartbeat};
use crate::server::ws_request_handler;

struct ConnState {
    authenticated: bool,
    token: Option<String>,
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
    registry: Arc<crate::server::connection_registry::ConnectionRegistry>,
    conn_state: Arc<Mutex<ConnState>>,
) {
    let (conn_id, mut close_rx) = registry.register();
    tracing::info!("[远程控制] WebSocket 已连接, conn_id={}", conn_id);

    let mut events = state.events.subscribe();
    let mut heartbeat = Heartbeat::start().await;
    let mut auth_timeout = Box::pin(sleep(Duration::from_secs(AUTH_TIMEOUT_SECS)));
    let mut pressed_buttons = HashSet::<String>::new();

    loop {
        tokio::select! {
            Some(reason) = close_rx.recv() => {
                tracing::info!("[远程控制] WebSocket 会话被关闭, conn_id={}, reason={}", conn_id, reason);
                let _ = socket.send(Message::Text(serde_json::json!({
                    "type": "session_closed",
                    "data": { "code": reason.clone() },
                }).to_string())).await;
                let _ = socket.send(Message::Close(Some(CloseFrame {
                    code: 1000,
                    reason: reason.into(),
                }))).await;
                break;
            }
            _ = &mut auth_timeout, if !conn_state.lock().await.authenticated => {
                tracing::warn!("[远程控制] WebSocket 认证超时, conn_id={}", conn_id);
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            Ok(event) = events.recv(), if conn_state.lock().await.authenticated => {
                registry.activity(conn_id);
                let msg = crate::protocol::requests::WsServerMessage::Event(event);
                if ws_request_handler::send_ws_message(&mut socket, msg, None).await.is_err() {
                    break;
                }
            }
            result = socket.recv() => {
                match result {
                    Some(Ok(msg)) => {
                        registry.activity(conn_id);
                        match msg {
                    Message::Close(_) => break,
                    Message::Pong(_) => heartbeat.on_pong(),
                    Message::Ping(data) => {
                        if socket.send(Message::Pong(data)).await.is_err() { break; }
                    }
                    Message::Text(text) => {
                        let request_id = serde_json::from_str::<serde_json::Value>(&text)
                            .ok()
                            .and_then(|value| value.get("request_id")?.as_str().map(str::to_owned))
                            .filter(|id| !id.is_empty() && id.len() <= 128);
                        let Ok(req) = serde_json::from_str::<crate::protocol::requests::WsClientRequest>(&text) else {
                            let response = crate::protocol::requests::WsServerMessage::Response(
                                crate::protocol::requests::WsServerResponse::error("INVALID_REQUEST", "请求格式无效"),
                            );
                            if ws_request_handler::send_ws_message(&mut socket, response, request_id.as_deref()).await.is_err() {
                                break;
                            }
                            continue;
                        };

                        let is_auth = conn_state.lock().await.authenticated;
                        let is_auth_request = matches!(
                            &req,
                            crate::protocol::requests::WsClientRequest::Authenticate { .. }
                                | crate::protocol::requests::WsClientRequest::Ping
                        );
                        let session_token = conn_state.lock().await.token.clone();
                        let resp = if !is_auth && !is_auth_request {
                            crate::protocol::requests::WsServerMessage::Response(
                                crate::protocol::requests::WsServerResponse::error(
                                    "UNAUTHORIZED",
                                    "Authentication required",
                                ),
                            )
                        } else {
                            let state_clone = state.clone();
                            let req_clone = req.clone();
                            let token_for_request = if is_auth { session_token.clone() } else { None };
                            tokio::task::spawn_blocking(move || {
                                ws_request_handler::handle_client_request(
                                    &state_clone,
                                    &req_clone,
                                    token_for_request.as_deref(),
                                )
                            }).await.unwrap_or_else(|_| crate::protocol::requests::WsServerMessage::Response(
                                crate::protocol::requests::WsServerResponse::error("INTERNAL_ERROR", "请求处理任务失败")
                            ))
                        };

                        if let crate::protocol::requests::WsServerMessage::Response(
                            crate::protocol::requests::WsServerResponse::Authenticated(auth),
                        ) = &resp {
                            let mut state_guard = conn_state.lock().await;
                            state_guard.authenticated = true;
                            state_guard.token = Some(auth.token.clone());
                            registry.bind_token(conn_id, &auth.token);
                        }

                        if let crate::protocol::requests::WsClientRequest::MouseButtonState {
                            button,
                            pressed,
                        } = &req
                        {
                            let succeeded = matches!(
                                &resp,
                                crate::protocol::requests::WsServerMessage::Response(
                                    crate::protocol::requests::WsServerResponse::Success
                                )
                            );
                            update_pressed_button_state(
                                &mut pressed_buttons,
                                button,
                                *pressed,
                                succeeded,
                            );
                        }

                        let should_close_after_revoke = matches!(
                            &req,
                            crate::protocol::requests::WsClientRequest::RevokeToken
                        ) && matches!(
                            resp,
                            crate::protocol::requests::WsServerMessage::Response(
                                crate::protocol::requests::WsServerResponse::Revoked
                            )
                        );
                        let should_reply = !matches!(
                            &req,
                            crate::protocol::requests::WsClientRequest::MouseMove { .. }
                                | crate::protocol::requests::WsClientRequest::MouseMoveAbsolute { .. }
                        ) || !is_auth;
                        if should_reply && ws_request_handler::send_ws_message(&mut socket, resp, request_id.as_deref()).await.is_err() {
                            break;
                        }
                        if should_close_after_revoke
                            && let Some(token) = session_token.as_deref() {
                                state.connection_registry.close_token(token, "REVOKED");
                                state.input_connection_registry.close_token(token, "REVOKED");
                            }
                        if should_close_after_revoke {
                            break;
                        }
                    }
                            _ => {}
                        }
                    }
                    Some(Err(error)) => {
                        tracing::debug!("[远程控制] WebSocket 接收失败, conn_id={}, error={}", conn_id, error);
                        break;
                    }
                    None => break,
                }
            }
            _ = heartbeat.tick() => {
                let token = conn_state.lock().await.token.clone();
                if let Some(token) = token
                    && !state.token_store.validate(&token) {
                        tracing::info!("[远程控制] WebSocket Token 已失效, conn_id={}", conn_id);
                        let _ = socket.send(Message::Text(serde_json::json!({
                            "type": "session_closed",
                            "data": { "code": "REVOKED" },
                        }).to_string())).await;
                        let _ = socket.send(Message::Close(Some(CloseFrame {
                            code: 1008,
                            reason: "REVOKED".into(),
                        }))).await;
                        break;
                    }
                if heartbeat.note_tick() {
                    tracing::warn!("[远程控制] WebSocket 心跳超时, conn_id={}", conn_id);
                    break;
                }
                if socket
                    .send(Message::Ping(format!("hb-{}", heartbeat.missed_pongs()).into_bytes()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }

    // A client can disappear between any two messages. Release every button
    // that this connection successfully held through the same platform input
    // path used for normal button-state requests.
    for button in pressed_buttons.drain() {
        let button_for_release = button.clone();
        match tokio::task::spawn_blocking(move || {
            crate::platform::mouse_button_state(&button_for_release, false)
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

    registry.unregister(conn_id);
    tracing::info!("[远程控制] WebSocket 连接已关闭, conn_id={}", conn_id);
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
