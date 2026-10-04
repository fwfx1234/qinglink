use serde::{Deserialize, Serialize};

include!("requests_auth_system.rs");
include!("requests_process_apps.rs");
include!("requests_custom_dirs.rs");
include!("requests_settings.rs");
include!("requests_windows.rs");
include!("requests_events.rs");

/// 客户端上行请求枚举（`WsClientRequest`）所在的子模块。
#[path = "requests_ws.rs"]
mod requests_ws;

pub use crate::protocol::responses_ws::{WsServerMessage, WsServerResponse};
pub use requests_ws::WsClientRequest;
