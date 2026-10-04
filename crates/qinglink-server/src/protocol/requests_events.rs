// === WebSocket events ===

/// 服务端通过 WebSocket 主动推送的事件。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// 前台窗口发生变化。
    ForegroundChanged {
        /// 新的前台窗口信息。
        data: ForegroundInfo,
    },
    /// 有进程启动。
    ProcessStarted {
        /// 新启动进程的基本信息。
        data: ProcessBasic,
    },
    /// 有进程结束。
    ProcessEnded {
        /// 已结束进程的基本信息。
        data: ProcessBasic,
    },
    /// 系统即将挂起（睡眠或休眠）。
    SystemSuspend,
    /// 系统已从挂起状态恢复。
    SystemResume,
    /// 系统正在关机。
    SystemShutdown {
        /// 触发关机的原因描述。
        reason: String,
    },
    /// 系统正在进入睡眠。
    SystemSleep,
    /// 启动器目录扫描完成。
    LauncherCatalogReady {
        /// 完成扫描的目录类别标识。
        kind: String,
    },
}

/// 前台窗口的详细信息。
#[derive(Debug, Clone, Serialize)]
pub struct ForegroundInfo {
    /// 前台窗口所属进程的 PID。
    pub pid: u32,
    /// 前台窗口标题。
    pub title: String,
    /// 前台窗口所属进程的可执行文件路径。
    pub path: Option<String>,
    /// 前台窗口句柄（HWND）的数值形式。
    pub hwnd: usize,
}

/// 进程的基本标识信息。
#[derive(Debug, Clone, Serialize)]
pub struct ProcessBasic {
    /// 进程 PID。
    pub pid: u32,
    /// 进程名。
    pub name: String,
}

