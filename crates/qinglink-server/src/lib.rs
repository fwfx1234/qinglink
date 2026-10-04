//! qinglink 服务端核心库。
//!
//! 汇集远程控制服务端所需的协议定义、平台抽象、应用与 Steam 扫描、
//! 以及 WebSocket 连接与业务服务等模块。

use std::sync::{Mutex, MutexGuard};

/// 自定义扫描目录的配置与持久化管理。
pub mod custom_dir;
/// 平台抽象层，在 Windows 上委托给 Win32 实现，其它平台使用存根。
pub mod platform;
/// 客户端与服务端之间的请求、响应及事件协议定义。
pub mod protocol;
/// WebSocket 服务端、连接注册表与鉴权逻辑。
pub mod server;
/// 远程控制业务服务集合。
pub mod service;
/// Steam 库目录与已安装游戏的扫描。
pub mod steam;

pub use server::{AppState, RemoteServer};
pub use service::RemoteControlService;

/// 锁定互斥锁，并在中毒状态下通过获取内部值来恢复。
pub(crate) fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
