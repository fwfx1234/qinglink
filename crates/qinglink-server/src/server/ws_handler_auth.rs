use super::*;

/// 令牌校验失败的统一响应：数据库不可用与"令牌失效"必须给出不同错误码。
///
/// 回 `REVOKED` 会让客户端清掉本地凭据、踢掉全部在线设备；而一次数据库抖动
/// 完全不该产生这个后果。所以这里用 `AUTH_UNAVAILABLE`，客户端可以提示重试。
pub(super) fn auth_unavailable(error: &anyhow::Error) -> WsServerMessage {
    tracing::error!(%error, "令牌校验失败：数据库不可用，本次不判定为令牌失效");
    WsServerMessage::Response(WsServerResponse::error(
        "AUTH_UNAVAILABLE",
        "设备授权校验暂时不可用，请稍后重试",
    ))
}

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Auth (no auth required) ===
        WsClientRequest::Authenticate {
            method,
            token,
            pin: _,
            device_name: _,
        } => match method.as_str() {
            "token" => {
                let Some(token) = token else {
                    return WsServerMessage::Response(WsServerResponse::error(
                        "INVALID_TOKEN",
                        "Invalid or expired token",
                    ));
                };
                match state.token_store.validate(token) {
                    Ok(true) => WsServerMessage::Response(WsServerResponse::Authenticated(
                        crate::protocol::responses::AuthenticatedResponse {
                            token: token.clone(),
                            session_id: uuid::Uuid::new_v4().to_string(),
                            expires_at: None,
                            mac_address: None,
                        },
                    )),
                    Ok(false) => WsServerMessage::Response(WsServerResponse::error(
                        "INVALID_TOKEN",
                        "Invalid or expired token",
                    )),
                    Err(error) => auth_unavailable(&error),
                }
            }
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
        WsClientRequest::RevokeToken => {
            let Some(token) = session_token else {
                return WsServerMessage::Response(WsServerResponse::error(
                    "REVOKE_FAILED",
                    "当前连接没有可撤销的令牌",
                ));
            };
            // 三态各有不同含义，不能再合流成一个"失败"：
            // `Ok(true)` 撤销成功、`Ok(false)` 令牌本就不存在（客户端视作已撤销），
            // `Err` 才是服务端存储故障（客户端应重试，不能当成已撤销）。
            match state.token_store.revoke(token) {
                Ok(true) => {
                    tracing::info!("[远程控制] 设备主动撤销了当前令牌");
                    WsServerMessage::Response(WsServerResponse::Revoked)
                }
                Ok(false) => WsServerMessage::Response(WsServerResponse::error(
                    "REVOKE_NOT_FOUND",
                    "该令牌本就不存在，可能已被撤销",
                )),
                Err(error) => {
                    tracing::error!(%error, "[远程控制] 撤销令牌失败，令牌仍然有效");
                    WsServerMessage::Response(WsServerResponse::error(
                        "REVOKE_FAILED",
                        "服务端撤销令牌失败，令牌可能仍然有效，请重试",
                    ))
                }
            }
        }

        _ => super::unrouted_request(),
    }
}
