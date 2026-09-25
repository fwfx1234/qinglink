pub const MAX_MOUSE_MOVE_DELTA: i32 = 500;

// === Auth ===

#[derive(Debug, Deserialize)]
pub struct AuthPairRequest {
    pub pin: String,
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthVerifyRequest {
    pub token: String,
}

// === System ===

#[derive(Debug, Default, Deserialize)]
pub struct ShutdownRequest {
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub delay_secs: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct SleepRequest {
    #[serde(default)]
    pub hibernate: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct RestartRequest {
    #[serde(default)]
    pub force: bool,
}


