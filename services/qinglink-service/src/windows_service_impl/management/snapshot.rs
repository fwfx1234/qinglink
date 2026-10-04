//! 服务快照：桌面端轮询以获取暂停、网络、配对与扫描状态。

use crate::windows_service_impl::management::record::{CommandError, CommandResult, value};
use qinglink_protocol::service_ipc::{CatalogScanState, PendingPairing, ServiceSnapshot};
use qinglink_server::AppState;

/// Build the snapshot the desktop UI renders: pause state, listener address,
/// pending pairing request, device count and both scan stamps.
pub(super) fn snapshot(state: &AppState) -> CommandResult {
    let pending_pairing = if state.control.pairing_available() {
        let manager = state
            .pairing
            .lock()
            .map_err(|_| CommandError::internal("pairing state unavailable"))?;
        let now = std::time::Instant::now();
        manager
            .pending()
            .filter(|request| request.expires_at > now)
            .min_by_key(|request| request.expires_at)
            .map(|request| PendingPairing {
                request_id: request.request_id.to_string(),
                device_id: request.device_id.clone(),
                device_name: request.device_name.clone(),
                pin: request.pin.clone(),
                expires_at_unix: unix_seconds()
                    + request.expires_at.saturating_duration_since(now).as_secs() as i64,
            })
    } else {
        None
    };
    let paused = state.control.is_paused();
    let network_ready = state.service.is_server_running() && !paused;
    // Scan stamps and the active device count ride along with the snapshot so the
    // desktop can detect background changes without a second round trip.
    let scan = state.app_db.lock().ok().map(|db| CatalogScanState {
        app: db.scan_state("app").ok().flatten(),
        game: db.scan_state("game").ok().flatten(),
    });
    value(ServiceSnapshot {
        paused,
        network_ready,
        address: network_ready.then(|| format!("0.0.0.0:{}", state.service.server_port())),
        last_error: state.control.network_error(),
        pending_pairing,
        device_count: Some(state.token_store.count_active()),
        scan,
    })
}

fn unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}
