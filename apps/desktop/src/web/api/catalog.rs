//! 应用目录管理。
//!
//! 这里的每次修改都转发给持有目录的服务。正因如此，浏览器里的重命名对这台 PC 的
//! 每个移动端客户端都立即生效，数据也跟随这台 PC 而不是控制端设备。
//!
//! 实现按职责拆到同目录子模块，本文件只做模块声明与公开路径再导出；
//! `web/api/mod.rs` 里的路由注册（`catalog::list` 等）因此无需改动：
//!
//! * [`listing`] — 目录列表与统计查询；
//! * [`icons`] — 图标读取、候选列表、刷新与设置；
//! * [`entries`] — 单条目编辑：重命名、隐藏、常用、置顶、删除；
//! * [`scan`] — 触发扫描与手动新增；
//! * [`launch`] — 在本机启动条目；
//! * [`batch`] — 批量操作；
//! * [`support`] — 各子模块共用的上限常量与入参校验。

mod batch;
mod entries;
mod icons;
mod launch;
mod listing;
mod scan;
mod support;

// 以下再导出是模块的公开接口，而不是本文件的局部快捷方式：路由只用到 handler
// 函数，请求结构体则由各子模块自己消费，所以这里显式允许“未在本文件使用”。
#[allow(unused_imports)]
pub use batch::{BatchAction, BatchRequest, batch};
#[allow(unused_imports)]
pub use entries::{
    DeleteRequest, FavoriteRequest, HiddenRequest, PinnedRequest, RenameRequest, delete_app,
    rename, set_favorite, set_hidden, set_pinned,
};
#[allow(unused_imports)]
pub use icons::{
    IconCandidatesQuery, IconQuery, IconRefreshRequest, IconSetRequest, icon, icon_candidates,
    refresh_icon, set_icon,
};
#[allow(unused_imports)]
pub use launch::{LaunchRequest, launch};
#[allow(unused_imports)]
pub use listing::{AppsQuery, list, stats};
#[allow(unused_imports)]
pub use scan::{AddRequest, ScanRequest, add, scan};
