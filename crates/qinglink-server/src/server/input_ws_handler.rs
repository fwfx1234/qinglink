//! 输入 WebSocket 通道：令牌认证、心跳与二进制鼠标位移帧。

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::protocol::requests::{WsClientRequest, WsServerMessage, WsServerResponse};
use crate::server::AppState;
use crate::server::connection_registry::{ConnectionId, ConnectionRegistry};
use crate::server::input_protocol::MouseMoveFrame;
use crate::server::ws_common::{AUTH_TIMEOUT_SECS, Heartbeat};
use crate::server::ws_request_handler;

struct InputConnState {
    authenticated: bool,
    token: Option<String>,
}

/// 输入通道单个会话的共享上下文：被拆出的处理函数通过它读取状态与注册表。
struct InputSession {
    state: AppState,
    registry: Arc<ConnectionRegistry>,
    conn_state: Arc<Mutex<InputConnState>>,
    conn_id: ConnectionId,
    connection_generation: u64,
}

/// 升级为 WebSocket 并处理输入通道会话，直到认证超时或连接关闭。
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
    registry: Arc<ConnectionRegistry>,
    conn_state: Arc<Mutex<InputConnState>>,
) {
    let (conn_id, mut close_rx) = registry.register();
    let connection_generation = state.input_generation();

    let session = InputSession {
        state,
        registry,
        conn_state,
        conn_id,
        connection_generation,
    };

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
            _ = &mut auth_timeout, if !session.conn_state.lock().await.authenticated => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            result = socket.recv() => {
                if session.handle_recv_result(result, &mut socket, &mut heartbeat).await {
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
    session.registry.unregister(session.conn_id);
}

impl InputSession {
    /// 处理一次 `socket.recv()` 的结果，返回 true 表示会话循环必须结束。
    async fn handle_recv_result(
        &self,
        result: Option<Result<Message, axum::Error>>,
        socket: &mut WebSocket,
        heartbeat: &mut Heartbeat,
    ) -> bool {
        match result {
            Some(Ok(Message::Close(_))) | None => true,
            Some(Ok(Message::Pong(_))) => {
                heartbeat.on_pong();
                false
            }
            Some(Ok(Message::Ping(data))) => socket.send(Message::Pong(data)).await.is_err(),
            Some(Ok(Message::Binary(data))) => self.handle_binary_frame(&data, socket).await,
            Some(Ok(Message::Text(text))) => self.handle_text_frame(&text, socket).await,
            Some(Err(_)) => true,
        }
    }

    /// 处理输入通道的二进制帧：校验认证与帧格式后投递鼠标位移。
    async fn handle_binary_frame(&self, data: &[u8], socket: &mut WebSocket) -> bool {
        if !self.conn_state.lock().await.authenticated {
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1008,
                    reason: "AUTH_REQUIRED".into(),
                })))
                .await;
            return true;
        }
        let Ok(frame) = MouseMoveFrame::parse(data) else {
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1003,
                    reason: "INVALID_INPUT_FRAME".into(),
                })))
                .await;
            return true;
        };
        let dx = frame.dx as i32;
        let dy = frame.dy as i32;
        // Keep binary input on the same worker route as the
        // rest of the input pipeline.  This avoids blocking
        // the WebSocket task and guarantees a single platform
        // injection per frame.
        if self.state.control.is_paused()
            || self.state.input_generation() != self.connection_generation
        {
            return true;
        }
        if self
            .state
            .input_move_tx
            .send((self.connection_generation, dx, dy))
            .is_err()
        {
            let response = WsServerMessage::Response(WsServerResponse::error(
                "MOUSE_INPUT_FAILED",
                "输入工作线程不可用",
            ));
            if ws_request_handler::send_ws_message(socket, response, None)
                .await
                .is_err()
            {
                return true;
            }
        }
        self.registry.activity(self.conn_id);
        false
    }

    /// 处理输入通道的文本帧：只允许 Token 认证与心跳请求。
    async fn handle_text_frame(&self, text: &str, socket: &mut WebSocket) -> bool {
        let request_id = serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|value| value.get("request_id")?.as_str().map(str::to_owned));
        let Ok(req) = serde_json::from_str::<WsClientRequest>(text) else {
            let _ = ws_request_handler::send_ws_message(
                socket,
                WsServerMessage::Response(WsServerResponse::error(
                    "INVALID_REQUEST",
                    "请求格式无效",
                )),
                request_id.as_deref(),
            )
            .await;
            return false;
        };
        let authenticated = self.conn_state.lock().await.authenticated;
        let response = match req {
            WsClientRequest::Authenticate { method, token, .. }
                if !authenticated && method == "token" =>
            {
                match token {
                    Some(token) if self.state.token_store.validate(&token) => {
                        let mut guard = self.conn_state.lock().await;
                        guard.authenticated = true;
                        guard.token = Some(token.clone());
                        self.registry.bind_token(self.conn_id, &token);
                        WsServerMessage::Response(WsServerResponse::Authenticated(
                            crate::protocol::responses::AuthenticatedResponse {
                                token,
                                session_id: uuid::Uuid::new_v4().to_string(),
                                expires_at: None,
                                mac_address: None,
                            },
                        ))
                    }
                    _ => {
                        let _ = socket
                            .send(Message::Close(Some(CloseFrame {
                                code: 1008,
                                reason: "INVALID_TOKEN".into(),
                            })))
                            .await;
                        return true;
                    }
                }
            }
            WsClientRequest::Ping if authenticated => {
                WsServerMessage::Response(WsServerResponse::Pong)
            }
            _ => WsServerMessage::Response(WsServerResponse::error(
                "INPUT_REQUEST_NOT_ALLOWED",
                "输入通道只允许 Token 认证和心跳",
            )),
        };
        ws_request_handler::send_ws_message(socket, response, request_id.as_deref())
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
        if token
            .as_deref()
            .is_some_and(|token| !self.state.token_store.validate(token))
        {
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1008,
                    reason: "REVOKED".into(),
                })))
                .await;
            return true;
        }
        if heartbeat.note_tick() {
            return true;
        }
        socket
            .send(Message::Ping(
                format!("hb-{}", heartbeat.missed_pongs()).into_bytes(),
            ))
            .await
            .is_err()
    }
}
