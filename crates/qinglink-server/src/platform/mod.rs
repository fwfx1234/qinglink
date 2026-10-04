//! 平台抽象层。
//!
//! 在 Windows 上，委托给 Win32 实现。
//! 在其他平台上，使用返回"不支持"错误的存根。
//!
//! 按职责拆分：进程操作在 `process`，电源与会话在 `power`，应用启动与图标在 `app`，
//! 窗口管理在 `window`，前台窗口与钩子在 `foreground`，音量与媒体在 `volume_media`，
//! 解锁与安全桌面在 `secure_desktop`，显示在 `display`，键鼠输入在 `input`。
//! 各子模块的公开项在此统一再导出，`crate::platform::<fn>()` 的调用路径保持不变。

/// 将带参数的平台函数分派给 Windows 或存根实现。
///
/// 调用时可在函数名前传入文档注释，该注释会附加到生成的函数上。
macro_rules! platform_fn_args {
    ($(#[$meta:meta])* $name:ident, $ret:ty, $($arg:ident : $arg_ty:ty),*) => {
        $(#[$meta])*
        #[cfg(target_os = "windows")]
        pub fn $name($($arg: $arg_ty),*) -> $ret {
            windows::$name($($arg),*)
        }
        $(#[$meta])*
        #[cfg(not(target_os = "windows"))]
        pub fn $name($($arg: $arg_ty),*) -> $ret {
            stub::$name($($arg),*)
        }
    };
}

mod app;
mod display;
mod foreground;
mod input;
mod power;
mod process;
mod secure_desktop;
mod volume_media;
mod window;

pub use app::*;
pub use display::*;
pub use foreground::*;
pub use input::*;
pub use power::*;
pub use process::*;
pub use secure_desktop::*;
pub use volume_media::*;
pub use window::*;

// === Platform-specific implementations ===

#[cfg(target_os = "windows")]
mod platform_impl {
    pub use super::windows::*;

    use crate::protocol::responses::ProcessInfo;
    use sysinfo::Process;

    pub fn get_process_info(process: &Process) -> ProcessInfo {
        ProcessInfo {
            pid: process.pid().as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            memory_bytes: process.memory(),
            cpu_usage: process.cpu_usage(),
            path: process.exe().map(|p| p.to_string_lossy().into_owned()),
            started_at: process.start_time(),
            group: String::new(),
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform_impl {
    pub use super::stub::*;

    use crate::protocol::responses::ProcessInfo;
    use sysinfo::Process;

    pub fn get_process_info(process: &Process) -> ProcessInfo {
        ProcessInfo {
            pid: process.pid().as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            memory_bytes: process.memory(),
            cpu_usage: process.cpu_usage(),
            path: process.exe().map(|p| p.to_string_lossy().into_owned()),
            started_at: process.start_time(),
            group: String::new(),
        }
    }
}

#[cfg(target_os = "windows")]
pub use qinglink_windows::platform_windows as windows;

#[cfg(not(target_os = "windows"))]
mod stub;
