//! 启动装配：把数据目录、日志、数据库、Web、托盘与配对提示弹窗拼成一个可运行的桌面端。
//!
//! 这里的调用顺序是契约，见 `docs/architecture.md` §9.1：数据目录 → 日志 →
//! self-check → 等待重启父进程 → 单实例 → 会话检查 → 数据库迁移/注册 →
//! Preferences → ServiceRuntime → Web → 托盘 → 配对提示弹窗 → 服务租约。调整顺序必须同步该文档。
//!
//! 启动路径上不允许无期限阻塞（`docs/development.md`）；每次新增启动期 IO 都要
//! 给它一个期限，或者挪出启动路径。

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use qinglink_storage::{
    AppPaths,
    database::{DatabaseService, DatabaseSpec},
};
use qinglink_windows::single_instance::{
    SingleInstance, activate_existing, wait_for_restart_parent,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::admin_ui::{WEB_PORT, open_browser, web_url};
use crate::input_install::{self, SharedInputInstallState};
use crate::logging::{self, LogSystem};
use crate::pairing::{PairingPrompt, PromptEvent};
use crate::preferences::Preferences;
use crate::self_check;
use crate::service::{ServiceClient, ServiceRuntime};
use crate::service_lifecycle::ServiceLifecycleManager;
use crate::startup::StartupOptions;
use crate::startup_trace::trace_step;
use crate::tray::{TrayController, TrayEvent};
use crate::web;
use crate::web::state::{HostCommand, SharedSettings, WebState, WebStateParts};

/// 启动前半程的中间产物：日志、数据库与单实例都已经就位。
struct Environment {
    logs: Arc<LogSystem>,
    database: Arc<DatabaseService>,
    preferences: Preferences,
    debug_logging: bool,
    /// 偏好里的保持唤醒意图；是否真的恢复成运行期请求由 [`restore_keep_awake`] 决定。
    keep_awake: bool,
    instance: SingleInstance,
}

/// Web 装配结果：共享状态、主线程命令通道与 Web 线程句柄。
struct WebStart {
    state: Arc<WebState>,
    host_rx: UnboundedReceiver<HostCommand>,
    thread: std::thread::JoinHandle<()>,
}

/// 装配完成、可以进入主循环的桌面端。
pub struct Desktop {
    /// 日志系统；退出序列里最后显式 drop 它。
    pub logs: Arc<LogSystem>,
    /// 桌面端设置镜像；托盘与 HostCommand 都写它。
    pub settings: Arc<SharedSettings>,
    /// Web 管理面共享状态。
    pub web_state: Arc<WebState>,
    /// 服务通道客户端；主循环只用它读状态。
    pub client: ServiceClient,
    /// 托盘控制器；创建失败时为 `None`，界面退回浏览器。
    pub tray: Option<TrayController>,
    /// 托盘事件通道。
    pub tray_rx: UnboundedReceiver<TrayEvent>,
    /// 配对提示弹窗；创建失败时为 `None`，配对码退回浏览器界面。
    pub prompt: Option<PairingPrompt>,
    /// 弹窗事件通道（取消失败、用户关闭窗口）。
    pub prompt_rx: UnboundedReceiver<PromptEvent>,
    /// 来自 HTTP 处理器、必须落在 main 线程的命令通道。
    pub host_rx: UnboundedReceiver<HostCommand>,
    /// 单实例互斥体；必须持有到进程结束。
    pub instance: SingleInstance,
    /// 服务租约线程。
    pub lifecycle: ServiceLifecycleManager,
    /// 服务 IPC 传输。
    pub runtime: ServiceRuntime,
    /// 桌面端自己的数据库。
    pub database: Arc<DatabaseService>,
    /// 是否已经打开过浏览器兜底，主循环用它避免重复弹窗。
    pub browser_fallback_opened: bool,
    /// Web 线程句柄，随进程存活；只负责不让线程句柄被立刻丢弃。
    pub _web_thread: std::thread::JoinHandle<()>,
}

/// 执行启动序列，返回可进入主循环的桌面端。
///
/// 返回 `None` 表示这次启动不该继续：已有实例在运行（已激活对方），或某个
/// 先于装配执行的诊断命令已经完成。
pub fn start(options: &StartupOptions, started_at: Instant) -> Result<Option<Desktop>> {
    let Some(environment) = prepare(options, started_at)? else {
        return Ok(None);
    };
    assemble(environment, options, started_at).map(Some)
}

/// 数据目录、日志、诊断、重启等待、单实例、会话、数据库与设置。
fn prepare(options: &StartupOptions, started_at: Instant) -> Result<Option<Environment>> {
    let paths = AppPaths::resolve()?;
    trace_step(started_at, "data directory resolved");

    // Install the console layer before opening SQLite. This keeps
    // `cargo run` useful when a later startup step fails, while the same
    // records continue to be written to the rotating log file.
    let log_level_from_environment = logging::level_override_from_environment().is_some();
    let logs = Arc::new(LogSystem::initialize(&paths, log_level_from_environment)?);
    trace_step(started_at, "logs initialised");

    // Diagnostics run before any subsystem is started, so they work while the
    // application is already running and cannot fail for unrelated reasons.
    if options.self_check {
        trace_step(started_at, "running the self check");
        self_check::run(&paths)?;
        return Ok(None);
    }

    // A restart replacement must wait before claiming the instance. A normal
    // second launch is checked before database/service/Web initialization so it
    // only signals the owner and exits.
    if let Some(parent) = options.restart_parent {
        wait_for_restart_parent(parent)?;
        trace_step(started_at, "restart parent exited");
    }
    let Some(instance) = SingleInstance::acquire()? else {
        tracing::info!("已有实例在运行，激活并退出");
        activate_existing()?;
        return Ok(None);
    };
    trace_step(started_at, "single instance acquired");
    qinglink_windows::session::ensure_supported()?;
    trace_step(started_at, "session checked");

    let database = Arc::new(DatabaseService::new(paths.clone()));
    database.migrate_application_database()?;
    trace_step(started_at, "application database migrated");
    database.register_database(DatabaseSpec::app("qinglink/data", "qinglink.db"))?;
    trace_step(started_at, "application database registered");

    let preferences = Preferences::new(Arc::clone(&database));
    // An explicit environment override wins, so a developer can always force
    // detail from the shell without touching persisted settings.
    let debug_logging = if log_level_from_environment {
        true
    } else {
        preferences.debug_logging()
    };
    if !log_level_from_environment {
        logs.set_debug(debug_logging);
    }
    // 保持唤醒只读意图；实际请求要等装配阶段在 main 线程上恢复。
    let keep_awake = preferences.keep_awake();
    trace_step(started_at, "preferences loaded");

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        startup = options.startup,
        debug_logging,
        keep_awake,
        data_dir = %paths.data_dir().display(),
        integrity = %qinglink_windows::integrity::current_integrity_name(),
        shell_integrity = %qinglink_windows::integrity::shell_integrity_name(),
        "QingLink 桌面端启动"
    );

    Ok(Some(Environment {
        logs,
        database,
        preferences,
        debug_logging,
        keep_awake,
        instance,
    }))
}

