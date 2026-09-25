#![cfg_attr(windows, windows_subsystem = "windows")]

mod input_install;
mod logging;
mod service_lifecycle;
mod service_runtime;
mod tray;
mod view;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use gpui::{
    App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowKind, WindowOptions, px, size,
};
use qinglink_server::{AppState, RemoteControlService, server::pairing};
use qinglink_storage::{
    AppPaths,
    database::{DatabaseService, DatabaseSpec},
};
use qinglink_windows::single_instance::{SingleInstance, activate_existing};

use crate::{service_runtime::ServiceRuntime, tray::TrayController, view::RemoteDashboard};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(result) = input_install::handle_helper_command(&args) {
        return result;
    }
    let paths = AppPaths::resolve()?;
    let (_log_guard, log_switch) = logging::initialize(&paths)?;
    qinglink_windows::session::ensure_supported()?;

    let Some(_instance) = SingleInstance::acquire()? else {
        activate_existing()?;
        return Ok(());
    };

    let database = Arc::new(DatabaseService::new(paths.clone()));
    database.migrate_application_database()?;
    database.register_database(DatabaseSpec::app("qinglink/data", "qinglink.db"))?;
    database.register_database(DatabaseSpec::app("qinglink/scanner", "qinglink.db"))?;
    let service = RemoteControlService::new(paths.clone());
    let state = AppState::new(service, Arc::clone(&database));

    // 环境变量（QINGLINK_LOG_LEVEL/RUST_LOG）优先，否则使用上次界面里保存的开关。
    let debug_logging = if std::env::var_os("QINGLINK_LOG_LEVEL").is_some()
        || std::env::var_os("RUST_LOG").is_some()
    {
        log_switch.is_debug()
    } else {
        state
            .app_db
            .lock()
            .ok()
            .and_then(|database| database.get_setting("logging.debug").ok().flatten())
            .is_some_and(|value| value == "true")
    };
    log_switch.set_debug(debug_logging);
    tracing::info!(debug_logging, "日志级别已应用");
    let silent = args.contains(&"--silent".to_string());
    let startup = args.contains(&"--startup".to_string());
    tracing::info!(silent, startup, "桌面端启动");

    let mut lifecycle_manager = service_lifecycle::ServiceLifecycleManager::new();
    let lifecycle_result = if silent || startup {
        lifecycle_manager.try_start()
    } else {
        lifecycle_manager.start()
    };
    if let Err(error) = lifecycle_result {
        tracing::warn!(%error, "服务生命周期管理器启动失败，继续运行");
    }

    // Start the desktop-hosted server only after the Windows service has had
    // a chance to claim port 3721. This ordering prevents both implementations
    // from racing for the same listener during startup.
    let pairing_events = pairing::subscribe(&state);
    let pairing_snapshot = Arc::new(Mutex::new(None));
    let runtime =
        ServiceRuntime::start(state.clone(), pairing_events, Arc::clone(&pairing_snapshot));
    let persisted_paused = state
        .app_db
        .lock()
        .ok()
        .and_then(|database| database.get_setting("service.paused").ok().flatten())
        .is_some_and(|value| value == "true");
    if persisted_paused {
        let _ = runtime
            .desired
            .send(service_runtime::DesiredServiceState::Paused);
    }

    let autostart = qinglink_windows::autostart::is_enabled().unwrap_or(false);
    let show_on_start = !(silent || startup);
    let app = gpui::Application::new();
    let startup_error = Arc::new(Mutex::new(None::<String>));
    let error_slot = Arc::clone(&startup_error);
    let dashboard_state = state;
    let dashboard_status = Arc::clone(&runtime.status);
    let desired_service = runtime.desired.clone();
    app.run(move |cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(820.0), px(560.0)),
                cx,
            ))),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("轻联 QingLink".into()),
                ..Default::default()
            }),
            focus: show_on_start,
            show: show_on_start,
            window_background: WindowBackgroundAppearance::Opaque,
            window_decorations: Some(WindowDecorations::Server),
            kind: WindowKind::Normal,
            ..Default::default()
        };
        let tray = match TrayController::new(false, autostart) {
            Ok(tray) => Some(tray),
            Err(error) => {
                tracing::warn!(error = %error, "托盘图标创建失败，程序将继续运行（无托盘功能）");
                None
            }
        };
        match cx.open_window(options, move |window, cx| {
            cx.new(|cx| {
                RemoteDashboard::new(
                    dashboard_state,
                    pairing_snapshot,
                    dashboard_status,
                    desired_service,
                    tray,
                    autostart,
                    show_on_start,
                    log_switch,
                    window,
                    cx,
                )
            })
        }) {
            Ok(_) => {}
            Err(error) => {
                if let Ok(mut slot) = error_slot.lock() {
                    *slot = Some(error.to_string());
                }
                cx.quit();
            }
        }
    });

    runtime.shutdown()?;
    let _ = lifecycle_manager.shutdown();
    database.shutdown();
    if let Some(error) = startup_error.lock().ok().and_then(|slot| slot.clone()) {
        anyhow::bail!(error);
    }
    Ok(())
}
