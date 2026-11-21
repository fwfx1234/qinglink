//! 核心服务层。
//!
//! 提供 SQLite 持久化、配对管理与服务器设置。

pub mod app_catalog;
pub mod app_db;
pub mod app_scanner;
pub mod file_browser;
pub mod focus_monitor;
pub mod input_state_monitor;
pub mod process;
pub mod system;
pub mod window_monitor;

use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

use local_ip_address::local_ip;
use qinglink_storage::{AppPaths, database::DatabaseService};

use self::process::ProcessManager;
use self::system::SystemService;

/// 远程控制服务门面：统一持有数据目录、系统服务、进程管理器与配对状态。
#[derive(Clone)]
pub struct RemoteControlService {
    paths: AppPaths,
    system_service: Arc<SystemService>,
    process_manager: Arc<std::sync::Mutex<ProcessManager>>,
    inner: Arc<std::sync::Mutex<ServiceInner>>,
    pc_id: Uuid,
}

impl std::fmt::Debug for RemoteControlService {
    // 内部持有配对 PIN，禁止整体 derive；这里只输出非敏感的本机标识。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteControlService")
            .field("pc_id", &self.pc_id)
            .finish_non_exhaustive()
    }
}

const PAIRING_PIN_FILE: &str = "qinglink-pairing-pin";

/// Maximum consecutive failed PIN attempts before temporary lockout.
const PIN_MAX_ATTEMPTS: u32 = 5;
/// PIN lockout duration in seconds.
const PIN_LOCKOUT_SECS: u64 = 30;

#[derive(Default)]
struct ServiceInner {
    /// The current pairing PIN (6-digit). Empty if not in pairing mode.
    pairing_pin: Option<String>,
    /// Consecutive failed PIN attempts.
    pin_failed_attempts: u32,
    /// PIN lockout deadline (brute-force protection).
    pin_locked_until: Option<Instant>,
    /// Whether the HTTP server is running.
    server_running: bool,
    /// The port the server listens on.
    server_port: u16,
    /// Server auto-start on boot.
    auto_start: bool,
    /// Minimize to system tray on close.
    minimize_to_tray: bool,
    /// Local IP address.
    local_ip: String,
}

impl RemoteControlService {
    /// 创建服务实例：解析本机 IP、PC 标识，并恢复上次持久化的配对 PIN。
    pub fn new(paths: AppPaths) -> Self {
        let local_ip = Self::obtain_local_ip();
        let pairing_pin = Self::load_pairing_pin(&paths);
        let pc_id = Self::load_or_create_pc_id(&paths);
        Self {
            paths,
            system_service: Arc::new(SystemService::new()),
            process_manager: Arc::new(std::sync::Mutex::new(ProcessManager::new())),
            inner: Arc::new(std::sync::Mutex::new(ServiceInner {
                pairing_pin,
                pin_failed_attempts: 0,
                pin_locked_until: None,
                server_running: false,
                server_port: 3721,
                auto_start: false,
                minimize_to_tray: false,
                local_ip,
            })),
            pc_id,
        }
    }

    fn load_or_create_pc_id(paths: &AppPaths) -> Uuid {
        let path = paths.config("pc-id");
        if let Ok(value) = fs::read_to_string(&path)
            && let Ok(id) = Uuid::parse_str(value.trim())
        {
            return id;
        }
        let id = Uuid::new_v4();
        if let Err(error) = fs::write(path, id.to_string()) {
            tracing::warn!(%error, "failed to persist QingLink PC id");
        }
        id
    }

    /// 本机在 QingLink 中的唯一标识。
    pub fn pc_id(&self) -> Uuid {
        self.pc_id
    }