/// 服务通道、Web、托盘与服务租约。
fn assemble(
    environment: Environment,
    options: &StartupOptions,
    started_at: Instant,
) -> Result<Desktop> {
    // --- Service transport -------------------------------------------------
    let runtime = ServiceRuntime::start();
    let client = runtime.client();
    trace_step(started_at, "service transport started");

    // --- Settings ----------------------------------------------------------
    let settings = Arc::new(SharedSettings::default());
    let autostart = qinglink_windows::autostart::is_enabled().unwrap_or(false);
    // 保持唤醒随偏好跨重启恢复：`SetThreadExecutionState` 是 per-thread 状态，
    // 必须在这里（main 线程，也是随后主循环所在的线程）立刻应用。
    let keep_awake = restore_keep_awake(environment.keep_awake);
    settings.set(autostart, keep_awake, environment.debug_logging);

    // --- Secure-input install state ---------------------------------------
    let install_state = input_install::shared_state();
    trace_step(started_at, "install state probing started");

    // --- Web service -------------------------------------------------------
    let web_start = start_web(
        &client,
        &settings,
        &install_state,
        environment.preferences.clone(),
        &environment.logs,
        started_at,
    )?;

    // --- Tray --------------------------------------------------------------
    let (tray, tray_rx, browser_fallback_opened) =
        create_tray(&web_start.state, keep_awake, started_at);

    // --- Pairing prompt ----------------------------------------------------
    let (prompt, prompt_rx) = create_prompt(started_at);

    // --- Service lifecycle -------------------------------------------------
    let mut lifecycle = ServiceLifecycleManager::new();
    lifecycle.start(options.startup);
    trace_step(started_at, "service lease thread started");

    tracing::info!(url = %web_url(), "QingLink 已就绪");
    trace_step(started_at, "entering the main loop");

    Ok(Desktop {
        logs: environment.logs,
        settings,
        web_state: web_start.state,
        client,
        tray,
        tray_rx,
        prompt,
        prompt_rx,
        host_rx: web_start.host_rx,
        instance: environment.instance,
        lifecycle,
        runtime,
        database: environment.database,
        browser_fallback_opened,
        _web_thread: web_start.thread,
    })
}

