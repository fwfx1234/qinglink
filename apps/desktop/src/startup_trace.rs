//! 启动面包屑：定位"进程活着但什么都没发生"。
//!
//! 单个阶段一条记录，格式见 `docs/architecture.md` §9.1。

use std::time::Instant;

/// Environment variable that enables the startup breadcrumb trace.
///
/// A GUI-subsystem process has no console, and the file log needs a writable
/// data directory, so "the process is alive but nothing happens" has no visible
/// cause at all. With this set, each startup step is written to stderr with its
/// elapsed time, which is enough to localise a hang without a debugger.
const STARTUP_TRACE_ENV: &str = "QINGLINK_STARTUP_TRACE";

/// Whether the startup breadcrumb trace is enabled.
fn trace_enabled() -> bool {
    std::env::var_os(STARTUP_TRACE_ENV).is_some()
}

/// Write one startup breadcrumb, if the trace is enabled.
pub fn trace_step(started_at: Instant, step: &str) {
    if trace_enabled() {
        eprintln!(
            "[qinglink-startup] +{:>7.3}s pid={} {step}",
            started_at.elapsed().as_secs_f64(),
            std::process::id()
        );
    }
}
