//! 交给每个管理端点与事件 socket 的共享状态。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use qinglink_protocol::service_ipc::PendingPairing;
use serde::Serialize;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::input_install::{InputInstallStatus, SharedInputInstallState};
use crate::logging::{LogEntry, LogSystem};
use crate::preferences::Preferences;
use crate::service::{ServiceClient, ServiceLinkState, ServiceStatus};
use crate::tray::RegistrationReport;
use crate::updater::{SharedUpdateState, UpdateView};

use super::error::ApiError;

/// Reply channel for a request that only the main thread can fulfil.
pub type HostReply = oneshot::Sender<Result<serde_json::Value, ApiError>>;

/// Actions that must run on the application main thread.
///
/// Restart/quit have to unwind the main loop rather than kill the process from
/// a request handler. 保持唤醒不在这里：它由服务进程统一持有，Web 层通过
/// 服务命令直接读写，主循环只按快照同步托盘勾选。
#[derive(Debug)]
pub enum HostCommand {
    /// Open the admin interface in the default browser.
    OpenUi { reply: HostReply },
    /// Restart the desktop process.
    Restart { reply: HostReply },
    /// Quit the desktop process, leaving the service running.
    Exit { reply: HostReply },
}

/// Settings exposed by the admin API.
///
/// 保持唤醒是服务快照的镜像，由 Web 事件泵写入；其余两项是桌面端本地设置。
/// 全部用原子量，任何线程都能无锁读取。
#[derive(Debug, Default)]
pub struct SharedSettings {
    autostart: AtomicBool,
    keep_awake: AtomicBool,
    debug_log: AtomicBool,
}

impl SharedSettings {
    /// Publish the current values.
    pub fn set(&self, autostart: bool, keep_awake: bool, debug_log: bool) {
        self.autostart.store(autostart, Ordering::Relaxed);
        self.keep_awake.store(keep_awake, Ordering::Relaxed);
        self.debug_log.store(debug_log, Ordering::Relaxed);
    }

    /// Current autostart state.
    pub fn autostart(&self) -> bool {
        self.autostart.load(Ordering::Relaxed)
    }

    /// 只更新保持唤醒镜像，不触碰另外两项设置。
    ///
    /// 服务快照、Web 端点与托盘线程都可能写这个字段；只写它自己的原子量，
    /// 不会把并发进行的 autostart / debug_log 更新覆盖掉。
    pub fn set_keep_awake(&self, enabled: bool) {
        self.keep_awake.store(enabled, Ordering::Relaxed);
    }

    /// Current keep-awake state.
    pub fn keep_awake(&self) -> bool {
        self.keep_awake.load(Ordering::Relaxed)
    }

    /// Current debug-logging state.
    pub fn debug_log(&self) -> bool {
        self.debug_log.load(Ordering::Relaxed)
    }

    /// Serialize for the settings endpoint.
    pub fn view(&self) -> SettingsView {
        SettingsView {
            autostart: self.autostart(),
            keep_awake: self.keep_awake(),
            debug_log: self.debug_log(),
        }
    }
}

/// Settings as presented to the admin UI.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SettingsView {
    pub autostart: bool,
    pub keep_awake: bool,
    pub debug_log: bool,
}

/// Install state of the secure-input service, as presented to the admin UI.
#[derive(Clone, Debug, Default, Serialize)]
pub struct InstallStateView {
    pub package_available: bool,
    pub service_installed: bool,
    pub service_running: bool,
    pub service_binary_matches: bool,
    /// The service's loopback health protocol answered.
    pub network_ready: bool,
    /// Whether the management link is usable.
    ///
    /// `Some(false)` means the service is installed and running but the desktop
    /// cannot use it (a protocol mismatch or a refused desktop identity). The
    /// install checks above cannot see that, which is why a healthy-looking card
    /// used to sit next to a "服务未连接" banner.
    pub service_protocol_ok: Option<bool>,
    pub available: bool,
    pub detail: String,
    pub busy: bool,
    pub result: Option<String>,
}

