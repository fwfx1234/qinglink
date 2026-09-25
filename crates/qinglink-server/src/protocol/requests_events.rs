// === WebSocket events ===

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    ForegroundChanged { data: ForegroundInfo },
    ProcessStarted { data: ProcessBasic },
    ProcessEnded { data: ProcessBasic },
    SystemSuspend,
    SystemResume,
    SystemShutdown { reason: String },
    SystemSleep,
    LauncherCatalogReady { kind: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct ForegroundInfo {
    pub pid: u32,
    pub title: String,
    pub path: Option<String>,
    pub hwnd: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessBasic {
    pub pid: u32,
    pub name: String,
}


