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

/// 上行请求帧的序列化测试。挂在本模块而不是 `requests_ws.rs`，以免那个受
/// AGENTS.md §2.3 约束的文件继续变长（测试内部一律用绝对路径引用被测类型）。
#[cfg(test)]
#[path = "requests_ws_test.rs"]
mod requests_ws_test;
