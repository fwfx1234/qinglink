// === Windows ===

/// 单个窗口的快照信息。
#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    /// 窗口句柄（HWND）的数值形式。
    pub id: usize,
    /// 窗口标题。
    pub title: String,
    /// 窗口所属进程的 PID。
    pub pid: u32,
    /// 窗口所属进程的可执行文件路径。
    pub exe_path: Option<String>,
    /// 窗口当前是否可见。
    pub is_visible: bool,
    /// 窗口是否为当前前台窗口。
    pub is_foreground: bool,
    /// 窗口是否处于全屏状态。
    pub is_fullscreen: bool,
    /// 窗口是否置顶显示。
    pub is_topmost: bool,
}

/// 窗口切换的方向。
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowSwitchDirection {
    /// 切换到下一个窗口。
    Next,
    /// 切换到上一个窗口。
    Previous,
}

