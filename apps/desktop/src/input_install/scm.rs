//! Windows 服务控制管理器（SCM）交互。
//!
//! 服务安装/卸载、启停、状态查询与等待，以及服务对象 ACL（SDDL）收紧都经由本模块；
//! 桌面端与提权助手共用同一套原语，因此提权参数、控制码、等待间隔与 SDDL 只有
//! 一处定义。
//!
//! 实现按职责拆到同目录子模块，本文件只做模块声明与公开路径再导出，
//! `crate::input_install::scm::<符号>` 的路径与签名保持不变：
//!
//! * [`ffi`] — advapi32/kernel32 原始绑定、访问位常量、`#[repr(C)]` 结构与句柄 RAII；
//! * [`query`] — SCM/服务句柄打开与查询原语（存在性、当前状态、镜像路径）；
//! * [`status`] — 平台无关的状态查询门面与按固定间隔等待收敛；
//! * [`control`] — 启动/停止，以及启动、停止后的等待；
//! * [`manage`] — 服务创建/更新与删除，即安装、卸载流程；
//! * [`permissions`] — 服务对象 DACL（SDDL）收紧。

#[cfg(target_os = "windows")]
mod ffi;
#[cfg(target_os = "windows")]
mod query;

mod control;
mod manage;
mod permissions;
mod status;

pub(crate) use control::{start_and_wait, stop_and_wait};
pub(crate) use manage::{delete_service, install_or_configure};
pub(crate) use permissions::configure_service_permissions;
pub(crate) use status::{
    service_exists, service_image_path_matches, service_is_running, wait_for_running,
    wait_for_stopped,
};
