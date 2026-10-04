//! 系统托盘图标，直接实现在 Windows shell API 上。
//!
//! # 为什么不用第三方托盘 crate
//!
//! 之前的实现委托给 `tray-icon`。它的注册路径让"图标不出现"难以诊断，因为失败的
//! `Shell_NotifyIcon(NIM_ADD)` 实际上被当成瞬时状况处理，没有把结果暴露给应用。
//!
//! 本模块自己持有整条链路：一个隐藏的顶层窗口、一次显式检查的
//! `Shell_NotifyIcon(NIM_ADD)`，以及通知区域尚未就绪时的有界重试。每个可能失败的
//! 步骤都会记下原因，[`TrayController::registration`] 把它暴露给 `--self-check`
//! 与 Web 界面。
//!
//! # 线程模型
//!
//! 按 Win32 约定，窗口、菜单与图标都在**同一条**线程上创建、收发消息与销毁——即
//! 进程主线程，它每 50 ms 也会泵一次消息。因此控制器存活于 `main` 的全程且从不
//! 移动。属主窗口虽然隐藏，但仍是顶层窗口，所以 Explorer 在 shell 重启后可以向
//! 它广播 `TaskbarCreated`。
//!
//! # 模块划分
//!
//! 本模块只做声明与重导出，实现按职责分在下面几个子模块里，公开路径保持
//! `crate::tray::<符号>` 不变：
//!
//! - `controller`：[`TrayController`] 的生命周期，只决定"什么时候做什么"；
//! - `shell`：`NOTIFYICONDATAW` 组装、`Shell_NotifyIcon` 调用与失败判读、重试预算；
//! - `window`：隐藏消息窗口、窗口过程、事件分发与右键菜单弹出；
//! - `report`：状态与注册结果模型、[`blocked_reason`] 分类与提示文本；
//! - [`menu`]、[`icon`]：菜单定义与图标光栅化。

pub mod icon;
pub mod menu;

mod controller;
mod report;
mod shell;
mod window;

pub use menu::{TrayAction, TrayMenu};

pub use controller::TrayController;
pub use report::{RegistrationReport, TrayRect, TrayStatus, blocked_reason, build_tooltip};

pub(crate) use controller::TrayRetryResult;
pub(crate) use window::TrayEvent;
