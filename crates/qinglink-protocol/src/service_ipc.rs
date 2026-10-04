//! 桌面服务本地管理 IPC 的协议类型：命令、能力协商、应答与事件。
//!
//! 本文件只做子模块声明与重导出，公开路径保持为
//! `qinglink_protocol::service_ipc::<符号>`。实现按职责拆在同目录子模块中：
//!
//! - `envelope`：版本常量、请求 / 应答 / 事件信封与分页包装；
//! - `command`：命令枚举及其专属参数；
//! - `capabilities`：能力协商位；
//! - `snapshot`：服务运行快照与扫描进度；
//! - `device`：设备摘要与待确认配对载荷；
//! - `catalog`：目录条目、管理行、策展标记与统计；
//! - `icon`：图标数据、刷新结果与候选文件载荷；
//! - `icon_budget`：图标体积预算与校验函数。

mod capabilities;
mod catalog;
mod command;
mod device;
mod envelope;
mod icon;
mod icon_budget;
mod snapshot;

pub use capabilities::Capabilities;
pub use catalog::{CatalogApp, CatalogEntry, CatalogFlags, CatalogKindStats, CatalogStats};
pub use command::DesktopCommand;
pub use device::{DeviceSummary, PendingPairing};
pub use envelope::{
    DesktopRequest, MAX_PAGE_SIZE, Page, SERVICE_IPC_VERSION, ServiceEvent, ServiceResponse,
};
pub use icon::{AppIcon, AppIconCandidate, AppIconCandidates, AppIconUpdate};
pub use icon_budget::{MAX_ICON_BYTES, icon_fits_budget};
pub use snapshot::{CatalogScanState, ServiceSnapshot};
