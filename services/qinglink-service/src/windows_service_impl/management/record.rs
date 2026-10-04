//! 命令失败与应答编码。
//!
//! 每条管理命令都以相同的 `{ok, code, message}` 形状应答；这里的辅助函数把这一
//! 契约集中到一处，避免任何处理函数自造错误信封。

use serde::Serialize;

/// A command failure with a stable code the desktop maps onto its HTTP error
/// codes. `code` is never interpolated into user-visible text.
#[derive(Debug)]
pub(super) struct CommandError {
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl CommandError {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(super) fn internal(error: impl std::fmt::Display) -> Self {
        Self::new("INTERNAL", error.to_string())
    }

    pub(super) fn not_found() -> Self {
        Self::new("NOT_FOUND", "目录中不存在该条目")
    }
}

/// Result of a single desktop command: a JSON payload, or a coded failure.
pub(super) type CommandResult = std::result::Result<serde_json::Value, CommandError>;

/// Encode any serializable payload as the command's success value.
pub(super) fn value(value: impl Serialize) -> CommandResult {
    serde_json::to_value(value).map_err(CommandError::internal)
}
