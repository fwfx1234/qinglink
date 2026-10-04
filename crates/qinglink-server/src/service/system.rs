//! 系统服务：采集 CPU / 内存 / 运行时间，枚举网卡，并执行电源与会话控制。
//!
//! 本模块是门面：只声明子模块并把公开符号还原到原路径，实现按职责分散到同目录文件：
//!
//! - `status`：[`SystemService`] 的定义与 CPU / 内存 / 运行时间采集；
//! - `network`：唤醒网卡枚举、物理 MAC 地址与子网前缀查询；
//! - `power`：关机、重启与睡眠 / 休眠；
//! - `session`：注销、锁定、解锁状态查询、自动解锁与免密码唤醒配置。

mod network;
mod power;
mod session;
mod status;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;

pub use status::SystemService;
