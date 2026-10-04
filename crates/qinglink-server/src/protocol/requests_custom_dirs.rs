// === Custom directories ===

/// 新增自定义扫描目录的请求。
#[derive(Debug, Deserialize)]
pub struct AddCustomDirRequest {
    /// 要添加的目录绝对路径。
    pub path: String,
    /// 该目录的显示名称，缺省时由服务端依据路径生成。
    #[serde(default)]
    pub name: Option<String>,
    /// 是否启用该目录的扫描，默认为 true。
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    /// 递归扫描的最大子目录深度，0 表示仅扫描当前层。
    #[serde(default)]
    pub max_depth: u32,
    /// 只收录的文件扩展名过滤列表，为空表示不限扩展名。
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    /// 是否递归扫描子目录，默认为 true。
    #[serde(default = "default_bool_true")]
    pub recursive: bool,
}

fn default_bool_true() -> bool {
    true
}

/// 更新自定义扫描目录的请求，仅提交需要修改的字段。
#[derive(Debug, Deserialize)]
pub struct UpdateCustomDirRequest {
    /// 新的目录绝对路径。
    pub path: Option<String>,
    /// 新的显示名称。
    pub name: Option<String>,
    /// 是否启用该目录的扫描。
    pub enabled: Option<bool>,
    /// 新的递归扫描深度上限。
    pub max_depth: Option<u32>,
    /// 新的扩展名过滤列表。
    pub extensions: Option<Vec<String>>,
    /// 是否递归扫描子目录。
    pub recursive: Option<bool>,
}

