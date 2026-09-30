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
    // `rolling::daily` panics when the current user cannot open the log file.
    // Logging must never prevent the desktop UI and tray window from starting.
    let (writer, guard, file_logging) =
        match tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("qinglink.log")
            .build(directory)
        {
            Ok(appender) => {
                let (writer, guard) = tracing_appender::non_blocking(appender);
                (writer, guard, true)
            }
            Err(error) => {
                eprintln!(
                    "QingLink cannot initialize file logging at {}: {error}; using stderr",
                    log_path.display()
                );
                let (writer, guard) = tracing_appender::non_blocking(std::io::stderr());
                (writer, guard, false)
            }
        };

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
        file_logging,
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
        let directive = env_directive(trimmed);
        if let Ok(filter) = EnvFilter::try_new(&directive) {
            let debug = trimmed.to_ascii_lowercase().contains("debug")
                || trimmed.to_ascii_lowercase().contains("trace");
            return (filter, debug);
        }
    }
    (EnvFilter::new(level_directive(false)), false)
}

/// 把 `QINGLINK_LOG_LEVEL` / `RUST_LOG` 的取值转换成 EnvFilter 指令。
///
/// 只给了级别名（如 `debug`）时，展开为 `QingLink=debug,qinglink=debug`：
/// 桌面端 bin target 是大小写敏感的 `QingLink`，必须显式列出，否则本程序
/// 自身的日志（含托盘注册失败告警）会被 `EnvFilter` 全部丢弃。
/// 已经带 `=` 的完整指令原样透传，由调用方决定规则。
fn env_directive(trimmed: &str) -> String {
    if trimmed.contains('=') {
        trimmed.to_string()
    } else {
        format!("QingLink={trimmed},qinglink={trimmed}")
    }
}

/// 默认指令：同时放行桌面端 bin target（`QingLink`）与 `qinglink*` 依赖 crate。
///
/// `EnvFilter` 的目标匹配是大小写敏感的模块路径前缀匹配，而本程序的 tracing
/// target 是 `QingLink` / `QingLink::tray` 等。只写小写 `qinglink=info` 会让
/// 桌面端自身的日志（含托盘失败告警）被全部丢弃，因此两个前缀都要保留。
fn level_directive(debug: bool) -> &'static str {
    if debug {
        "QingLink=debug,qinglink=debug"
    } else {
        "QingLink=info,qinglink=info"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用真实的分层 subscriber 验证各 target 是否放行。
    ///
    /// `tracing::enabled!` 的 target 必须是编译期常量，因此断言都写在内层闭包里。
    fn assert_enabled(directive: &str, check: impl FnOnce()) {
        let subscriber = tracing_subscriber::registry().with(EnvFilter::new(directive));
        tracing::subscriber::with_default(subscriber, check);
    }

    #[test]
    fn default_directive_enables_desktop_binary_targets() {
        assert_enabled(level_directive(false), || {
            assert!(
                tracing::enabled!(target: "QingLink", tracing::Level::INFO),
                "桌面端 bin target 必须被日志过滤器放行"
            );
            assert!(
                tracing::enabled!(target: "QingLink::tray", tracing::Level::INFO),
                "托盘模块日志必须被放行，否则注册失败无法排查"
            );
            // 依赖 crate（连字符会转成下划线）同样保留原有行为。
            assert!(tracing::enabled!(
                target: "qinglink_server::server",
                tracing::Level::INFO
            ));
        });
    }

    #[test]
    fn default_directive_keeps_debug_level_gated() {
        assert_enabled(level_directive(false), || {
            assert!(!tracing::enabled!(
                target: "QingLink::tray",
                tracing::Level::DEBUG
            ));
        });
        assert_enabled(level_directive(true), || {
            assert!(tracing::enabled!(
                target: "QingLink::tray",
                tracing::Level::DEBUG
            ));
        });
    }

    #[test]
    fn bare_env_level_keeps_desktop_targets() {
        // 模拟 `QINGLINK_LOG_LEVEL=debug`：不能只生成小写 `qinglink=debug`。
        let directive = env_directive("debug");
        assert_enabled(&directive, || {
            assert!(tracing::enabled!(
                target: "QingLink::tray",
                tracing::Level::DEBUG
            ));
        });
    }

    #[test]
    fn explicit_env_directive_is_passed_through() {
        assert_eq!(env_directive("QingLink=debug"), "QingLink=debug");
    }
}