impl InstallStateView {
    /// Project the shared install state into its serializable form.
    ///
    /// `service_protocol_ok` is passed in rather than probed here: the live
    /// service client is the authority, and probing the pipe again would report
    /// the same thing twice.
    pub fn from_shared(
        shared: &SharedInputInstallState,
        service_protocol_ok: Option<bool>,
    ) -> Self {
        let state = shared.lock().map(|state| state.clone()).unwrap_or_default();
        Self::from_parts(
            &state.status,
            state.busy,
            state.result_message,
            service_protocol_ok,
        )
    }

    fn from_parts(
        status: &InputInstallStatus,
        busy: bool,
        result: Option<String>,
        service_protocol_ok: Option<bool>,
    ) -> Self {
        Self {
            package_available: status.package_available,
            service_installed: status.service_installed,
            service_running: status.service_running,
            service_binary_matches: status.service_binary_matches,
            network_ready: status.network_ready,
            service_protocol_ok,
            available: status.available,
            detail: status.detail.clone(),
            busy,
            result,
        }
    }
}

/// A live event for connected browsers.
///
/// One broadcast channel feeds every socket; the socket task maps variants onto
/// the documented `{channel, event, data}` envelope.
#[derive(Clone)]
pub enum ServerEvent {
    /// Service connection state or snapshot changed.
    Service(Box<ServiceStatus>),
    /// The pending pairing request changed.
    Pairing(Box<Option<PendingPairing>>),
    /// Secure-input install state changed.
    InstallState(Box<InstallStateView>),
    /// A coalesced batch of new log records.
    Logs(Arc<Vec<LogEntry>>),
    /// Records dropped by the log batcher because a subscriber was too slow.
    LogsDropped(u64),
    /// The catalog changed; `reason` is a stable token such as `rename`.
    CatalogChanged(&'static str),
    /// A catalog scan started.
    ScanStarted(&'static str),
    /// The active device set changed size.
    Devices { total: usize },
    /// Desktop settings changed, including a tray-initiated change.
    Settings(Box<SettingsView>),
    /// The in-memory log buffer was cleared.
    LogsCleared { cleared: u64 },
    /// A transient message for the user.
    Toast {
        level: &'static str,
        message: String,
    },
    /// 更新流程状态变化（检查/下载进度/就绪/失败）。
    UpdateState(Box<UpdateView>),
    /// The desktop is going away.
    Shutdown(&'static str),
}

/// Redacts the pairing PIN, which `PendingPairing` refuses to format.
impl std::fmt::Debug for ServerEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Service(status) => formatter.debug_tuple("Service").field(status).finish(),
            Self::Pairing(pairing) => formatter
                .debug_tuple("Pairing")
                .field(&pairing.as_ref().as_ref().map(|p| p.request_id.as_str()))
                .finish(),
            Self::InstallState(state) => {
                formatter.debug_tuple("InstallState").field(state).finish()
            }
            Self::Logs(entries) => formatter.debug_tuple("Logs").field(&entries.len()).finish(),
            Self::LogsDropped(count) => formatter.debug_tuple("LogsDropped").field(count).finish(),
            Self::CatalogChanged(reason) => formatter
                .debug_tuple("CatalogChanged")
                .field(reason)
                .finish(),
            Self::ScanStarted(kind) => formatter.debug_tuple("ScanStarted").field(kind).finish(),
            Self::Devices { total } => formatter.debug_tuple("Devices").field(total).finish(),
            Self::Settings(settings) => formatter.debug_tuple("Settings").field(settings).finish(),
            Self::LogsCleared { cleared } => {
                formatter.debug_tuple("LogsCleared").field(cleared).finish()
            }
            Self::Toast { level, message } => formatter
                .debug_struct("Toast")
                .field("level", level)
                .field("message", message)
                .finish(),
            Self::UpdateState(state) => formatter.debug_tuple("UpdateState").field(state).finish(),
            Self::Shutdown(reason) => formatter.debug_tuple("Shutdown").field(reason).finish(),
        }
    }
}

