// === Custom directories ===

#[derive(Debug, Deserialize)]
pub struct AddCustomDirRequest {
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    #[serde(default)]
    pub max_depth: u32,
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    #[serde(default = "default_bool_true")]
    pub recursive: bool,
}

fn default_bool_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateCustomDirRequest {
    pub path: Option<String>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub max_depth: Option<u32>,
    pub extensions: Option<Vec<String>>,
    pub recursive: Option<bool>,
}


