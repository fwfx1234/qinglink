//! COM 公寓线程初始化辅助：多线程单元，供音频、图标与快捷方式解析共用。

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// COM 公寓线程初始化辅助
pub(super) struct ComApartment {
    initialized: bool,
}

impl ComApartment {
    pub(super) fn init() -> windows::core::Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if result.is_ok() {
            return Ok(Self { initialized: true });
        }
        if result == RPC_E_CHANGED_MODE {
            // The current thread already belongs to another valid COM
            // apartment. It remains usable, but must not be uninitialized here.
            return Ok(Self { initialized: false });
        }
        Err(windows::core::Error::from(result))
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}
