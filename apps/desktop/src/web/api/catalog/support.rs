//! 目录 handler 共用的上限常量与入参校验。

use crate::web::error::ApiError;

/// Longest display name accepted by the service, mirrored here so the UI can be
/// rejected before a round trip.
pub(super) const MAX_DISPLAY_NAME_CHARS: usize = 128;

/// Longest executable path accepted by the service, mirrored for early rejection.
pub(super) const MAX_APP_PATH_CHARS: usize = 512;

/// Only the two kinds the desktop actually manages.
pub(super) fn normalize_kind(kind: Option<&str>) -> &'static str {
    match kind.map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("game") => "game",
        _ => "app",
    }
}

/// 校验条目 id：去掉首尾空白后为空即按“缺少参数”拒绝。
pub(super) fn required_id(id: &str) -> Result<String, ApiError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request("缺少 id"));
    }
    Ok(trimmed.to_string())
}
