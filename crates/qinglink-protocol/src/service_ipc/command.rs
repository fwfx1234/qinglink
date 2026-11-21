//! 本地管理 IPC 的命令枚举：每条命令及其专属参数。

use serde::{Deserialize, Serialize};

/// Desktop management commands share the versioned local transport.
///
/// Every command here is answered with a JSON `payload`. Commands added after
/// version 2 are gated by [`Capabilities`] so an older service simply reports
/// them as unsupported instead of failing to decode the request.
///
/// [`Capabilities`]: super::Capabilities
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "command", content = "payload", rename_all = "snake_case")]
pub enum DesktopCommand {
    /// 查询服务端支持的命令能力。
    Capabilities,
    /// 读取服务运行快照。
    GetSnapshot {
        /// 桌面当前是否已解锁，由调用方提供。
        desktop_unlocked: bool,
    },
    /// 暂停或恢复服务。
    SetPaused {
        /// 是否暂停服务。
        paused: bool,
    },
    /// 开启或关闭保持唤醒（阻止系统自动休眠与显示器熄屏）。
    ///
    /// 开关由服务进程统一持有：桌面端托盘、管理界面设置页与移动端控制中心
    /// 都读写这一份状态，避免两个进程各持一个 `SetThreadExecutionState` 请求。
    SetKeepAwake {
        /// 是否开启。
        enabled: bool,
    },
    /// 取消一次待确认的配对。
    CancelPairing {
        /// 要取消的配对请求标识。
        pairing_id: String,
    },
    /// 分页列出已配对设备。
    ListDevices {
        /// 起始偏移量。
        offset: usize,
        /// 最多返回的条数。
        limit: usize,
    },
    /// 吊销一台已配对设备。
    RevokeDevice {
        /// 要吊销的设备标识。
        device_id: String,
    },
    /// 分页读取目录（应用 / 游戏清单）。
    GetCatalog {
        /// 起始偏移量。
        offset: usize,
        /// 最多返回的条数。
        limit: usize,
    },
    /// Admin catalog listing. Unlike `GetCatalog` this includes hidden entries
    /// and carries every field the management UI needs.
    ListApps {
        /// 目录类别（`app` / `game`）。
        kind: String,
        /// 起始偏移量。
        offset: usize,
        /// 最多返回的条数。
        limit: usize,
        /// 可选的名称过滤关键字。
        #[serde(default)]
        query: Option<String>,
        /// 是否包含已隐藏的条目。
        #[serde(default = "default_true")]
        include_hidden: bool,
        /// Only entries in 常用（"我的库"）.
        #[serde(default)]
        favorite_only: bool,
    },
    /// 读取单个目录条目。
    GetApp {
        /// 条目标识，可能是 Windows 路径。
        id: String,
    },
    /// Renaming writes `scanner_apps.display_name` on the service, so the new
    /// name is authoritative for every client that connects to this PC.
    /// An empty `name` clears the override and restores `original_name`.
    RenameApp {
        /// 要重命名的条目标识。
        id: String,
        /// 新的显示名称；为空表示清除覆盖并恢复扫描名称。
        name: String,
    },
    /// 设置条目是否对客户端隐藏。
    SetAppHidden {
        /// 要修改的条目标识。
        id: String,
        /// 是否隐藏该条目。
        hidden: bool,
    },
    /// 加入 / 移出"常用"（我的库）。Curated on the service so the phone's list
    /// follows the PC instead of the device.
    SetAppFavorite {
        /// 要修改的条目标识。
        id: String,
        /// 是否加入常用。
        favorite: bool,
    },
    /// 置顶 / 取消置顶到手机首页。置顶会自动加入常用。
    SetAppPinned {
        /// 要修改的条目标识。
        id: String,
        /// 是否置顶。
        pinned: bool,
    },
    /// 删除一个目录条目。
    DeleteApp {
        /// 要删除的条目标识。
        id: String,
    },
    /// 重新扫描指定类别的目录。
    RescanApps {
        /// 要扫描的类别（`app` / `game`）。
        kind: String,
    },
    /// 读取单个条目的图标。
    GetAppIcon {
        /// 条目标识。
        id: String,
    },
    /// Re-resolve one entry's icon and persist the result.
    ///
    /// `icon_path` selects an explicit file; `None` re-runs automatic
    /// discovery (shortcut icon → executable resource → sibling artwork).
    RefreshAppIcon {
        /// 要刷新图标的条目标识。
        id: String,
        /// 指定的图标文件路径；`None` 表示重新自动发现。
        #[serde(default)]
        icon_path: Option<String>,
    },
    /// Icon files the user could pick from for one entry.
    ListAppIcons {
        /// 条目标识。
        id: String,
    },
    /// 读取各类别的目录统计。
    GetCatalogStats,
    /// Add a user-supplied catalog entry.
    ///
    /// `kind` is normalized on the service; `args` are the launch arguments of
    /// the entry and are stored verbatim after control characters are removed.
    AddApp {
        /// 新条目的可执行文件路径。
        path: String,
        /// 新条目的显示名称。
        name: String,
        /// 条目类别，服务端会做归一化。
        kind: String,
        /// 启动参数，去除控制字符后原样保存。
        #[serde(default)]
        args: Vec<String>,
    },
    /// Launch an existing catalog entry in the interactive user session.
    LaunchApp {
        /// 要启动的条目标识。
        id: String,
    },
}

/// `ListApps::include_hidden` 的 serde 默认值。
const fn default_true() -> bool {
    true
}
