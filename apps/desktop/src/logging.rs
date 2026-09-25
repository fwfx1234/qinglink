use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use qinglink_storage::AppPaths;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, reload, util::SubscriberInitExt};

/// 运行时可切换的日志级别开关。
///
/// 初始化后由主进程持有（通过 [`initialize`] 的返回值），并克隆给 UI，
/// 让“服务设置”页的调试日志开关能在不重启程序的情况下在
/// `qinglink=info` 与 `qinglink=debug` 之间切换。
#[derive(Clone)]
pub struct LogLevelSwitch {
    debug: Arc<AtomicBool>,
    apply: Arc<dyn Fn(bool) + Send + Sync>,
}

impl LogLevelSwitch {
    fn new(debug: bool, apply: Arc<dyn Fn(bool) + Send + Sync>) -> Self {
        Self {
            debug: Arc::new(AtomicBool::new(debug)),
            apply,
        }
    }

    /// 当前是否处于调试日志级别。
    pub fn is_debug(&self) -> bool {
        self.debug.load(Ordering::Relaxed)
    }

    /// 切换日志级别并立即生效。
    pub fn set_debug(&self, enabled: bool) {
        self.debug.store(enabled, Ordering::Relaxed);
        (self.apply)(enabled);
    }
}

/// 初始化桌面端日志。
///
/// 返回的 [`WorkerGuard`] 必须由调用方持有到进程退出，否则日志文件可能丢缓冲；
/// [`LogLevelSwitch`] 可克隆后交给 UI 用于运行时切换日志级别。
pub fn initialize(paths: &AppPaths) -> Result<(WorkerGuard, LogLevelSwitch)> {
    let log_path = paths.log_file("qinglink.log");
    let directory = log_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid QingLink log path"))?;
    let appender = tracing_appender::rolling::daily(directory, "qinglink.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let (filter, initial_debug) = initial_filter();
    let (filter_layer, filter_handle) = reload::Layer::new(filter);

    let registry = tracing_subscriber::registry().with(filter_layer);

    #[cfg(debug_assertions)]
    let registry = registry.with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr));

    registry
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .init();

    tracing::info!(
        log_path = %log_path.display(),
        debug = initial_debug,
        "桌面端日志系统已初始化"
    );

    let switch = LogLevelSwitch::new(
        initial_debug,
        Arc::new(move |enabled: bool| {
            if let Err(error) = filter_handle.reload(EnvFilter::new(level_directive(enabled))) {
                eprintln!("无法切换日志级别: {error}");
            }
        }),
    );
    Ok((guard, switch))
}

/// 构造初始过滤器：`QINGLINK_LOG_LEVEL`（或 `RUST_LOG`）优先，否则默认 info。
fn initial_filter() -> (EnvFilter, bool) {
    let env_value = std::env::var("QINGLINK_LOG_LEVEL")
        .ok()
        .or_else(|| std::env::var("RUST_LOG").ok());
    if let Some(value) = env_value.as_deref() {
        let trimmed = value.trim();
        // 只给了级别名（如 `debug`）时，限定为 qinglink 依赖树。
        let directive = if trimmed.contains('=') {
            trimmed.to_string()
        } else {
            format!("qinglink={trimmed}")
        };
        if let Ok(filter) = EnvFilter::try_new(&directive) {
            let debug = trimmed.to_ascii_lowercase().contains("debug")
                || trimmed.to_ascii_lowercase().contains("trace");
            return (filter, debug);
        }
    }
    (EnvFilter::new(level_directive(false)), false)
}

fn level_directive(debug: bool) -> &'static str {
    if debug {
        "qinglink=debug"
    } else {
        "qinglink=info"
    }
}
