use crate::{Result, WindowsCapabilityError};

/// 校验当前平台是否支持显示控制能力。
///
/// # Errors
/// 当编译目标不是 Windows 时返回
/// [`WindowsCapabilityError::Unsupported`]（能力名为 `display control`）。
pub fn ensure_supported() -> Result<()> {
    super::is_supported()
        .then_some(())
        .ok_or(WindowsCapabilityError::Unsupported("display control"))
}