/// 装配 WebState 并拉起 Web 线程；路由构建失败或端口被占用都是硬失败。
fn start_web(
    client: &ServiceClient,
    settings: &Arc<SharedSettings>,
    install_state: &SharedInputInstallState,
    preferences: Preferences,
    logs: &Arc<LogSystem>,
    started_at: Instant,
) -> Result<WebStart> {
    let (host_tx, host_rx) = web::host_channel();
    let web_state = Arc::new(WebState::new(WebStateParts {
        service: client.clone(),
        logs: Arc::clone(logs),
        settings: Arc::clone(settings),
        preferences,
        install_state: Arc::clone(install_state),
        host: host_tx,
        port: WEB_PORT,
        started_at,
    }));
    trace_step(started_at, "web state assembled");
    let thread = web::run_on_thread(WEB_PORT, Arc::clone(&web_state))?;
    trace_step(started_at, "web listener reporting ready");
    Ok(WebStart {
        state: web_state,
        host_rx,
        thread,
    })
}

/// 创建配对提示窗口；失败时降级为"配对码只在管理界面显示"。
///
/// 与本文件其它启动步骤一样，这里不允许无期限阻塞：建窗只是几个 Win32 调用。
/// 失败不是硬错误——没有这个窗口，配对仍然可以在浏览器里完成。
fn create_prompt(started_at: Instant) -> (Option<PairingPrompt>, UnboundedReceiver<PromptEvent>) {
    match PairingPrompt::create() {
        Ok((prompt, receiver)) => {
            trace_step(started_at, "pairing prompt created");
            (Some(prompt), receiver)
        }
        Err(error) => {
            tracing::warn!(%error, "配对提示窗口创建失败，配对码只能在管理界面查看");
            let (_, receiver) = tokio::sync::mpsc::unbounded_channel();
            (None, receiver)
        }
    }
}

/// 按偏好恢复保持唤醒，返回本次启动真实生效的状态。
///
/// 必须在 main 线程调用：`SetThreadExecutionState` 是 per-thread 状态，而承载它的
/// 就是随后的主循环线程。失败只降级为"开关关闭"，不影响启动。
fn restore_keep_awake(persisted: bool) -> bool {
    if !persisted {
        return false;
    }
    match qinglink_windows::power::set_keep_awake(true) {
        Ok(()) => {
            tracing::info!("已按偏好恢复保持唤醒");
            true
        }
        Err(error) => {
            tracing::warn!(%error, "恢复保持唤醒失败，本次启动按关闭处理");
            false
        }
    }
}

/// 创建托盘图标，并在 shell 永久拒绝时立刻改用浏览器兜底。
///
/// 返回值里的布尔量表示"已经打开过兜底浏览器"：主循环的重试逻辑要用它避免重复弹窗。
fn create_tray(
    web_state: &Arc<WebState>,
    keep_awake: bool,
    started_at: Instant,
) -> (Option<TrayController>, UnboundedReceiver<TrayEvent>, bool) {
    let (tray, tray_rx) = match TrayController::create(keep_awake) {
        Ok((tray, receiver)) => {
            web_state.set_tray_report(tray.registration());
            (Some(tray), receiver)
        }
        Err(error) => {
            // Losing the tray costs the user their shortcut, not the product:
            // the interface is still reachable from a browser.
            tracing::error!(%error, "托盘图标创建失败，改为自动打开浏览器");
            let (_, receiver) = tokio::sync::mpsc::unbounded_channel();
            (None, receiver)
        }
    };
    trace_step(started_at, "tray registration attempted");
    // A transiently rejected NIM_ADD is retried by the tray controller while
    // Explorer finishes startup. A rejection the shell will never accept — a
    // process that sits below the notification area's integrity level — is
    // reported and answered with the browser fallback right away instead of
    // burning the retry budget first.
    let mut browser_fallback_opened = false;
    if tray
        .as_ref()
        .is_some_and(|tray| tray.registration().is_blocked())
    {
        browser_fallback_opened = true;
        tracing::error!("通知区域不可用（完整性级别不匹配），改为自动打开浏览器");
        if let Err(error) = open_browser() {
            tracing::error!(%error, "托盘不可用且浏览器兜底失败，请手动访问管理界面");
        }
    } else if tray.is_none() {
        browser_fallback_opened = true;
        if let Err(error) = open_browser() {
            tracing::error!(%error, "托盘不可用且浏览器兜底失败，请手动访问管理界面");
        }
    }
    (tray, tray_rx, browser_fallback_opened)
}