    /// 本机计算机名，取不到环境变量时回退为 `QingLink PC`。
    pub fn pc_name(&self) -> String {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "QingLink PC".to_string())
    }

    /// Obtain the local IP address.
    fn obtain_local_ip() -> String {
        local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string())
    }

    /// 数据目录布局，用于定位配置与数据库文件。
    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    /// 共享的系统状态与电源控制服务。
    pub fn system_service(&self) -> &SystemService {
        &self.system_service
    }

    /// 共享的进程管理器，使用前需先获取其内部互斥锁。
    pub fn process_manager(&self) -> &std::sync::Mutex<ProcessManager> {
        &self.process_manager
    }

    /// 生成新的 6 位配对 PIN 并持久化，同时清零失败计数与锁定状态。
    pub fn generate_pin(&self) -> String {
        let uuid = uuid::Uuid::new_v4();
        let bytes = uuid.as_bytes();
        let pin: String = (0..6)
            .map(|i| {
                let byte_index = bytes[i] % 10;
                (b'0' + byte_index) as char
            })
            .collect();
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.pairing_pin = Some(pin.clone());
        inner.pin_failed_attempts = 0;
        inner.pin_locked_until = None;
        drop(inner);
        self.persist_pairing_pin(Some(&pin));
        pin
    }

    /// 校验配对 PIN，失败会累计次数并在超过上限后临时锁定。
    pub fn check_pin(&self, pin: &str) -> bool {
        self.check_pin_inner(pin)
    }

    /// Validate the PIN and enforce attempt counting with lockout.
    ///
    /// After [`PIN_MAX_ATTEMPTS`] consecutive failures, the PIN is locked for
    /// [`PIN_LOCKOUT_SECS`] seconds. Even a correct PIN is rejected during
    /// the lockout window to prevent LAN brute-force attacks.
    fn check_pin_inner(&self, pin: &str) -> bool {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        if let Some(locked_until) = inner.pin_locked_until {
            if Instant::now() < locked_until {
                return false;
            }
            // Lockout expired; reset state.
            inner.pin_locked_until = None;
            inner.pin_failed_attempts = 0;
        }
        let matches = inner.pairing_pin.as_deref() == Some(pin);
        if matches {
            inner.pin_failed_attempts = 0;
        } else {
            inner.pin_failed_attempts = inner.pin_failed_attempts.saturating_add(1);
            if inner.pin_failed_attempts >= PIN_MAX_ATTEMPTS {
                inner.pin_locked_until =
                    Some(Instant::now() + Duration::from_secs(PIN_LOCKOUT_SECS));
                inner.pin_failed_attempts = 0;
                tracing::warn!(
                    "[远程控制] PIN 连续失败次数过多，已临时锁定 {} 秒",
                    PIN_LOCKOUT_SECS
                );
            }
        }
        matches
    }

    /// Test helper: clear the PIN lockout state.
    #[cfg(test)]
    pub(crate) fn clear_pin_lockout(&self) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.pin_locked_until = None;
        inner.pin_failed_attempts = 0;
    }

    /// 清除当前配对 PIN（配对成功后调用）。
    pub fn consume_pin(&self) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.pairing_pin = None;
        drop(inner);
        self.persist_pairing_pin(None);
    }

    /// 记录 HTTP 服务的运行状态与监听端口。
    pub fn set_server_running(&self, running: bool, port: u16) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.server_running = running;
        inner.server_port = port;
    }

    /// HTTP 服务当前是否正在运行。
    pub fn is_server_running(&self) -> bool {
        crate::lock_unpoisoned(&self.inner).server_running
    }

    /// HTTP 服务当前监听的端口。
    pub fn server_port(&self) -> u16 {
        crate::lock_unpoisoned(&self.inner).server_port
    }

    /// Get the current pairing PIN.
    pub fn get_current_pin(&self) -> Option<String> {
        crate::lock_unpoisoned(&self.inner).pairing_pin.clone()
    }

    fn load_pairing_pin(paths: &AppPaths) -> Option<String> {
        let path = paths.config(PAIRING_PIN_FILE);
        let pin = fs::read_to_string(path).ok()?.trim().to_string();
        if pin.len() == 6 && pin.bytes().all(|byte| byte.is_ascii_digit()) {
            Some(pin)
        } else {
            None
        }
    }

    fn persist_pairing_pin(&self, pin: Option<&str>) {
        let path = self.paths.config(PAIRING_PIN_FILE);
        let result = match pin {
            Some(pin) => fs::write(path, pin),
            None => match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            },
        };
        if let Err(error) = result {
            tracing::warn!(%error, "[远程控制] 持久化配对 PIN 失败");
        }
    }

    /// Get the local IP address.
    pub fn local_ip(&self) -> String {
        crate::lock_unpoisoned(&self.inner).local_ip.clone()
    }

    /// Get the current server settings.
    pub fn get_settings(&self) -> crate::protocol::responses::ServerSettings {
        let inner = crate::lock_unpoisoned(&self.inner);
        crate::protocol::responses::ServerSettings {
            port: inner.server_port,
            auto_start: inner.auto_start,
            minimize_to_tray: inner.minimize_to_tray,
        }
    }

    /// Update the server settings.
    pub fn update_settings(&self, req: &crate::protocol::requests::UpdateSettingsRequest) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        if let Some(port) = req.port {
            inner.server_port = port;
        }
        if let Some(auto_start) = req.auto_start {
            inner.auto_start = auto_start;
        }
        if let Some(minimize_to_tray) = req.minimize_to_tray {
            inner.minimize_to_tray = minimize_to_tray;
        }
    }

    /// Revoke a device pairing.
    pub fn revoke_device(&self, name: &str, database: &DatabaseService) -> bool {
        let key = "qinglink/data";
        let connection = match database.connection(key) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let affected = connection
            .execute("DELETE FROM tokens_v2 WHERE device_name = ?1", [name])
            .unwrap_or(0);
        affected > 0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::RemoteControlService;
    use qinglink_storage::AppPaths;

    #[test]
    fn consume_pin_invalidates_the_active_pairing_pin() {
        let temp_dir = tempfile::tempdir().expect("create temporary data directory");
        let service = RemoteControlService::new(AppPaths::for_test(temp_dir.path()));

        let pin = service.generate_pin();
        assert!(service.check_pin(&pin));

        service.consume_pin();

        assert_eq!(service.get_current_pin(), None);
        assert!(!service.check_pin(&pin));
    }

    #[test]
    fn pairing_pin_survives_service_restart() {
        let temp_dir = tempfile::tempdir().expect("create temporary data directory");
        let paths = AppPaths::for_test(temp_dir.path());
        let service = RemoteControlService::new(paths.clone());
        let pin = service.generate_pin();
        drop(service);

        let restarted_service = RemoteControlService::new(paths);
        assert_eq!(restarted_service.get_current_pin(), Some(pin.clone()));
        assert!(restarted_service.check_pin(&pin));
    }

    #[test]
    fn pairing_pin_remains_valid_after_verification() {
        let temp_dir = tempfile::tempdir().expect("create temporary data directory");
        let service = RemoteControlService::new(AppPaths::for_test(temp_dir.path()));
        let pin = service.generate_pin();

        assert!(service.check_pin(&pin));
        assert!(service.check_pin(&pin));
    }

    #[test]
    fn pin_is_locked_after_repeated_failures() {
        let temp_dir = tempfile::tempdir().expect("create temporary data directory");
        let service = RemoteControlService::new(AppPaths::for_test(temp_dir.path()));
        let pin = service.generate_pin();

        // 连续失败达到上限
        for _ in 0..5 {
            assert!(!service.check_pin("000000"));
        }
        // 锁定期间即使输入正确 PIN 也会被拒绝
        assert!(!service.check_pin(&pin));
    }

    #[test]
    fn pin_lockout_is_released_after_timeout() {
        let temp_dir = tempfile::tempdir().expect("create temporary data directory");
        let service = RemoteControlService::new(AppPaths::for_test(temp_dir.path()));
        let pin = service.generate_pin();

        for _ in 0..5 {
            assert!(!service.check_pin("000000"));
        }
        assert!(!service.check_pin(&pin));

        // 模拟锁定超时后恢复
        service.clear_pin_lockout();
        assert!(service.check_pin(&pin));
    }

    #[test]
    fn generating_a_new_pin_resets_the_lockout() {
        let temp_dir = tempfile::tempdir().expect("create temporary data directory");
        let service = RemoteControlService::new(AppPaths::for_test(temp_dir.path()));
        let pin = service.generate_pin();

        for _ in 0..5 {
            assert!(!service.check_pin("000000"));
        }
        assert!(!service.check_pin(&pin));

        let new_pin = service.generate_pin();
        assert!(service.check_pin(&new_pin));
        assert_ne!(pin, new_pin);
    }
}
