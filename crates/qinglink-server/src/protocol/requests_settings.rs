// === Server settings ===

#[derive(Debug, Default, Deserialize)]
pub struct UpdateSettingsRequest {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub auto_start: Option<bool>,
    #[serde(default)]
    pub minimize_to_tray: Option<bool>,
}


