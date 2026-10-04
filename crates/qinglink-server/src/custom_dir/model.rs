//! 自定义扫描目录的配置模型：目录配置、请求体与目录校验结果。

use serde::{Deserialize, Serialize};

/// 自定义扫描目录配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomDir {
    /// 目录的唯一标识
    pub id: String,
    /// 目录的规范化绝对路径
    pub path: String,
    /// 目录的显示名称
    pub name: String,
    /// 是否启用该目录参与扫描
    pub enabled: bool,
    /// 递归扫描的最大深度，0 表示使用默认上限
    #[serde(default)]
    pub max_depth: u32,
    /// 参与扫描的文件扩展名（不含前导点）
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    /// 是否递归扫描子目录
    #[serde(default = "default_bool_true")]
    pub recursive: bool,
    /// 创建时间（Unix 时间戳，秒）
    pub created_at: i64,
    /// 最近一次扫描时间（Unix 时间戳，秒）
    pub last_scanned: Option<i64>,
}

/// 未显式给出扩展名时的默认值：可执行文件与快捷方式。
pub(super) fn default_extensions() -> Vec<String> {
    vec!["exe".to_string(), "lnk".to_string()]
}

/// `recursive` 字段的缺省值。
fn default_bool_true() -> bool {
    true
}

/// 自定义目录配置集合
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CustomDirConfig {
    /// 已配置的自定义扫描目录列表
    #[serde(default)]
    pub dirs: Vec<CustomDir>,
}

/// 添加自定义目录请求
#[derive(Clone, Debug, Deserialize)]
pub struct AddCustomDirRequest {
    /// 待添加目录的路径
    pub path: String,
    /// 目录显示名称，缺省时取目录名
    #[serde(default)]
    pub name: Option<String>,
    /// 是否启用该目录
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    /// 递归扫描的最大深度
    #[serde(default)]
    pub max_depth: u32,
    /// 参与扫描的文件扩展名，缺省为 exe 与 lnk
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    /// 是否递归扫描子目录
    #[serde(default = "default_bool_true")]
    pub recursive: bool,
}

/// 更新自定义目录请求
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateCustomDirRequest {
    /// 新的目录路径
    pub path: Option<String>,
    /// 新的显示名称
    pub name: Option<String>,
    /// 是否启用该目录
    pub enabled: Option<bool>,
    /// 新的递归扫描最大深度
    pub max_depth: Option<u32>,
    /// 新的文件扩展名列表
    pub extensions: Option<Vec<String>>,
    /// 是否递归扫描子目录
    pub recursive: Option<bool>,
}

/// 目录验证结果
#[derive(Clone, Debug, Serialize)]
pub struct DirValidationResult {
    /// 目录是否通过校验
    pub valid: bool,
    /// 路径是否存在
    pub exists: bool,
    /// 路径是否为目录
    pub is_dir: bool,
    /// 目录是否可读
    pub readable: bool,
    /// 目录顶层可执行文件数量
    pub exe_count: usize,
    /// 校验失败的原因
    pub error: Option<String>,
}
