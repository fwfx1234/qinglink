//! 目录写操作的入参校验与稳定 id。
//!
//! 同一份目录有两个写入口：手机端的 WebSocket 请求，与桌面端的管理 IPC。
//! 两边都必须经过同一组校验器——**放在 server crate 而不是 service crate 是
//! 结构决定的**：依赖方向只能是 `service → server`，校验器若留在 service，
//! `qinglink-server` 引用它就会编译不过，WS 入口于是"无法校验"而不是"忘了校验"。
//! 修复前正是这个状态：同一 exe 加两次得两行、删不存在的 id 也回成功。

use sha2::{Digest, Sha256};

/// 显示名的字符数上限。
pub const MAX_DISPLAY_NAME: usize = 128;

/// 可执行文件路径的字符数上限。
pub const MAX_APP_PATH: usize = 512;

/// 用户添加条目时可携带的启动参数个数上限。
pub const MAX_APP_ARGS: usize = 64;

/// 目录写操作的失败原因：稳定错误码 + 可直接展示的文案。
///
/// 两种传输各自把它渲染成自己的信封（WS 的 `WsServerResponse::error`、
/// 管理 IPC 的 `{ok, code, message}`），错误码集合因此只有一份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError {
    /// 稳定错误码，不参与本地化。
    pub code: &'static str,
    /// 面向用户的文案。
    pub message: String,
}

impl CatalogError {
    /// 用给定错误码与文案构造。
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 入参不合法。
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message)
    }

    /// 目标条目或文件不存在。
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", message)
    }

    /// 与既有目录冲突（例如同一 exe 被添加两次）。
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("CONFLICT", message)
    }

    /// 数据库不可用等内部失败。
    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::new("INTERNAL", error.to_string())
    }
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// 校验目录条目 id：空 id 查库只会得到 0 行，属于入参错误而不是"不存在"。
///
/// # Errors
/// id 去空白后为空时返回 `BAD_REQUEST`。
pub fn validate_entry_id(id: &str) -> Result<String, CatalogError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(CatalogError::bad_request("缺少条目 id"));
    }
    Ok(trimmed.to_string())
}

/// 路径比较用的规范形式：去空白与引号、统一分隔符、转小写。
pub fn normalize_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('/', "\\")
        .to_lowercase()
}

/// 校验显示名并返回去空白后的值；空串合法，表示"恢复扫描得到的名字"。
///
/// # Errors
/// 名称含控制字符或超过 [`MAX_DISPLAY_NAME`] 个字符时返回 `BAD_REQUEST`。
pub fn validate_display_name(name: &str) -> Result<String, CatalogError> {
    let trimmed = name.trim();
    if trimmed.chars().any(char::is_control) {
        return Err(CatalogError::bad_request("名称不能包含控制字符"));
    }
    if trimmed.chars().count() > MAX_DISPLAY_NAME {
        return Err(CatalogError::bad_request(format!(
            "名称不能超过 {MAX_DISPLAY_NAME} 个字符"
        )));
    }
    Ok(trimmed.to_string())
}

/// 校验用户给出的可执行文件路径并返回去引号后的值。
///
/// # Errors
/// 路径为空、超长、含控制字符，或该路径在当前机器上不是文件时返回错误。
pub fn validate_app_path(path: &str) -> Result<String, CatalogError> {
    let trimmed = validate_app_path_input(path)?;
    if !std::path::Path::new(&trimmed).is_file() {
        return Err(CatalogError::not_found("可执行文件不存在"));
    }
    Ok(trimmed)
}

/// 只校验路径的**形状**（去引号、非空、长度、控制字符），不要求文件仍在。
///
/// 修改既有条目时必须用这个版本：条目可能比磁盘上的程序活得更久，例如先
/// 卸载程序再重命名或删除条目——若要求 `is_file()`，这些条目就再也改不动了。
///
/// # Errors
/// 路径为空、超长或含控制字符时返回 `BAD_REQUEST`。
pub fn validate_app_path_input(path: &str) -> Result<String, CatalogError> {
    let trimmed = path.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return Err(CatalogError::bad_request("缺少可执行文件路径"));
    }
    if trimmed.chars().count() > MAX_APP_PATH || trimmed.chars().any(char::is_control) {
        return Err(CatalogError::bad_request("可执行文件路径不合法"));
    }
    Ok(trimmed.to_string())
}

/// 用户给定路径对应的稳定目录 id。
///
/// 由规范化路径派生，因此同一 exe 永远映射到同一行——重复添加才能变成显式
/// 冲突而不是第二行。
pub fn app_id_for_path(path: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(normalize_path(path).as_bytes()));
    format!("user:{}", &digest[..16])
}

/// 调用方没给显示名时，从可执行文件名推断一个。
pub fn default_app_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.trim().chars().take(MAX_DISPLAY_NAME).collect())
        .unwrap_or_default()
}

/// 去掉启动参数里的控制字符、丢弃空串并截断到 [`MAX_APP_ARGS`] 个。
pub fn sanitize_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|argument| {
            argument
                .chars()
                .filter(|character| !character.is_control())
                .collect::<String>()
        })
        .filter(|argument| !argument.is_empty())
        .take(MAX_APP_ARGS)
        .collect()
}
