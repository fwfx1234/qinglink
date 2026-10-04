//! 窗口列表缓存：短期复用已枚举的窗口，支持显式失效。

use std::time::{Duration, Instant};

use crate::server::AppState;

impl AppState {
    /// 返回当前窗口列表，缓存过期或要求强制刷新时重新枚举。
    pub fn list_windows(&self, force_refresh: bool) -> Vec<crate::protocol::requests::WindowInfo> {
        const WINDOW_CACHE_TTL: Duration = Duration::from_millis(500);
        if !force_refresh
            && let Ok(cache) = self.window_list_cache.lock()
            && let Some((created_at, windows)) = cache.as_ref()
            && created_at.elapsed() < WINDOW_CACHE_TTL
        {
            return windows.clone();
        }
        let windows = crate::platform::enum_windows_list();
        if let Ok(mut cache) = self.window_list_cache.lock() {
            *cache = Some((Instant::now(), windows.clone()));
        }
        windows
    }

    /// 丢弃窗口列表缓存，使下一次查询重新枚举窗口。
    pub fn invalidate_window_list(&self) {
        if let Ok(mut cache) = self.window_list_cache.lock() {
            *cache = None;
        }
    }
}
