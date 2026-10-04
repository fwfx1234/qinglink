//! QingLink 的领域层：与平台无关的能力契约和设备配对状态机。
//!
//! 这里只定义 `PowerControl` / `ProcessControl` 的接口与配对请求的类型和规则，
//! Windows 实现见 `qinglink-windows`，HTTP 接线见 `qinglink-server`。

/// 平台能力契约：电源控制与进程控制。
pub mod capabilities;
/// 设备配对状态机与请求/结果类型。
pub mod pairing;

pub use capabilities::{CapabilityError, PowerControl, ProcessControl};
pub use pairing::{PairingError, PairingManager, PairingRequest, PairingResult};
