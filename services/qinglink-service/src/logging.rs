//! Windows 服务的日志初始化。
//!
//! 服务进程没有控制台，也不写事件日志，因此这里必须保证"日志一定有去处"：
//! 目录建不起来时退到临时目录，再失败则退到 stderr 并明确记一条错误，
//! 而不是整段静默返回、让进程在全机零痕迹的情况下运行。

#[cfg(target_os = "windows")]
use std::path::PathBuf;

/// 与桌面端共用的日志级别变量名；未设置时只记录 `qinglink` 目标的 info。
#[cfg(target_os = "windows")]
const LOG_LEVEL_VARIABLE: &str = "QINGLINK_LOG_LEVEL";

/// 未设置环境变量时的默认过滤指令。
#[cfg(target_os = "windows")]
const DEFAULT_FILTER: &str = "qinglink=info";

#[cfg(target_os = "windows")]
fn filter_directive() -> String {
    std::env::var(LOG_LEVEL_VARIABLE)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_FILTER.to_string())
}

/// 解析可用的日志目录：优先 `PROGRAMDATA`，失败退到临时目录。
///
/// 两级都失败时返回 `None`，由调用方切到 stderr 兜底。
#[cfg(target_os = "windows")]
fn resolve_log_directory() -> Option<PathBuf> {
    let preferred = std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("QingLink\\logs");
    if std::fs::create_dir_all(&preferred).is_ok() {
        return Some(preferred);
    }
    let fallback = std::env::temp_dir().join("QingLink\\logs");
    if std::fs::create_dir_all(&fallback).is_ok() {
        return Some(fallback);
    }
    None
}

#[cfg(target_os = "windows")]
pub fn init_service_logging() {
    let directive = filter_directive();
    match resolve_log_directory() {
        Some(directory) => {
            let file_appender = tracing_appender::rolling::daily(&directory, "QingLinkService.log");
            let (writer, guard) = tracing_appender::non_blocking(file_appender);
            let _ = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(writer)
                .with_env_filter(directive.as_str())
                .try_init();
            Box::leak(Box::new(guard));
            tracing::info!(
                log_directory = %directory.display(),
                filter = %directive,
                "服务日志已初始化"
            );
        }
        None => {
            let _ = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(std::io::stderr)
                .with_env_filter(directive.as_str())
                .try_init();
            tracing::error!(
                filter = %directive,
                "无法创建任何日志目录（PROGRAMDATA 与临时目录均失败），日志只写 stderr；\
                 服务进程无控制台，本次运行不会留下任何持久记录"
            );
        }
    }
    install_panic_hook();
}

/// 把 panic 转成一条 `tracing::error!`。
///
/// 服务端没有控制台，默认 hook 的 stderr 写入等于写向虚空；不装这个钩子，
/// 一次 panic 只表现为"远程指令突然全部失灵"，日志文件里一个字都没有。
#[cfg(target_os = "windows")]
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map_or_else(
            || "<unknown>".to_string(),
            |value| format!("{}:{}:{}", value.file(), value.line(), value.column()),
        );
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_string());
        let thread = std::thread::current();
        tracing::error!(
            location = %location,
            thread = %thread.name().unwrap_or("<unnamed>"),
            payload = %payload,
            "线程 panic"
        );
        previous(info);
    }));
}
