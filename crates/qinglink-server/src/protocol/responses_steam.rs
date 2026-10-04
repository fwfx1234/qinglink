// === Steam responses ===

/// Steam 游戏扫描结果响应。
#[derive(Debug, Serialize)]
pub struct SteamGamesResponse {
    /// 是否检测到 Steam 安装。
    pub steam_installed: bool,
    /// Steam 安装目录路径，未安装时为 None。
    pub steam_path: Option<String>,
    /// 检测到的 Steam 库目录列表。
    pub libraries: Vec<crate::steam::SteamLibrary>,
    /// 扫描到的游戏列表。
    pub games: Vec<crate::steam::SteamGame>,
    /// 游戏总数。
    pub total: usize,
}

