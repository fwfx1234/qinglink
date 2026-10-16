//! 远程控制服务端：共享状态、连接注册表与 HTTP/WebSocket 接入层。
//!
//! 本文件只做模块声明与重导出，实现按职责分文件：
//! `state.rs` 定义并构造 `AppState`，`state/input_gate.rs` 负责输入门控与保留
//! 输入，`window_cache.rs` 负责窗口列表缓存，`process_icon.rs` 负责进程图标缓存，
//! `catalog.rs` 负责目录扫描与广播，`lifecycle.rs` 负责服务器装配与启动。

pub mod auth;
pub mod connection_registry;
pub mod control;
pub mod held_input;
pub mod icon_service;
pub mod input_protocol;
pub mod input_ws_handler;
pub mod pairing;
pub mod ws_common;
pub mod ws_handler;
pub mod ws_request_handler;

mod catalog;
mod launch_display;
mod lifecycle;
mod process_icon;
mod state;
mod window_cache;

pub use auth::TokenStore;
pub use catalog::{normalize_catalog_kind, reload_visible_catalog, scan_catalog_kind};
pub use launch_display::LaunchDisplayCoordinator;
pub use lifecycle::RemoteServer;
pub use state::{AppState, EventSender};
