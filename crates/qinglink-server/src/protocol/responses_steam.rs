// === Steam responses ===

#[derive(Debug, Serialize)]
pub struct SteamGamesResponse {
    pub steam_installed: bool,
    pub steam_path: Option<String>,
    pub libraries: Vec<crate::steam::SteamLibrary>,
    pub games: Vec<crate::steam::SteamGame>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SteamRefreshResponse {
    pub scanned: usize,
}


