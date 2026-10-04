//! 管理界面入口：监听端口、URL，以及"交给系统默认浏览器"。
//!
//! 界面只服务回环地址，端口在这里定义，Web 线程与托盘入口都用同一个值。

use anyhow::Result;

/// Port serving the management interface. Bound to loopback only.
pub const WEB_PORT: u16 = 3722;

/// The management interface URL.
pub fn web_url() -> String {
    format!("http://127.0.0.1:{WEB_PORT}")
}

/// Open the management interface in the default browser.
///
/// Called both for an explicit request (tray click, API call) and as the
/// fallback when the notification area turns out to be unreachable.
///
/// The launch itself lives in `qinglink_windows::browser`, which keeps the
/// browser out of an elevated token — an elevated Edge refuses to start, which
/// looks like "the browser is not responding" and asks for administrator rights.
pub fn open_browser() -> Result<()> {
    let url = web_url();
    tracing::info!(
        url,
        elevated = qinglink_windows::browser::is_elevated(),
        "打开管理界面"
    );
    match qinglink_windows::browser::open_url(&url) {
        Ok(qinglink_windows::browser::OpenOutcome::Opened) => {
            tracing::debug!(url, "已把管理界面交给系统默认浏览器");
            Ok(())
        }
        Ok(qinglink_windows::browser::OpenOutcome::BelowShellIntegrity) => {
            let error = anyhow::anyhow!(
                "本进程完整性低于通知区域所有者（{} < {}），请手动访问 {url}",
                qinglink_windows::integrity::current_integrity_name(),
                qinglink_windows::integrity::shell_integrity_name(),
            );
            tracing::warn!(%error, "无法把界面交给浏览器");
            Err(error)
        }
        Err(error) => {
            tracing::error!(%error, url, "无法打开浏览器，请手动访问");
            Err(error)
        }
    }
}
