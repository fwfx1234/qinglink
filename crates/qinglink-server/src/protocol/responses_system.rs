// === System responses ===

#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_percent: f32,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct AuthenticatedResponse {
    pub token: String,
    pub session_id: String,
    pub expires_at: Option<i64>,
    pub mac_address: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeStatus {
    pub percent: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputsResponse {
    pub devices: Vec<AudioOutputDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputStatus {
    pub available: bool,
    pub service_running: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputResult {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
