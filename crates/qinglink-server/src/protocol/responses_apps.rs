// === App responses ===

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct SearchAppsResponse {
    pub apps: Vec<AppInfo>,
}


