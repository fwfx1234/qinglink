//! 服务端共享状态：`AppState` 的定义与构造。
//!
//! 输入门控需要读写本类型的私有输入字段，因此放在 `state/input_gate.rs`
//! 作为子模块；只依赖公开字段的行为（窗口缓存、目录扫描）放在同级模块。

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use qinglink_domain::PairingManager;
use qinglink_storage::database::DatabaseService;
use tokio::sync::broadcast;

use crate::protocol::requests::WsEvent;
use crate::server::auth;
use crate::server::connection_registry;
use crate::server::control;
use crate::server::held_input;
use crate::server::launch_display::LaunchDisplayCoordinator;
use crate::server::pairing;
use crate::server::process_icon::ProcessIconCache;
use crate::service::RemoteControlService;
use crate::service::app_scanner::AppScanner;
use crate::service::process::ProcessManager;
use crate::service::system::SystemService;
use crate::service::window_monitor::WindowMonitor;

mod input_gate;

/// 向已连接客户端广播服务端事件的发送端。
pub type EventSender = broadcast::Sender<WsEvent>;
type WindowListCache = Option<(Instant, Vec<crate::protocol::requests::WindowInfo>)>;

/// 服务端进程内共享状态：控制标志、服务句柄、连接注册表与事件通道。
// 内部字段多是不支持 Debug 的服务句柄，且含令牌存储，故不实现 Debug。
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct AppState {
    /// 暂停开关与输入门闩等全局控制状态。
    pub control: Arc<control::ControlState>,
    /// 记录已成功按下的按键与鼠标键，供暂停或断线时释放。
    pub held_input: Arc<Mutex<held_input::HeldInput>>,
    /// 与桌面端共享的远程控制服务句柄。
    pub service: Arc<RemoteControlService>,
    /// 提供系统信息、电源与网络唤醒能力的服务。
    pub system_service: Arc<SystemService>,
    /// 进程查询、挂起与结束操作的执行器。
    pub process_manager: Arc<std::sync::Mutex<ProcessManager>>,
    /// 设备令牌的持久化存储与校验缓存。
    pub token_store: Arc<auth::TokenStore>,
    /// 向所有已认证连接广播事件的发送端。
    pub events: EventSender,
    /// 应用与游戏目录扫描器。
    pub app_scanner: Arc<AppScanner>,
    /// 窗口列表与游戏运行状态监视器。
    pub window_monitor: Arc<std::sync::Mutex<WindowMonitor>>,
    /// 控制（事件）通道的 WebSocket 连接注册表。
    pub connection_registry: Arc<connection_registry::ConnectionRegistry>,
    /// 输入通道的 WebSocket 连接注册表。
    pub input_connection_registry: Arc<connection_registry::ConnectionRegistry>,
    /// 向输入工作线程投递鼠标相对位移的通道。
    pub input_move_tx: tokio::sync::watch::Sender<(u64, i32, i32)>,
    input_worker_started: Arc<AtomicBool>,
    input_generation: Arc<std::sync::atomic::AtomicU64>,
    /// 应用目录、图标与设置的持久化数据库。
    pub app_db: Arc<std::sync::Mutex<crate::service::app_db::AppDatabase>>,
    /// Serializes explicit catalog scans.  The WebSocket handlers run these
    /// filesystem-heavy jobs off the async reactor, but they must not compete
    /// with one another for disk bandwidth.
    pub scan_gate: Arc<Mutex<()>>,
    /// 窗口列表的短期缓存，避免频繁枚举系统窗口。
    pub window_list_cache: Arc<Mutex<WindowListCache>>,
    /// 进程图标的路径级缓存：同一程序只解析一次。
    pub process_icon_cache: Arc<Mutex<ProcessIconCache>>,
    /// 配对请求状态机，由互斥锁保护。
    pub pairing: Arc<Mutex<PairingManager>>,
    /// 游戏启动显示模式协调器，负责切换、进程跟踪与恢复。
    pub launch_display: LaunchDisplayCoordinator,
    /// 向桌面端广播配对请求变化的发送端。
    pub pairing_events: broadcast::Sender<pairing::PairingNotification>,
}

impl AppState {
    /// 依据已注册的数据库构造服务端状态，并恢复缓存的目录与暂停设置。
    pub fn new(service: RemoteControlService, database: Arc<DatabaseService>) -> Self {
        let (tx, _) = broadcast::channel(256);
        let (pairing_events, _) = broadcast::channel(32);
        let scanner_config_path = service.paths().config("scanner_custom_dirs.json");
        let mut window_monitor = WindowMonitor::new();
        window_monitor.set_event_sender(tx.clone());
        let token_store = Arc::new(auth::TokenStore::new(&database));
        let (input_move_tx, _) = tokio::sync::watch::channel((0_u64, 0_i32, 0_i32));
        // Clean up expired tokens.
        token_store.cleanup();
        let app_db = Arc::new(std::sync::Mutex::new(
            crate::service::app_db::AppDatabase::new(database),
        ));
        // Restore the persisted directory before accepting requests.  A fresh
        // process must not make the first mobile list request scan the disk.
        let cached_apps = app_db
            .lock()
            .ok()
            .and_then(|db| db.get_visible_apps().ok())
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect();
        let game_scan_completed = app_db
            .lock()
            .ok()
            .and_then(|db| db.scan_completed("game").ok())
            .unwrap_or(false);
        let cached_steam_games = if game_scan_completed {
            app_db
                .lock()
                .ok()
                .and_then(|db| db.get_steam_games().ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let app_scanner = Arc::new(AppScanner::with_cached_apps(
            scanner_config_path,
            cached_apps,
        ));
        // An empty Steam table is not a useful cache marker: it can mean the
        // previous scan ran before Steam was detected. Let the scanner probe
        // again so installed Steam libraries are not permanently skipped.
        if game_scan_completed && !cached_steam_games.is_empty() {
            app_scanner.replace_steam_games(cached_steam_games);
        }
        let control = Arc::new(control::ControlState::default());
        let paused = app_db
            .lock()
            .ok()
            .and_then(|db| db.get_setting("service.paused").ok().flatten())
            .is_some_and(|value| value == "true");
        control.set_paused(paused);
        let state = Self {
            control,
            held_input: Arc::new(Mutex::new(held_input::HeldInput::default())),
            service: Arc::new(service),
            system_service: Arc::new(SystemService::new()),
            process_manager: Arc::new(std::sync::Mutex::new(ProcessManager::new())),
            token_store,
            events: tx,
            app_scanner,
            window_monitor: Arc::new(std::sync::Mutex::new(window_monitor)),
            connection_registry: Arc::new(connection_registry::ConnectionRegistry::new()),
            input_connection_registry: Arc::new(connection_registry::ConnectionRegistry::new()),
            input_move_tx,
            input_worker_started: Arc::new(AtomicBool::new(false)),
            input_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            app_db,
            scan_gate: Arc::new(Mutex::new(())),
            window_list_cache: Arc::new(Mutex::new(None)),
            process_icon_cache: Arc::new(Mutex::new(ProcessIconCache::default())),
            pairing: Arc::new(Mutex::new(PairingManager::default())),
            pairing_events,
            launch_display: LaunchDisplayCoordinator::default(),
        };
        state.launch_display.recover(&state);
        state
    }
}
