// === Server management responses ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerSettings {
    pub port: u16,
    pub auto_start: bool,
    pub minimize_to_tray: bool,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            port: 3721,
            auto_start: false,
            minimize_to_tray: false,
        }
    }
}


