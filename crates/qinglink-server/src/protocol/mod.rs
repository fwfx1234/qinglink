//! WebSocket 协议帧定义：请求帧由客户端上行，响应帧由服务端下行。

/// 客户端上行请求帧的结构定义。
pub mod requests;
/// 服务端下行响应帧的结构定义。
pub mod responses;
/// 服务端下行的 WebSocket 响应词汇（`WsServerResponse` / `WsServerMessage`），
/// 由 `requests` 模块按历史路径重新导出。
pub(crate) mod responses_ws;
