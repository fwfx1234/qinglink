use crate::{Result, WindowsCapabilityError};

pub fn ensure_supported() -> Result<()> {
    super::is_supported()
        .then_some(())
        .ok_or(WindowsCapabilityError::Unsupported("session control"))
}
