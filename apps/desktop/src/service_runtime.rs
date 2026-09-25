use std::sync::{Arc, Mutex};

use anyhow::Result;
use qinglink_server::{AppState, server::pairing::PairingNotification};
use tokio::sync::{broadcast, watch};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesiredServiceState {
    Running,
    Paused,
    Shutdown,
}

#[derive(Clone, Debug, Default)]
pub struct ServiceStatus {
    pub running: bool,
    pub address: Option<String>,
    pub error: Option<String>,
}

pub struct ServiceRuntime {
    pub desired: watch::Sender<DesiredServiceState>,
    pub status: Arc<Mutex<ServiceStatus>>,
}

impl ServiceRuntime {
    pub fn start(
        state: AppState,
        pairing_events: broadcast::Receiver<PairingNotification>,
        pairing_snapshot: Arc<Mutex<Option<PairingNotification>>>,
    ) -> Self {
        let (desired, _) = watch::channel(DesiredServiceState::Running);
        let status = Arc::new(Mutex::new(ServiceStatus {
            running: crate::input_install::service_network_ready(),
            address: Some("service://QingLinkService".into()),
            error: None,
        }));
        let _ = (state, pairing_events, pairing_snapshot);
        Self { desired, status }
    }

    pub fn shutdown(self) -> Result<()> {
        let _ = self.desired.send(DesiredServiceState::Shutdown);
        Ok(())
    }
}