/// Everything an endpoint may need.
pub struct WebState {
    /// Service transport.
    pub service: ServiceClient,
    /// Log sinks.
    pub logs: Arc<LogSystem>,
    /// Desktop settings mirror.
    pub settings: Arc<SharedSettings>,
    /// Persistent desktop preferences.
    pub preferences: Preferences,
    /// Secure-input install state.
    pub install_state: SharedInputInstallState,
    /// 程序更新状态（检查/下载/安装）。
    pub update_state: SharedUpdateState,
    /// Channel to the application main thread.
    pub host: mpsc::UnboundedSender<HostCommand>,
    /// Fan-out for WebSocket clients.
    pub events: broadcast::Sender<ServerEvent>,
    /// Tray registration outcome, for the diagnostics panel.
    pub tray_registration: Mutex<Option<RegistrationReport>>,
    /// Whether a native file picker is already waiting for an answer.
    ///
    /// Only one may exist at a time: a second modal window would cover the first
    /// and leave the user with two prompts to dismiss.
    pub file_picker_open: AtomicBool,
    /// When the process started, not when the web server did.
    started_at: Instant,
    /// The port the admin interface is served on.
    port: u16,
}

impl std::fmt::Debug for WebState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebState")
            .field("port", &self.port)
            .field("uptime_secs", &self.uptime_secs())
            .field("settings", &self.settings.view())
            .finish_non_exhaustive()
    }
}

/// Everything [`WebState::new`] needs, bundled so the constructor keeps a
/// readable signature.
pub struct WebStateParts {
    /// Service transport.
    pub service: ServiceClient,
    /// Log sinks.
    pub logs: Arc<LogSystem>,
    /// Desktop settings mirror.
    pub settings: Arc<SharedSettings>,
    /// Persistent desktop preferences.
    pub preferences: Preferences,
    /// Secure-input install state.
    pub install_state: SharedInputInstallState,
    /// Channel to the application main thread.
    pub host: mpsc::UnboundedSender<HostCommand>,
    /// The port the admin interface is served on.
    pub port: u16,
    /// When the process started, not when the web server did.
    pub started_at: Instant,
}

impl WebState {
    /// Assemble the shared state.
    pub fn new(parts: WebStateParts) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            service: parts.service,
            logs: parts.logs,
            settings: parts.settings,
            preferences: parts.preferences,
            install_state: parts.install_state,
            update_state: SharedUpdateState::default(),
            host: parts.host,
            events,
            tray_registration: Mutex::new(None),
            file_picker_open: AtomicBool::new(false),
            started_at: parts.started_at,
            port: parts.port,
        }
    }

    /// Tray registration outcome, or `None` before the tray is created.
    pub fn tray_report(&self) -> Option<RegistrationReport> {
        self.tray_registration
            .lock()
            .ok()
            .and_then(|report| report.clone())
    }

    /// Record the tray registration outcome for the diagnostics panel.
    pub fn set_tray_report(&self, report: RegistrationReport) {
        if let Ok(mut slot) = self.tray_registration.lock() {
            *slot = Some(report);
        }
    }

    /// Seconds since the process started.
    ///
    /// Deliberately not measured from web-server creation: migration, the
    /// secure-input probe, the restart wait and the tray probe all run first, so
    /// that would report a few seconds of uptime for an hours-old process.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Port serving the admin interface.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Publish an event to connected browsers.
    pub fn publish(&self, event: ServerEvent) {
        // A send error only means nobody is connected.
        let _ = self.events.send(event);
    }

    /// Subscribe to live events.
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.events.subscribe()
    }

    /// Current install state.
    ///
    /// The management link state is folded in so the interface never reports a
    /// healthy service while the admin API says "服务未连接".
    pub fn install_view(&self) -> InstallStateView {
        InstallStateView::from_shared(&self.install_state, self.service_protocol_ok())
    }

    /// Whether the installed service speaks a protocol this desktop can use.
    ///
    /// `None` means "no answer yet" (not installed, not running, or still
    /// connecting), which the interface shows as unknown rather than broken.
    pub fn service_protocol_ok(&self) -> Option<bool> {
        match self.service.status().state {
            ServiceLinkState::Connected => Some(true),
            ServiceLinkState::ProtocolMismatch | ServiceLinkState::Unauthorized => Some(false),
            ServiceLinkState::Connecting | ServiceLinkState::Unreachable => None,
        }
    }

    /// Publish the current install state.
    pub fn publish_install_state(&self) {
        self.publish(ServerEvent::InstallState(Box::new(self.install_view())));
    }

    /// 发布当前更新状态到事件通道。
    pub fn publish_update_state(&self) {
        self.publish(ServerEvent::UpdateState(Box::new(self.update_state.view())));
    }
}
