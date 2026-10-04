//! 在同一个已授权的 legacy/v2 端点上提供类型化的桌面管理。
//!
//! 这是桌面管理界面修改业务数据的**唯一**路径。把它放在这里意味着持久化的目录只有
//! 一个写入者（服务），因此浏览器里做的重命名会立即对连到这台 PC 的每个移动端
//! 客户端可见。
//!
//! 本模块按职责拆分：命令分发与快照（`command`、`snapshot`）、设备管理
//! （`devices`）、配对转发（`pairing`）、目录读取（`catalog`）、目录写操作
//! （`catalog_edit`）、图标（`icons`）、应用增删与启动（`apps`）、条目查找与
//! 校验（`lookup`）、结果编码与分页（`record`、`page`）。

mod apps;
mod catalog;
mod catalog_edit;
mod command;
mod devices;
mod icons;
mod lookup;
mod page;
mod pairing;
mod record;
mod snapshot;

pub(super) use command::dispatch;
