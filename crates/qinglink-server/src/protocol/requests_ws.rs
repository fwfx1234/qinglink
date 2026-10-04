// === WebSocket client requests ===

use serde::Deserialize;

use super::{
    WindowSwitchDirection, default_bool_true, default_page, default_page_size,
    default_process_filter, default_process_sort,
};

/// 客户端通过 WebSocket 发送的请求，按 `type` 字段的蛇形命名区分具体指令。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientRequest {
    // === Auth (no auth required) ===
    /// 认证当前连接，是唯一无需预先认证的指令。
    Authenticate {
        /// 认证方式，目前支持 `token`（配对 v2 已改为 HTTP 接口）。
        method: String,
        /// 待校验的设备令牌，`method` 为 `token` 时必需。
        #[serde(default)]
        token: Option<String>,
        /// 旧版配对 PIN 码，仅为兼容老客户端保留。
        #[serde(default)]
        pin: Option<String>,
        /// 客户端设备名，用于在管理端标识该设备。
        #[serde(default)]
        device_name: Option<String>,
    },
    /// 心跳探测，服务端以 `Pong` 应答。
    Ping,
    /// 撤销当前连接所使用的设备令牌。
    RevokeToken,

    // === System ===
    /// 获取 CPU、内存、磁盘等系统总览状态。
    GetSystemStatus,
    /// 获取本机主网卡的 MAC 地址。
    GetMacAddress,
    /// 获取局域网子网前缀长度，用于计算网络唤醒的广播地址。
    GetSubnetPrefix,
    /// 获取支持网络唤醒的网卡列表。
    GetWakeNetworks,
    /// 关闭计算机。
    Shutdown {
        /// 是否强制关闭，跳过应用程序的确认流程。
        #[serde(default)]
        force: bool,
        /// 延迟执行的秒数，0 表示立即执行。
        #[serde(default)]
        delay_secs: u64,
    },
    /// 让计算机进入睡眠或休眠。
    Sleep {
        /// 为 `true` 时进入休眠（状态写入磁盘），否则为普通睡眠。
        #[serde(default)]
        hibernate: bool,
    },
    /// 重启计算机。
    Restart {
        /// 是否强制重启，跳过应用程序的确认流程。
        #[serde(default)]
        force: bool,
        /// 延迟执行的秒数，0 表示立即执行。
        #[serde(default)]
        delay_secs: u64,
    },
    /// 注销当前登录的 Windows 用户。
    Logoff,
    /// 锁定当前 Windows 会话。
    Lock,
    /// 查询锁屏状态以及是否具备自动解锁条件。
    GetUnlockStatus,
    /// 请求自动解锁当前的锁屏会话。
    AutoUnlock {
        /// 可选的 PIN 码，缺省时使用已保存的登录凭据。
        #[serde(default)]
        pin: Option<String>,
    },
    /// 查询安全桌面（锁屏或 UAC 界面）的输入通道是否就绪。
    GetLockInputStatus,
    /// 在锁屏安全桌面上提交 Windows 登录 PIN。
    SubmitLockPin {
        /// 待提交的登录 PIN 明文。
        pin: String,
    },
    /// 查询安全桌面状态：当前桌面、是否锁屏、输入是否可用等。
    GetSecureDesktopStatus,
    /// 发送安全注意序列（Ctrl+Alt+Del）。
    SecureAttention,
    /// 在安全桌面上输入可打印 ASCII 文本。
    SendSecureAscii {
        /// 待输入文本，仅允许 1 到 256 个 ASCII 可打印字符。
        text: String,
    },

    // === Display & remote input ===
    /// 获取显示器列表及其支持的分辨率。
    GetDisplays,
    /// 切换指定显示器的分辨率与刷新率。
    SetDisplayMode {
        /// 目标显示器标识，形如 `\\.\DISPLAY1`。
        display_id: String,
        /// 目标宽度，单位为像素。
        width: u32,
        /// 目标高度，单位为像素。
        height: u32,
        /// 目标刷新率，单位为毫赫兹（60000 表示 60 Hz）。
        refresh_millihz: u32,
    },
    /// 以相对位移移动鼠标指针。
    MouseMove {
        /// 水平位移，服务端会将其限制在允许的最大值内。
        dx: i32,
        /// 垂直位移，服务端会将其限制在允许的最大值内。
        dy: i32,
    },
    /// 把鼠标指针移动到屏幕绝对坐标。
    MouseMoveAbsolute {
        /// 屏幕绝对横坐标，单位为像素。
        x: u32,
        /// 屏幕绝对纵坐标，单位为像素。
        y: u32,
    },
    /// 在指针当前位置单击一次鼠标按键。
    MouseButton {
        /// 按键名，例如 `left`、`right`、`middle`。
        button: String,
    },
    /// 按下或抬起指定鼠标按键，用于实现拖拽。
    MouseButtonState {
        /// 按键名，例如 `left`、`right`、`middle`。
        button: String,
        /// 为 `true` 表示按下，`false` 表示抬起。
        pressed: bool,
    },
    /// 滚动鼠标滚轮。
    MouseWheel {
        /// 滚动量，正值向上滚动，负值向下滚动。
        delta: i32,
    },
    /// 向当前前台窗口输入一段文本。
    SendText {
        /// 待输入文本，长度上限为 64 KiB。
        text: String,
    },
    /// 触发一次具名按键，例如 `backspace`、`enter`。
    KeyPress {
        /// 受支持的按键名。
        key: String,
    },
    /// 发送原始 HID 键盘事件，用于组合键与长按场景。
    KeyEvent {
        /// HID 用法码，有效范围为 0x04 到 0x73。
        usage: u8,
        /// 修饰键位掩码，表示 Ctrl、Shift、Alt、Win 的组合。
        #[serde(default)]
        modifiers: u8,
        /// 为 `true` 表示按下，`false` 表示抬起。
        pressed: bool,
    },

    // === Processes ===
    /// 分页查询进程列表。
    GetProcesses {
        /// 按进程名过滤的关键字，缺省表示不过滤。
        #[serde(default)]
        search: Option<String>,
        /// 页码，从 0 开始，默认 0。
        #[serde(default = "default_page")]
        page: usize,
        /// 每页条数，默认 50，服务端最多返回 50 条。
        #[serde(default = "default_page_size")]
        page_size: usize,
        /// 过滤范围，默认 `user` 表示仅返回用户进程。
        #[serde(default = "default_process_filter")]
        filter: String,
        /// 排序方式，默认 `active` 表示按活跃度排序。
        #[serde(default = "default_process_sort")]
        sort: String,
    },
    /// 获取当前前台窗口所属的进程信息。
    GetForeground,
    /// 结束指定进程。
    KillProcess {
        /// 目标进程 ID。
        pid: u32,
    },
    /// 挂起指定进程。
    SuspendProcess {
        /// 目标进程 ID。
        pid: u32,
    },
    /// 恢复被挂起的进程。
    ResumeProcess {
        /// 目标进程 ID。
        pid: u32,
    },

    // === Apps ===
    /// 按关键字搜索已扫描到的应用。
    SearchApps {
        /// 搜索关键字。
        query: String,
    },
    /// 启动指定的可执行文件。
    LaunchApp {
        /// 可执行文件的绝对路径。
        path: String,
        /// 启动参数列表，默认为空。
        #[serde(default)]
        args: Vec<String>,
    },

    // === Scanner ===
    /// 分页查询启动器条目（按类别筛选）。
    ListLauncherItems {
        /// 条目类别：`game`/`app`/`favorite`/`steam`/`software`/`other`；其它取值不筛选。
        kind: String,
        /// 可选的名称过滤关键字。
        #[serde(default)]
        query: Option<String>,
        /// 页码，从 0 开始，默认 0。
        #[serde(default = "default_page")]
        page: usize,
        /// 每页条数，默认 50。
        #[serde(default = "default_page_size")]
        page_size: usize,
    },
    /// 获取最近启动过的启动器条目。
    GetRecentLauncherItems {
        /// 条目类别，取值为 `game` 或 `app`。
        kind: String,
    },
    /// 获取指定启动器条目的图标数据。
    GetLauncherIcon {
        /// 启动器条目 ID。
        id: String,
    },
    /// 重新解析指定条目的图标。
    RefreshScannerAppIcon {
        /// 启动器条目 ID。
        id: String,
        /// Optional explicit icon file. `None` re-runs the automatic chain
        /// (shortcut icon → executable resource → sibling artwork → shell).
        #[serde(default)]
        icon_path: Option<String>,
    },
    /// List the icon files a user could pick for one entry.
    ListScannerAppIcons {
        /// 启动器条目 ID。
        id: String,
    },
    /// Persist an explicit icon file (`None` clears the override) and re-resolve.
    SetScannerAppIcon {
        /// 启动器条目 ID。
        id: String,
        /// 显式指定的图标文件路径，`None` 表示清除覆盖并恢复自动解析。
        #[serde(default)]
        icon_path: Option<String>,
    },
    /// List the entries the user blocked, so they can be restored.
    ListBlockedScannerApps,
    /// 手动触发一次启动器目录扫描。
    RefreshLauncherCatalog {
        /// `app` and `game` are independent manual refreshes.  Older clients
        /// omit this field and therefore refresh software only.
        #[serde(default)]
        kind: String,
    },
    /// 获取应用扫描结果（旧版接口）。
    GetScannerApps,
    /// 强制重新扫描并同步应用目录（旧版接口）。
    RefreshScannerApps,
    /// 重命名某个已扫描到的应用。
    RenameScannerApp {
        /// 启动器条目 ID。
        id: String,
        /// 条目的原始名称，用于同步内存目录。
        original_name: String,
        /// 新的显示名称。
        new_name: String,
    },
    /// 按 ID 启动启动器条目，支持 Steam 条目。
    LaunchScannerApp {
        /// 启动器条目 ID。
        id: String,
    },
    /// 扫描目录中的可执行文件，返回候选列表供用户确认。
    ScanGameDirectory {
        /// 待扫描的目录路径。
        path: String,
        /// `game` (default) or `app`; decides the category of the entries that
        /// the user later confirms.
        #[serde(default)]
        kind: String,
    },
    /// 把用户在扫描候选中确认的条目加入目录。
    AddScannedGames {
        /// 本次扫描的根目录，用于校验候选文件确实位于其中。
        path: String,
        /// 用户勾选的可执行文件路径列表。
        exe_paths: Vec<String>,
        /// Optional display names parallel to `exe_paths` (mobile clients).
        #[serde(default)]
        names: Vec<String>,
        /// 条目类别，`game` 为默认值，`app` 表示按实用工具归入应用。
        #[serde(default)]
        kind: String,
    },

    // === Steam ===
    /// 获取已安装的 Steam 游戏及库信息。
    GetSteamGames,
    /// 重新扫描 Steam 库并同步到本地目录。
    RefreshSteamGames,
    /// 触发指定 Steam 游戏的安装。
    InstallSteamGame {
        /// Steam 应用 ID。
        app_id: u32,
    },
    /// 启动指定的 Steam 游戏。
    LaunchSteamGame {
        /// Steam 应用 ID。
        app_id: u32,
    },

    // === Custom Scan Dirs ===
    /// 获取自定义扫描目录列表。
    GetCustomDirs,
    /// 新增一个自定义扫描目录。
    AddCustomDir {
        /// 目录的绝对路径。
        path: String,
        /// 显示名称，缺省时使用目录名。
        #[serde(default)]
        name: Option<String>,
        /// 是否启用该目录的扫描，默认为启用。
        #[serde(default = "default_bool_true")]
        enabled: bool,
        /// 最大递归深度，0 表示不限制。
        #[serde(default)]
        max_depth: u32,
        /// 限定扫描的文件扩展名，缺省表示使用默认集合。
        #[serde(default)]
        extensions: Option<Vec<String>>,
        /// 是否递归扫描子目录，默认为递归。
        #[serde(default = "default_bool_true")]
        recursive: bool,
    },
    /// 更新自定义扫描目录，未提供的字段保持原值。
    UpdateCustomDir {
        /// 目标目录 ID。
        id: String,
        /// 新的目录路径，`None` 表示不修改。
        #[serde(default)]
        path: Option<String>,
        /// 新的显示名称，`None` 表示不修改。
        #[serde(default)]
        name: Option<String>,
        /// 新的启用状态，`None` 表示不修改。
        #[serde(default)]
        enabled: Option<bool>,
        /// 新的最大递归深度，`None` 表示不修改。
        #[serde(default)]
        max_depth: Option<u32>,
        /// 新的扩展名过滤集合，`None` 表示不修改。
        #[serde(default)]
        extensions: Option<Vec<String>>,
        /// 新的递归开关，`None` 表示不修改。
        #[serde(default)]
        recursive: Option<bool>,
    },
    /// 删除指定的自定义扫描目录。
    DeleteCustomDir {
        /// 目标目录 ID。
        id: String,
    },
    /// 校验目录是否存在、可读，并统计其中的可执行文件数量。
    ValidateDir {
        /// 待校验的目录路径。
        path: String,
    },

    // === Tasks ===
    /// 分页查询任务（进程）列表。
    GetTasks {
        /// 按进程名过滤的关键字，缺省表示不过滤。
        #[serde(default)]
        search: Option<String>,
        /// 页码，从 0 开始，默认 0。
        #[serde(default = "default_page")]
        page: usize,
        /// 每页条数，默认 50。
        #[serde(default = "default_page_size")]
        page_size: usize,
        /// 过滤范围，默认 `user` 表示仅返回用户进程。
        #[serde(default = "default_process_filter")]
        filter: String,
    },
    /// 获取系统资源占用统计。
    GetTasksStats,
    /// 结束指定的任务。
    KillTask {
        /// 目标进程 ID。
        pid: u32,
    },
    /// 设置指定任务的优先级。
    SetTaskPriority {
        /// 目标进程 ID。
        pid: u32,
        /// 目标优先级名称，例如 `normal`、`high`。
        priority: String,
    },

    // === Volume & Media ===
    /// 获取当前音量与静音状态。
    GetVolume,
    /// 获取音频输出设备列表。
    GetAudioOutputs,
    /// 切换默认的音频输出设备。
    SetAudioOutput {
        /// 目标音频输出设备 ID。
        device_id: String,
    },
    /// 设置系统主音量。
    SetVolume {
        /// 音量百分比，取值范围 0 到 100。
        percent: u8,
    },
    /// 设置系统静音状态。
    SetMuted {
        /// 为 `true` 表示静音。
        muted: bool,
    },
    /// 调高一级系统音量。
    VolumeUp,
    /// 调低一级系统音量。
    VolumeDown,
    /// 切换系统静音状态。
    VolumeMute,
    /// 播放或暂停当前媒体。
    MediaPlayPause,
    /// 停止当前媒体播放。
    MediaStop,
    /// 切换到下一个媒体曲目。
    MediaNext,
    /// 切换到上一个媒体曲目。
    MediaPrev,

    // === File System ===
    /// 浏览文件系统，未提供路径时返回驱动器列表。
    BrowseFiles {
        /// 目标目录路径，`None` 表示列出所有驱动器。
        #[serde(default)]
        path: Option<String>,
    },
    /// 获取文件浏览器的快速访问项。
    GetQuickAccess,

    // === Window Control ===
    /// 获取当前打开的窗口列表。
    ListWindows {
        /// 为 `true` 时忽略缓存，强制重新枚举窗口。
        #[serde(default)]
        force_refresh: bool,
    },
    /// 把指定窗口切换到前台。
    FocusWindow {
        /// 目标窗口句柄。
        hwnd: usize,
    },
    /// 按方向切换到上一个或下一个窗口。
    SwitchWindow {
        /// 相对当前窗口的切换方向。
        direction: WindowSwitchDirection,
    },
    /// 最小化指定窗口。
    MinimizeWindow {
        /// 目标窗口句柄。
        hwnd: usize,
    },
    /// 最大化指定窗口。
    MaximizeWindow {
        /// 目标窗口句柄。
        hwnd: usize,
    },
    /// 把指定窗口恢复为普通显示状态。
    RestoreWindow {
        /// 目标窗口句柄。
        hwnd: usize,
    },
    /// 关闭指定窗口。
    CloseWindow {
        /// 目标窗口句柄。
        hwnd: usize,
    },
    /// 最小化当前前台窗口。
    MinimizeForeground,
    /// 最大化当前前台窗口。
    MaximizeForeground,
    /// 关闭当前前台窗口。
    CloseForeground,
    // === Scanner App Management ===
    /// 手动把一个应用加入启动器目录。
    AddScannerApp {
        /// 应用显示名称。
        name: String,
        /// 可执行文件路径。
        exe_path: String,
        /// 应用分类，例如 `Application` 或 `Game`。
        category: String,
        /// 客户端显式提供的 Base64 图标，缺省时由服务端解析。
        icon_base64: Option<String>,
        /// 条目来源，缺省时记为 `User`。
        #[serde(default)]
        source: Option<String>,
        /// 启动参数列表，默认为空。
        #[serde(default)]
        args: Vec<String>,
    },
    /// 更新用户添加的应用条目。
    UpdateScannerApp {
        /// 目标条目 ID。
        id: String,
        /// 新的显示名称。
        name: String,
        /// 新的可执行文件路径。
        exe_path: String,
        /// 新的 Base64 图标，`None` 表示保留原图标。
        icon_base64: Option<String>,
        /// 新的启动参数，`None` 表示保留原参数。
        #[serde(default)]
        args: Option<Vec<String>>,
    },
    /// 从目录中删除一个应用条目。
    DeleteScannerApp {
        /// 目标条目 ID。
        id: String,
    },
    /// 屏蔽或恢复显示一个应用条目。
    SetScannerAppHidden {
        /// 目标条目 ID。
        id: String,
        /// 为 `true` 表示屏蔽该条目。
        hidden: bool,
    },
    /// 加入 / 移出"常用"（我的库）。Stored on the service so it follows the PC.
    SetScannerAppFavorite {
        /// 目标条目 ID。
        id: String,
        /// 为 `true` 表示加入常用，`false` 表示移出。
        favorite: bool,
    },
    /// 置顶 / 取消置顶到手机首页。置顶会自动加入常用。
    SetScannerAppPinned {
        /// 目标条目 ID。
        id: String,
        /// 为 `true` 表示置顶，`false` 表示取消置顶。
        pinned: bool,
    },
}

/// 序列化测试独立成文件，避免本文件超出 AGENTS.md §2.1 的规模上限。
#[cfg(test)]
#[path = "requests_ws_test.rs"]
mod requests_ws_test;
