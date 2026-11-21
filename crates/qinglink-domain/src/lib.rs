//! QingLink 的领域层：与平台无关的设备配对状态机。
//!
//! 这里只定义配对请求的类型与规则，Windows 实现见 `qinglink-windows`，
//! HTTP 接线见 `qinglink-server`。

/// 设备配对状态机与请求/结果类型。
pub mod pairing;

pub use pairing::{PairingError, PairingManager, PairingRequest, PairingResult};
