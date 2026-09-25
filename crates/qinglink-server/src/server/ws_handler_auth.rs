use super::*;

pub(super) fn handle(state: &AppState, req: &WsClientRequest, session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === Auth (no auth required) ===
        WsClientRequest::Authenticate {
            method,
            token,
            pin: _,
            device_name: _,
        } => match method.as_str() {
            "token" => match token {
                Some(token) if state.token_store.validate(token) => {
                    WsServerMessage::Response(WsServerResponse::Authenticated(
                        crate::protocol::responses::AuthenticatedResponse {
                            token: token.clone(),
                            session_id: uuid::Uuid::new_v4().to_string(),
                            expires_at: None,
                            mac_address: None,
                        },
                    ))
                }
                _ => WsServerMessage::Response(WsServerResponse::error(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            },
            "pair" => WsServerMessage::Response(WsServerResponse::error(
                "PAIRING_V2_REQUIRED",
                "Use /api/v2/pairing/requests and /api/v2/pairing/confirm",
            )),
            _ => WsServerMessage::Response(WsServerResponse::error(
                "INVALID_AUTH_METHOD",
                "Unsupported authentication method",
            )),
        },
        WsClientRequest::Ping => WsServerMessage::Response(WsServerResponse::Pong),
        WsClientRequest::RevokeToken => match session_token {
            Some(token) if state.token_store.revoke(token) => {
                tracing::info!("[远程控制] 设备主动撤销了当前令牌");
                WsServerMessage::Response(WsServerResponse::Revoked)
            }
            _ => WsServerMessage::Response(WsServerResponse::error(
                "REVOKE_FAILED",
                "当前设备令牌已失效",
            )),
        },

        _ => unreachable!("request routed to the wrong handler"),
    }
}


