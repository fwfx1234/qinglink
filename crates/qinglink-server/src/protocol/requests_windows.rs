// === Windows ===

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub id: usize,
    pub title: String,
    pub pid: u32,
    pub exe_path: Option<String>,
    pub is_visible: bool,
    pub is_foreground: bool,
    pub is_fullscreen: bool,
    pub is_topmost: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowListResponse {
    pub windows: Vec<WindowInfo>,
    pub active_id: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct FocusWindowRequest {
    pub id: usize,
}

#[derive(Debug, Deserialize)]
pub struct MoveWindowRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct SetAlwaysOnTopRequest {
    pub enable: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowSwitchDirection {
    Next,
    Previous,
}


