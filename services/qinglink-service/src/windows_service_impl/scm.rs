//! Windows SCM 入口与有序的服务停机。

use super::{PIPE_NAME, SERVICE_NAME, dispatch, lifecycle_monitor, network, pipe};
use anyhow::{Context, Result};
use std::io::Write;
use std::sync::mpsc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use windows_service::service::{ServiceControl, ServiceState, ServiceStatus, ServiceType};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

pub(super) fn run() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("failed to dispatch Windows service")?;
    Ok(())
}

extern "system" fn ffi_service_main(_arguments: u32, _argv: *mut *mut u16) {
    if let Err(error) = service_main() {
        eprintln!("{error:#}");
    }
}

fn service_main() -> Result<()> {
    crate::logging::init_service_logging();
    tracing::info!("QingLink service starting");
    let (stop_tx, stop_rx) = mpsc::channel();
    let control_stop_tx = stop_tx.clone();
    let lifecycle_stop = Arc::new(AtomicBool::new(false));
    let monitor = lifecycle_monitor::LifecycleMonitor::new(Arc::clone(&lifecycle_stop));
    monitor.start_monitors();
    let paths = qinglink_storage::AppPaths::resolve_system()?;
    let database = Arc::new(qinglink_storage::database::DatabaseService::new(
        paths.clone(),
    ));
    database.register_database(qinglink_storage::database::DatabaseSpec::app(
        "qinglink/data",
        "qinglink.db",
    ))?;
    let state = qinglink_server::AppState::new(
        qinglink_server::service::RemoteControlService::new(paths),
        database,
    );
    let control_cancel = Arc::clone(&lifecycle_stop);
    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = control_stop_tx.send(());
                control_cancel.store(true, Ordering::Release);
                wake_control_pipe();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;
    status_handle
        .set_service_status(status(ServiceState::StartPending, Duration::from_secs(60)))?;
    let agent_stop = Arc::new(AtomicBool::new(false));
    let agent_worker_stop = Arc::clone(&agent_stop);
    let agent_worker = std::thread::spawn(move || super::session_agent::run(&agent_worker_stop));
    let network_stop = Arc::new(AtomicBool::new(false));
    let network_worker_stop = Arc::clone(&network_stop);
    let network_state = state.clone();
    let network_worker =
        std::thread::spawn(move || network::run(&network_worker_stop, &network_state));
    status_handle.set_service_status(status_running())?;
    loop {
        if stop_rx.try_recv().is_ok() || lifecycle_stop.load(Ordering::Acquire) {
            break;
        }
        if let Err(error) = serve_one_client(&monitor, &state, &stop_tx) {
            tracing::warn!(%error, "客户端请求处理失败，继续运行");
        }
    }
    // Stop network listeners first so their held-input cleanup runs before the
    // session agent is terminated; the two workers intentionally have separate flags.
    network_stop.store(true, Ordering::Release);
    let _ = network_worker.join();
    agent_stop.store(true, Ordering::Release);
    let _ = agent_worker.join();
    status_handle.set_service_status(status_stopped())?;
    Ok(())
}

fn serve_one_client(
    monitor: &lifecycle_monitor::LifecycleMonitor,
    state: &qinglink_server::AppState,
    stop_tx: &mpsc::Sender<()>,
) -> Result<()> {
    let stream = pipe::accept(
        PIPE_NAME,
        "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)",
        Arc::clone(&monitor.should_stop),
    )?;
    dispatch::handle_client(stream, monitor, Some(state), Some(stop_tx))
}

fn wake_control_pipe() {
    if let Ok(mut pipe) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE_NAME)
    {
        let _ = pipe.write_all(&0u32.to_le_bytes());
    }
}

fn status(state: ServiceState, wait_hint: Duration) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: windows_service::service::ServiceControlAccept::empty(),
        exit_code: windows_service::service::ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint,
        process_id: None,
    }
}
fn status_running() -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: windows_service::service::ServiceControlAccept::STOP
            | windows_service::service::ServiceControlAccept::SHUTDOWN,
        exit_code: windows_service::service::ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    }
}
fn status_stopped() -> ServiceStatus {
    status(ServiceState::Stopped, Duration::default())
}
