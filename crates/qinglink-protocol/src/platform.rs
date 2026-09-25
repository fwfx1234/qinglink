use serde::{Deserialize, Serialize};

pub const MAX_MOUSE_MOVE_DELTA: i32 = 500;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowSwitchDirection {
    Next,
    Previous,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputsResponse {
    pub devices: Vec<AudioOutputDevice>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeStatus {
    pub percent: u8,
    pub muted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnlockStatus {
    pub locked: bool,
    pub mode: String,
    pub auto_login_enabled: bool,
    pub message: String,
    #[serde(default)]
    pub uac_detected: bool,
    #[serde(default)]
    pub surface: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputStatus {
    pub available: bool,
    pub service_running: bool,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputResult {
    pub code: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecureDesktopStatus {
    pub desktop: String,
    pub locked: bool,
    pub input_ready: bool,
    pub sas_supported: bool,
    pub session_id: u32,
    pub agent_state: String,
    #[serde(default)]
    pub uac_detected: bool,
    #[serde(default)]
    pub token_integrity_level: u32,
    #[serde(default)]
    pub last_error: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForegroundResponse {
    pub pid: u32,
    pub title: String,
    pub path: Option<String>,
    pub process_name: String,
    pub hwnd: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplayModeResponse {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplayInfoResponse {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub current_mode: DisplayModeResponse,
    pub modes: Vec<DisplayModeResponse>,
}
