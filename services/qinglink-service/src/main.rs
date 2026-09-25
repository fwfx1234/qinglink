//! QingLink system service and secure-session agent.
//!
//! 一个高权限 Windows 服务，为 QingLink 远程控制应用
//! 提供安全桌面输入注入功能。

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("QingLinkService is only supported on Windows");
}

#[cfg(target_os = "windows")]
mod logging;

#[cfg(target_os = "windows")]
mod windows_service_impl;

#[cfg(target_os = "windows")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--inject-move") {
        let dx = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
        let dy = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(0);
        windows_service_impl::helper_move(dx, dy);
        return;
    }
    if args.get(1).map(String::as_str) == Some("--inject-button") {
        windows_service_impl::helper_button(args.get(2).map(String::as_str).unwrap_or("left"));
        return;
    }
    if args.get(1).map(String::as_str) == Some("--inject-button-state") {
        windows_service_impl::helper_button_state(
            args.get(2).map(String::as_str).unwrap_or("left"),
            args.get(3).map(String::as_str).unwrap_or("up"),
        );
        return;
    }
    if args.get(1).map(String::as_str) == Some("--inject-wheel") {
        windows_service_impl::helper_wheel(args.get(2).and_then(|v| v.parse().ok()).unwrap_or(0));
        return;
    }
    if args.get(1).map(String::as_str) == Some("--inject-key") {
        windows_service_impl::helper_key(args.get(2).map(String::as_str).unwrap_or(""));
        return;
    }
    if args.get(1).map(String::as_str) == Some("--session-agent") {
        logging::init_service_logging();
        if let Err(error) = windows_service_impl::run_session_agent_entry(&args) {
            tracing::error!(%error, "QingLink session agent failed");
            std::process::exit(1);
        }
        return;
    }
    logging::init_service_logging();
    if let Err(error) = windows_service_impl::run() {
        tracing::error!(%error, "QingLinkService failed");
        std::process::exit(1);
    }
}
