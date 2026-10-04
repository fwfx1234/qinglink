//! 平台无关的服务状态查询门面与等待收敛。
//!
//! 桌面端与提权助手都只经过这里查询服务：Windows 上转发给 [`super::query`]，
//! 其它平台一律报告“不存在、未运行”，并用固定间隔轮询直到状态收敛。

use std::{path::Path, thread, time::Duration};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const POLL_ATTEMPTS: usize = 40;

pub(crate) fn service_exists(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        super::query::exists(name)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        false
    }
}

pub(crate) fn service_is_running(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        super::query::state(name) == Some(super::ffi::SERVICE_RUNNING)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        false
    }
}

pub(crate) fn service_image_path_matches(name: &str, expected: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        super::query::image_path_matches(name, expected)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (name, expected);
        false
    }
}

pub(crate) fn wait_for_stopped(name: &str) {
    for _ in 0..POLL_ATTEMPTS {
        if !service_exists(name) || !service_is_running(name) {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

pub(crate) fn wait_for_running(name: &str) {
    for _ in 0..POLL_ATTEMPTS {
        if service_is_running(name) {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
}
