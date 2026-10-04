//! 独立于 SCM 与控制 IPC 调和网络监听器。
use qinglink_server::AppState;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

pub(super) fn run(stop: &Arc<AtomicBool>, state: &AppState) {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            state.control.set_network_error(Some(error.to_string()));
            return;
        }
    };
    runtime.block_on(async {
        let mut backoff = Duration::from_secs(1);
        let mut cleanup_pending = false;
        while !stop.load(Ordering::Acquire) {
            if cleanup_pending {
                if let Err(error) = release_inputs(state.clone()).await {
                    state
                        .control
                        .set_network_error(Some(format!("input cleanup: {error}")));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                cleanup_pending = false;
                state.control.set_network_error(None);
            }
            if state.control.is_paused() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            let generation = state.input_generation();
            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
            match qinglink_server::RemoteServer::run_with_shutdown(state.clone(), 3721, async {
                let _ = shutdown_rx.await;
            })
            .await
            {
                Ok((address, mut server)) => {
                    let discovery = qinglink_discovery::UdpDiscoveryResponder::start(
                        state.service.pc_id(),
                        state.service.pc_name(),
                        address.port(),
                        None,
                    );
                    let discovery = match discovery {
                        Ok(discovery) => discovery,
                        Err(error) => {
                            state
                                .control
                                .set_network_error(Some(format!("UDP discovery: {error}")));
                            state.service.set_server_running(false, address.port());
                            let _ = shutdown_tx.send(());
                            if tokio::time::timeout(Duration::from_secs(1), &mut server)
                                .await
                                .is_err()
                            {
                                server.abort();
                            }
                            wait_retry(stop, state, backoff).await;
                            backoff = (backoff * 2).min(Duration::from_secs(15));
                            continue;
                        }
                    };
                    state.control.set_network_error(None);
                    backoff = Duration::from_secs(1);
                    tracing::info!(%address, "network listeners ready");
                    loop {
                        if stop.load(Ordering::Acquire)
                            || state.control.is_paused()
                            || state.input_generation() != generation
                            || server.is_finished()
                        {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                    // Withdraw discovery and close both channels before draining.
                    drop(discovery);
                    state.invalidate_input();
                    state.service.set_server_running(false, address.port());
                    state.connection_registry.close_all("SERVICE_STOPPING");
                    state
                        .input_connection_registry
                        .close_all("SERVICE_STOPPING");
                    let _ = shutdown_tx.send(());
                    if tokio::time::timeout(Duration::from_secs(1), &mut server)
                        .await
                        .is_err()
                    {
                        server.abort();
                    }
                    if let Err(error) = release_inputs(state.clone()).await {
                        cleanup_pending = true;
                        state
                            .control
                            .set_network_error(Some(format!("input cleanup: {error}")));
                    }
                }
                Err(error) => {
                    state.control.set_network_error(Some(error.to_string()));
                    tracing::warn!(%error, ?backoff, "network listener unavailable");
                    wait_retry(stop, state, backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(15));
                }
            }
        }
    });
}

async fn release_inputs(state: AppState) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || state.release_held_input()).await?
}

async fn wait_retry(stop: &AtomicBool, state: &AppState, delay: Duration) {
    let deadline = tokio::time::Instant::now() + delay;
    while tokio::time::Instant::now() < deadline
        && !stop.load(Ordering::Acquire)
        && !state.control.is_paused()
    {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
