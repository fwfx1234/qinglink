//! 日志栈的组装：过滤层、持久化文件 sink 与全局 subscriber 的安装。
//!
//! 两部分都与运行时控制它们的句柄成对构建：过滤层返回 `reload::Handle`，文件
//! sink 返回必须在进程整个生命周期内存活的 `WorkerGuard`。采集级别的环境变量覆盖
//! 也在这里，因为过滤器正由它初始化。

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, reload};

use super::directory::{LOG_FILE_STEM, LOG_FILE_SUFFIX};

/// Level captured when neither `QINGLINK_LOG_LEVEL` nor `RUST_LOG` is set.
///
/// The default is `info` rather than `error`: a `cargo run` that prints nothing
/// while the process is alive is indistinguishable from a process that is stuck
/// before it logs anything, and that was a real development cost here.
pub(crate) const DEFAULT_LOG_LEVEL: &str = "info";

/// Environment variable that overrides the level, highest priority first.
const LEVEL_ENVIRONMENT_VARIABLES: [&str; 2] = ["QINGLINK_LOG_LEVEL", "RUST_LOG"];

/// The durable file sink: a rotating text file, or nothing.
///
/// The worker guard returned alongside this layer must outlive the process, so
/// it is owned by [`super::LogSystem`]; the layer only holds the writer.
pub(crate) struct FileLayer {
    appender: Option<tracing_appender::non_blocking::NonBlocking>,
}

impl<S> tracing_subscriber::Layer<S> for FileLayer
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(appender) = &self.appender {
            let layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(appender.clone());
            tracing_subscriber::Layer::on_event(&layer, event, context);
        }
    }
}

/// Build the file sink, degrading to "no file layer" when the directory is not
/// writable. Logging must never prevent the application from starting, and the
/// console layer installed next to this one already covers that case.
pub(crate) fn file_layer(directory: &Path) -> (FileLayer, WorkerGuard) {
    match tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_FILE_STEM)
        .filename_suffix(LOG_FILE_SUFFIX)
        .build(directory)
    {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (
                FileLayer {
                    appender: Some(writer),
                },
                guard,
            )
        }
        Err(error) => {
            eprintln!(
                "QingLink cannot write logs to {}: {error}; using the console only",
                directory.display()
            );
            // A sink keeps the guard's type concrete; nothing is written to the
            // file layer in this case.
            let (_writer, guard) = tracing_appender::non_blocking(std::io::sink());
            (FileLayer { appender: None }, guard)
        }
    }
}

/// Build the filter layer together with the handle that switches it at runtime.
pub(crate) fn filter_layer(
    debug: bool,
) -> (
    reload::Layer<EnvFilter, tracing_subscriber::Registry>,
    reload::Handle<EnvFilter, tracing_subscriber::Registry>,
) {
    tracing_subscriber::reload::Layer::new(initial_filter(debug))
}

/// Install the global subscriber: filter, file sink, console and collector.
///
/// The layer types never leave this module, so the trait that provides `.with`
/// is needed only here; [`super::LogSystem::initialize`] just passes the handles
/// in and keeps the ones it has to own.
pub(crate) fn install(
    filter_layer: reload::Layer<EnvFilter, tracing_subscriber::Registry>,
    file_layer: FileLayer,
    collector: &super::collector::LogCollector,
) {
    // Both console layers are kept in every build:
    //
    // * a GUI-subsystem release build has no console, so its stderr writes go
    //   nowhere — that costs nothing;
    // * a debug build is a console-subsystem process (`main.rs`), so it
    //   inherits the terminal that ran `cargo run` and shows these records
    //   directly.
    //
    // The file layer is only installed when a rotating file was actually
    // opened; when the directory is unusable the appender already writes to
    // stderr, and adding the console layer on top of it would print every
    // record twice.
    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(file_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr),
        )
        .with(collector.clone());
    subscriber.init();
}

/// The level override from the environment, if one is set to a usable value.
///
/// A variable that is set but blank counts as unset: `RUST_LOG=` in a shell
/// profile must not silently switch the application to `error`-only logging.
pub fn level_override_from_environment() -> Option<String> {
    LEVEL_ENVIRONMENT_VARIABLES
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.trim().is_empty())
}

/// Initial filter: `QINGLINK_LOG_LEVEL` / `RUST_LOG` win over the stored
/// preference, so a developer can always force more detail from the shell.
///
/// A bare level name goes through [`env_directive`], which expands it into the
/// application's two targets; the fallback is [`level_filter`], which is the same
/// expansion for [`DEFAULT_LOG_LEVEL`].
fn initial_filter(debug: bool) -> EnvFilter {
    if let Some(value) = level_override_from_environment()
        && let Ok(filter) = EnvFilter::try_new(env_directive(value.trim()))
    {
        return filter;
    }
    level_filter(debug)
}

/// The filter for a given level switch position.
///
/// Both positions are built as an explicit directive list, so the runtime switch
/// and the startup default can never drift apart: `debug` expands to the two
/// application targets at `debug`, and the off position to [`DEFAULT_LOG_LEVEL`]
/// over the same two targets.
pub(crate) fn level_filter(debug: bool) -> EnvFilter {
    let level = if debug { "debug" } else { DEFAULT_LOG_LEVEL };
    EnvFilter::new(env_directive(level))
}

/// Expand a bare level name into explicit directives.
///
/// `EnvFilter` matches targets case-sensitively, and this binary's target is
/// `QingLink` while its dependencies are `qinglink_*`. A bare `debug` must
/// therefore expand to both, or the desktop's own records (including the tray
/// registration diagnostics) would be silently discarded.
fn env_directive(trimmed: &str) -> String {
    if trimmed.contains('=') {
        trimmed.to_string()
    } else {
        format!("QingLink={trimmed},qinglink={trimmed}")
    }
}
