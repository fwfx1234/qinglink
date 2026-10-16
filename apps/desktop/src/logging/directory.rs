//! 磁盘上的日志目录及其滚动文件。
//!
//! 涉及两个目录：本桌面端写入的目录，以及 `QingLinkService` 写入的目录。两者以
//! 相同方式列出，且列举永不失败——目录不可读时返回空列表，因为日志页面必须仍能
//! 渲染。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use qinglink_storage::AppPaths;
use serde::Serialize;

/// Stem of the rotating text log files. `tracing_appender` inserts the rotation
/// date before the suffix, producing names such as `qinglink.2026-10-04.log`
/// that sort chronologically as strings.
///
/// 历史版本产出的是 `qinglink.log.2026-09-04` 这种"后缀在前"的名字；两种命名会同时留在
/// 磁盘上，且旧的按名字排在新版之后，因此"最新文件"必须按修改时间挑（见
/// [`newest_log_name`]），不能按名字。
pub(crate) const LOG_FILE_STEM: &str = "qinglink";

/// Suffix of the rotating text log files.
pub(crate) const LOG_FILE_SUFFIX: &str = "log";

/// Conventional name used when no rotating file exists yet.
pub(crate) const LOG_FILE_FALLBACK: &str = "qinglink.log";

/// Environment override for the service's log directory, used by tests and by
/// portable deployments that do not write to `%ProgramData%`.
const SERVICE_LOG_DIR_ENV: &str = "QINGLINK_SERVICE_LOG_DIR";

/// File name prefix of the service's rotating logs.
pub(crate) const SERVICE_LOG_STEM: &str = "QingLinkService.log";

/// A rotating log file on disk.
#[derive(Debug, Serialize)]
pub struct LogFileInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified_secs: i64,
}

/// Directory this desktop writes its rotating logs to.
///
/// Resolved from [`AppPaths`] rather than composed here, so the desktop and the
/// storage crate can never disagree about where the logs live.
pub(crate) fn desktop_log_directory(paths: &AppPaths) -> Result<PathBuf> {
    let log_path = paths.log_file(LOG_FILE_FALLBACK);
    log_path
        .parent()
        .map(Path::to_path_buf)
        .context("the log path has no parent directory")
}

/// Directory the service writes its rotating logs to.
pub(crate) fn service_log_directory() -> PathBuf {
    if let Some(path) = std::env::var_os(SERVICE_LOG_DIR_ENV).filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }
    std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("QingLink")
        .join("logs")
}

/// List the regular files of a log directory, sorted by name.
pub(crate) fn list_log_files(directory: &Path) -> Vec<LogFileInfo> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut files: Vec<LogFileInfo> = entries
        .flatten()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let modified_secs = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|elapsed| elapsed.as_secs() as i64)
                .unwrap_or_default();
            Some(LogFileInfo {
                name,
                size_bytes: metadata.len(),
                modified_secs,
            })
        })
        .collect();
    files.sort_by(|left, right| left.name.cmp(&right.name));
    files
}

/// Name of the most recently modified file with `stem` as its prefix.
///
/// Selection is by modification time rather than by name: the legacy
/// `qinglink.log.2026-09-04` naming sorts *after* `qinglink.2026-10-04.log`, so
/// name order would hand out a stale log — the same trap the frontend works
/// around when it picks a download target.
pub(crate) fn newest_log_name(files: Vec<LogFileInfo>, stem: &str) -> Option<String> {
    files
        .into_iter()
        .filter(|file| file.name.starts_with(stem))
        .max_by_key(|file| file.modified_secs)
        .map(|file| file.name)
}
