//! 能力协商：服务端声明自己支持哪些命令组。

use serde::{Deserialize, Serialize};

/// Negotiated capabilities. Every field defaults to `false` so that a newer
/// desktop talking to an older service degrades to "unsupported" rather than
/// failing to decode the response.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Capabilities {
    /// 是否支持管理类命令。
    #[serde(default)]
    pub management: bool,
    /// 是否支持配对相关命令。
    #[serde(default)]
    pub pairing: bool,
    /// 是否支持暂停 / 恢复。
    #[serde(default)]
    pub pause: bool,
    /// 是否支持设备列表与吊销。
    #[serde(default)]
    pub devices: bool,
    /// 是否支持目录读取。
    #[serde(default)]
    pub catalog: bool,
    /// `ListApps` / `GetApp` / `RenameApp` / `SetAppHidden` / `DeleteApp` /
    /// `RescanApps` / `GetAppIcon` / `GetCatalogStats`.
    #[serde(default)]
    pub catalog_management: bool,
    /// `AddApp`.
    #[serde(default)]
    pub app_add: bool,
    /// `LaunchApp`.
    #[serde(default)]
    pub app_launch: bool,
    /// `RefreshAppIcon` / `ListAppIcons`.
    #[serde(default)]
    pub app_icon_manage: bool,
    /// `SetKeepAwake` 与快照里的 `keep_awake` 字段。
    #[serde(default)]
    pub keep_awake: bool,
}
