use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::protocol::requests::{WsClientRequest, WsServerMessage, WsServerResponse};
use crate::server::AppState;
use crate::server::input_protocol::MouseMoveFrame;
use crate::server::ws_common::{AUTH_TIMEOUT_SECS, Heartbeat};
use crate::server::ws_request_handler;

struct InputConnState {
    authenticated: bool,
    token: Option<String>,
}

pub async fn input_ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::Extension(state): axum::extract::Extension<AppState>,
) -> impl IntoResponse {
    let registry = state.input_connection_registry.clone();
    let conn_state = Arc::new(Mutex::new(InputConnState {
        authenticated: false,
        token: None,
    }));
    ws.max_message_size(64 * 1024)
        .max_frame_size(64 * 1024)
        .on_upgrade(|socket| handle_socket(socket, state, registry, conn_state))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    registry: Arc<crate::server::connection_registry::ConnectionRegistry>,
    conn_state: Arc<Mutex<InputConnState>>,
) {
    let (conn_id, mut close_rx) = registry.register();
    let mut heartbeat = Heartbeat::start().await;
    let mut auth_timeout = Box::pin(sleep(Duration::from_secs(AUTH_TIMEOUT_SECS)));

    loop {
        tokio::select! {
            Some(reason) = close_rx.recv() => {
                let _ = socket.send(Message::Close(Some(CloseFrame {
                    code: 1000,
                    reason: reason.into(),
                }))).await;
                break;
            }
            _ = &mut auth_timeout, if !conn_state.lock().await.authenticated => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            result = socket.recv() => {
                match result {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Pong(_))) => heartbeat.on_pong(),
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() { break; }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        if !conn_state.lock().await.authenticated {
                            let _ = socket.send(Message::Close(Some(CloseFrame { code: 1008, reason: "AUTH_REQUIRED".into() }))).await;
                            break;
                        }
                        let Ok(frame) = MouseMoveFrame::parse(&data) else {
                            let _ = socket.send(Message::Close(Some(CloseFrame { code: 1003, reason: "INVALID_INPUT_FRAME".into() }))).await;
                            break;
                        };
                        let dx = frame.dx as i32;
                        let dy = frame.dy as i32;
                        // Keep binary input on the same worker route as the
                        // rest of the input pipeline.  This avoids blocking
                        // the WebSocket task and guarantees a single platform
                        // injection per frame.
                        if state.input_move_tx.send((dx, dy)).is_err() {
                            let response = WsServerMessage::Response(WsServerResponse::error(
                                "MOUSE_INPUT_FAILED",
                                "输入工作线程不可用",
                            ));
                            if ws_request_handler::send_ws_message(&mut socket, response, None).await.is_err() {
                                break;
                            }
                        }
                        registry.activity(conn_id);
                    }
                    Some(Ok(Message::Text(text))) => {
                        let request_id = serde_json::from_str::<serde_json::Value>(&text)
                            .ok()
                            .and_then(|value| value.get("request_id")?.as_str().map(str::to_owned));
                        let Ok(req) = serde_json::from_str::<WsClientRequest>(&text) else {
                            let _ = ws_request_handler::send_ws_message(&mut socket, WsServerMessage::Response(WsServerResponse::error("INVALID_REQUEST", "请求格式无效")), request_id.as_deref()).await;
                            continue;
                        };
                        let authenticated = conn_state.lock().await.authenticated;
                        let response = match req {
                            WsClientRequest::Authenticate { method, token, .. } if !authenticated && method == "token" => {
                                match token {
                                    Some(token) if state.token_store.validate(&token) => {
                                        let mut guard = conn_state.lock().await;
                                        guard.authenticated = true;
                                        guard.token = Some(token.clone());
                                        registry.bind_token(conn_id, &token);
                                        WsServerMessage::Response(WsServerResponse::Authenticated(crate::protocol::responses::AuthenticatedResponse {
                                            token,
                                            session_id: uuid::Uuid::new_v4().to_string(),
                                            expires_at: None,
                                            mac_address: None,
                                        }))
                                    }
                                    _ => {
                                        let _ = socket.send(Message::Close(Some(CloseFrame {
                                            code: 1008,
                                            reason: "INVALID_TOKEN".into(),
                                        }))).await;
                                        break;
                                    }
                                }
                            }
                            WsClientRequest::Ping if authenticated => WsServerMessage::Response(WsServerResponse::Pong),
                            _ => WsServerMessage::Response(WsServerResponse::error("INPUT_REQUEST_NOT_ALLOWED", "输入通道只允许 Token 认证和心跳")),
                        };
                        if ws_request_handler::send_ws_message(&mut socket, response, request_id.as_deref()).await.is_err() { break; }
                    }
                    Some(Err(_)) => break,
                }
            }
            _ = heartbeat.tick() => {
                let token = conn_state.lock().await.token.clone();
                if token.as_deref().is_some_and(|token| !state.token_store.validate(token)) {
                    let _ = socket.send(Message::Close(Some(CloseFrame { code: 1008, reason: "REVOKED".into() }))).await;
                    break;
                }
                if heartbeat.note_tick() { break; }
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
    registry.unregister(conn_id);
}
