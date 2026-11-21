//! 链路状态、快照视图与事件类型。
//!
//! 本模块只描述“桌面端看到了什么”，不涉及任何管道 I/O。

use std::time::Instant;

use qinglink_protocol::service_ipc::{
    Capabilities, DesktopCommand, SERVICE_IPC_VERSION, ServiceSnapshot,
};

/// How the desktop↔service link currently stands.
///
/// The variants exist so the admin UI can distinguish three failures that all
/// present as "not connected" but need opposite responses: an unreachable pipe
/// is an environment or install problem, a protocol mismatch needs a service
/// upgrade, and a refused desktop identity needs the service to re-register the
/// desktop executable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceLinkState {
    /// No snapshot has been received yet and no failure was recorded.
    #[default]
    Connecting,
    /// A snapshot was received recently enough to trust.
    Connected,
    /// The pipe answered, but with a protocol version this desktop cannot use.
    ProtocolMismatch,
    /// The service refused this desktop executable's identity.
    Unauthorized,
    /// The pipe could not be reached at all.
    Unreachable,
}

/// Coarse service state, refreshed from the periodic snapshot.
#[derive(Clone, Default)]
pub struct ServiceState {
    /// A snapshot was received recently enough to trust.
    pub connected: bool,
    /// Classified link state, kept in step with `connected`.
    pub link_state: ServiceLinkState,
    /// Protocol version the service reported, once it answered at all.
    pub service_version: Option<u32>,
    /// Negotiated command support, present only while connected.
    pub capabilities: Option<Capabilities>,
    /// Most recent authoritative snapshot.
    pub snapshot: ServiceSnapshot,
    /// Why the last attempt failed, cleared on success.
    pub last_error: Option<String>,
    /// When `snapshot` was received.
    pub snapshot_at: Option<Instant>,
}

/// Redacts the pairing PIN.
///
/// `PendingPairing` deliberately does not implement `Debug`, and this type is
/// reachable from `tracing` call sites, so the PIN must never be formatted.
impl std::fmt::Debug for ServiceState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServiceState")
            .field("connected", &self.connected)
            .field("link_state", &self.link_state)
            .field("service_version", &self.service_version)
            .field("capabilities", &self.capabilities)
            .field("paused", &self.snapshot.paused)
            .field("network_ready", &self.snapshot.network_ready)
            .field("address", &self.snapshot.address)
            .field("last_error", &self.last_error)
            .field(
                "pending_pairing",
                &self
                    .snapshot
                    .pending_pairing
                    .as_ref()
                    .map(|pairing| pairing.request_id.as_str()),
            )
            .field("snapshot_at", &self.snapshot_at)
            .finish()
    }
}

impl ServiceState {
    /// Whether a command may be attempted right now.
    pub(super) fn supports(&self, command: &DesktopCommand) -> bool {
        if !self.connected {
            return false;
        }
        let Some(capabilities) = &self.capabilities else {
            return false;
        };
        if !capabilities.management {
            return false;
        }
        match command {
            DesktopCommand::Capabilities | DesktopCommand::GetSnapshot { .. } => true,
            DesktopCommand::SetPaused { .. } => capabilities.pause,
            DesktopCommand::CancelPairing { .. } => capabilities.pairing,
            DesktopCommand::ListDevices { .. } | DesktopCommand::RevokeDevice { .. } => {
                capabilities.devices
            }
            DesktopCommand::GetCatalog { .. } => capabilities.catalog,
            DesktopCommand::ListApps { .. }
            | DesktopCommand::GetApp { .. }
            | DesktopCommand::RenameApp { .. }
            | DesktopCommand::SetAppHidden { .. }
            | DesktopCommand::SetAppFavorite { .. }
            | DesktopCommand::SetAppPinned { .. }
            | DesktopCommand::DeleteApp { .. }
            | DesktopCommand::RescanApps { .. }
            | DesktopCommand::GetAppIcon { .. }
            | DesktopCommand::GetCatalogStats => capabilities.catalog_management,
            DesktopCommand::RefreshAppIcon { .. } | DesktopCommand::ListAppIcons { .. } => {
                capabilities.app_icon_manage
            }
            DesktopCommand::AddApp { .. } => capabilities.app_add,
            DesktopCommand::LaunchApp { .. } => capabilities.app_launch,
            DesktopCommand::SetKeepAwake { .. } => capabilities.keep_awake,
        }
    }

    /// Snapshot for the admin API.
    pub fn view(&self) -> ServiceStatus {
        ServiceStatus {
            connected: self.connected,
            state: self.link_state,
            expected_version: SERVICE_IPC_VERSION,
            service_version: self.service_version,
            service_app_version: self.snapshot.app_version.clone(),
            paused: self.snapshot.paused,
            network_ready: self.snapshot.network_ready,
            address: self.snapshot.address.clone(),
            last_error: self
                .last_error
                .clone()
                .or_else(|| self.snapshot.last_error.clone()),
            capabilities: self.capabilities.clone(),
        }
    }
}

/// Service status as presented to the admin UI.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ServiceStatus {
    /// Whether the management link is usable. Kept next to `state` so existing
    /// clients that only know this field keep working.
    pub connected: bool,
    /// Classified link state; the authoritative discriminator for the UI.
    pub state: ServiceLinkState,
    /// Protocol version this desktop speaks.
    pub expected_version: u32,
    /// Protocol version the service reported, if it answered at all.
    pub service_version: Option<u32>,
    /// 服务端产品版本号（如 `0.1.0+13`）；旧服务不返回时为 `None`。
    ///
    /// 与 [`Self::service_version`] 区分：那个是本地管理协议的版本，这个是服务端
    /// 可执行文件自身的版本，用于确认"桌面端与服务端是不是同一次构建"。
    pub service_app_version: Option<String>,
    pub paused: bool,
    pub network_ready: bool,
    pub address: Option<String>,
    pub last_error: Option<String>,
    pub capabilities: Option<Capabilities>,
}

/// Changes worth pushing to connected browsers.
#[derive(Clone)]
pub enum ServiceEvent {
    /// Connection state or snapshot changed.
    Status,
    /// A pairing request appeared, changed, or expired.
    Pairing(Option<qinglink_protocol::service_ipc::PendingPairing>),
    /// The set of active device tokens changed size.
    Devices { total: usize },
    /// A background catalog scan finished.
    CatalogScanFinished,
    /// 服务进程的保持唤醒开关变化，携带当前生效值。
    KeepAwake(bool),
}

/// Redacts the pairing PIN.
impl std::fmt::Debug for ServiceEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Status => formatter.write_str("ServiceEvent::Status"),
            Self::Pairing(pairing) => formatter
                .debug_tuple("ServiceEvent::Pairing")
                .field(&pairing.as_ref().map(|pairing| pairing.request_id.as_str()))
                .finish(),
            Self::Devices { total } => formatter
                .debug_struct("ServiceEvent::Devices")
                .field("total", total)
                .finish(),
            Self::CatalogScanFinished => formatter.write_str("ServiceEvent::CatalogScanFinished"),
            Self::KeepAwake(enabled) => formatter
                .debug_tuple("ServiceEvent::KeepAwake")
                .field(enabled)
                .finish(),
        }
    }
}
