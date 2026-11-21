//! 周期快照刷新：状态迁移与变更事件派发。

use std::sync::Arc;
use std::time::Instant;

use qinglink_protocol::service_ipc::{
    CatalogScanState, DesktopCommand, SERVICE_IPC_VERSION, ServiceSnapshot,
};

use super::envelope::{decode, ensure_negotiated, exchange};
use super::inner::Inner;
use super::transport::Transport;
use super::{ServiceEvent, ServiceLinkState};
use crate::service::{SNAPSHOT_INTERVAL, ServiceError};

pub(super) fn refresh_snapshot(
    transport: &mut impl Transport,
    inner: &Arc<Inner>,
    next_snapshot: &mut Instant,
) -> Result<(), ServiceError> {
    if Instant::now() < *next_snapshot {
        return Ok(());
    }
    ensure_negotiated(transport, inner)?;

    // A locked desktop cannot approve pairings, so a PIN must never be shown
    // from a snapshot taken while the workstation was locked.
    let desktop_unlocked = qinglink_windows::secure_desktop::pairing_desktop_available();
    let mut snapshot: ServiceSnapshot = exchange(
        transport,
        DesktopCommand::GetSnapshot { desktop_unlocked },
        inner,
    )
    .and_then(decode)?;
    if !desktop_unlocked {
        snapshot.pending_pairing = None;
    }

    // Read everything the diff needs before the state is overwritten: the
    // browser is only told about *changes*, because it cannot poll.
    let (
        previous_view,
        previous_pairing,
        previous_devices,
        previous_scan,
        previous_keep_awake,
        had_snapshot,
    ) = inner
        .state
        .lock()
        .map(|state| {
            (
                Some(state.view()),
                state.snapshot.pending_pairing.clone(),
                state.snapshot.device_count,
                scan_fingerprint(state.snapshot.scan.as_ref()),
                state.snapshot.keep_awake,
                state.snapshot_at.is_some(),
            )
        })
        .unwrap_or((None, None, None, (None, None), None, false));
    let current_pairing = snapshot.pending_pairing.clone();
    let current_devices = snapshot.device_count;
    let current_scan = scan_fingerprint(snapshot.scan.as_ref());
    let current_keep_awake = snapshot.keep_awake;

    if let Ok(mut state) = inner.state.lock() {
        state.snapshot = snapshot;
        state.snapshot_at = Some(Instant::now());
        state.connected = true;
        state.link_state = ServiceLinkState::Connected;
        state.service_version = Some(SERVICE_IPC_VERSION);
        state.last_error = None;
    }
    *next_snapshot = Instant::now() + SNAPSHOT_INTERVAL;

    let current_view = inner.state.lock().ok().map(|state| state.view());
    if previous_view != current_view {
        let _ = inner.events.send(ServiceEvent::Status);
    }

    // 保持唤醒以服务快照为唯一状态源：首帧也要发，否则服务侧已开启而桌面端
    // 初始值为 false 时，设置页与托盘会一直停在错误的状态上。
    if let Some(enabled) = current_keep_awake
        && previous_keep_awake != current_keep_awake
    {
        let _ = inner.events.send(ServiceEvent::KeepAwake(enabled));
    }

    // Pairing has its own channel and payload, so it is announced separately.
    if pairing_fingerprint(&previous_pairing) != pairing_fingerprint(&current_pairing) {
        let _ = inner.events.send(ServiceEvent::Pairing(current_pairing));
    }

    // Device and scan diffs are only meaningful once a previous snapshot was
    // received; otherwise a freshly started desktop would announce a spurious
    // "everything changed" on its first poll.
    if had_snapshot {
        if let (Some(previous), Some(current)) = (previous_devices, current_devices)
            && previous != current
        {
            let _ = inner.events.send(ServiceEvent::Devices { total: current });
        }
        if previous_scan != current_scan {
            let _ = inner.events.send(ServiceEvent::CatalogScanFinished);
        }
    }
    Ok(())
}

/// Per-kind scan completion stamps, used only for change detection.
fn scan_fingerprint(scan: Option<&CatalogScanState>) -> (Option<i64>, Option<i64>) {
    match scan {
        Some(scan) => (scan.app, scan.game),
        None => (None, None),
    }
}

fn pairing_fingerprint(
    pairing: &Option<qinglink_protocol::service_ipc::PendingPairing>,
) -> Option<(String, String)> {
    pairing
        .as_ref()
        .map(|pairing| (pairing.request_id.clone(), pairing.pin.clone()))
}

pub(super) fn mark_disconnected(inner: &Arc<Inner>, error: &ServiceError) {
    // Read the previous state before overwriting it, so a persistent outage is
    // reported once rather than on every retry.
    let was_connected = inner
        .state
        .lock()
        .map(|state| state.connected)
        .unwrap_or(false);
    let link_state = match error {
        ServiceError::Unsupported(_) => ServiceLinkState::ProtocolMismatch,
        ServiceError::Unauthorized(_) => ServiceLinkState::Unauthorized,
        ServiceError::Unavailable(_) => ServiceLinkState::Unreachable,
        ServiceError::Timeout | ServiceError::Rejected { .. } | ServiceError::Internal(_) => {
            ServiceLinkState::Unreachable
        }
    };
    if let Ok(mut state) = inner.state.lock() {
        state.connected = false;
        state.link_state = link_state;
        state.capabilities = None;
        state.snapshot.pending_pairing = None;
        state.last_error = Some(error.to_string());
    }
    if was_connected {
        tracing::warn!(%error, state = ?link_state, "service IPC unavailable");
    }
    let _ = inner.events.send(ServiceEvent::Status);
}
