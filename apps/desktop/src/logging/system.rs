//! 组装完成的日志系统：内存 sink 加运行时的级别开关。
//!
//! 这里刻意不设日志数据库。文本文件就是持久记录——它们能挺过崩溃，不依赖本应用
//! 即可阅读，而且不同于连接池背后的 SQLite 文件，绝不会让启动等待一次缓慢或被
//! 拒绝的磁盘操作。内存环形缓冲区服务于实时界面。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use qinglink_storage::AppPaths;
use serde::Serialize;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Registry, reload};

use super::LogFileInfo;
use super::buffer::{LogBuffer, LogFilter};
use super::collector::LogCollector;
use super::directory::{
    LOG_FILE_FALLBACK, LOG_FILE_STEM, SERVICE_LOG_STEM, desktop_log_directory, list_log_files,
    newest_log_name, service_log_directory,
};
use super::entry::LogEntry;
use super::log_file::{read_tail, resolve_log_path};
use super::service_log;
use super::subscriber::{file_layer, filter_layer, install, level_filter};

/// Records kept in memory for instant queries.
pub const BUFFER_CAPACITY: usize = 5_000;

/// One page of records parsed from a service log file.
#[derive(Debug, Serialize)]
pub struct ServiceLogPage {
    /// Name of the file the page was parsed from.
    pub file: String,
    pub items: Vec<LogEntry>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
}

/// One page of log records.
#[derive(Debug, Serialize)]
pub struct LogPage {
    pub items: Vec<LogEntry>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
}

/// Aggregate log statistics for the UI header.
#[derive(Debug, Serialize)]
pub struct LogStats {
    /// Records currently held in memory. Text files hold the rest.
    pub total: u64,
    pub by_level: std::collections::BTreeMap<String, u64>,
    pub buffer_capacity: usize,
    pub buffer_len: usize,
    pub oldest_ms: Option<i64>,
    pub newest_ms: Option<i64>,
    pub files: Vec<LogFileInfo>,
}

/// Owns the log sinks and the runtime level switch.
///
/// There is deliberately no log database. Text files are the durable record —
/// they survive a crash, they are readable without the application, and unlike a
/// SQLite file behind a connection pool they can never make startup wait on a
/// slow or denied disk operation. The in-memory ring answers the live interface.
pub struct LogSystem {
    buffer: Arc<LogBuffer>,
    collector: LogCollector,
    filter: reload::Handle<EnvFilter, Registry>,
    debug: AtomicBool,
    log_directory: PathBuf,
    /// Where `QingLinkService` writes its own rotating logs.
    ///
    /// Read directly from disk rather than over IPC: `%ProgramData%\QingLink\logs`
    /// grants the interactive user read access, and the management pipe caps a
    /// frame at 64 KiB, which a log download would have to work around.
    service_log_directory: PathBuf,
    /// Must stay alive for the process lifetime, or file logs lose buffered
    /// records on exit.
    _file_guard: WorkerGuard,
}

impl std::fmt::Debug for LogSystem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LogSystem")
            .field("log_directory", &self.log_directory)
            .field("service_log_directory", &self.service_log_directory)
            .field("debug", &self.is_debug())
            .finish_non_exhaustive()
    }
}

impl LogSystem {
    /// Install the global `tracing` subscriber.
    ///
    /// `debug` seeds the level; it can be changed later with
    /// [`Self::set_debug`] without restarting the process. Nothing here blocks
    /// on the data directory: the file sink degrades to stderr, and the buffer is
    /// memory, so neither can hold startup hostage.
    pub fn initialize(paths: &AppPaths, debug: bool) -> Result<Self> {
        let log_directory = desktop_log_directory(paths)?;

        let (file_layer, file_guard) = file_layer(&log_directory);
        let buffer = Arc::new(LogBuffer::new(BUFFER_CAPACITY));
        // The collector shares its id counter and sinks across clones, so the
        // layer installed below and the handle kept here stay in lockstep.
        let collector = LogCollector::new(Arc::clone(&buffer));
        let (filter_layer, filter_handle) = filter_layer(debug);

        // The layer stack itself is assembled in `subscriber`, which is where
        // every layer type lives; this function only wires the handles together.
        install(filter_layer, file_layer, &collector);

        Ok(Self {
            buffer,
            collector,
            filter: filter_handle,
            debug: AtomicBool::new(debug),
            log_directory,
            service_log_directory: service_log_directory(),
            _file_guard: file_guard,
        })
    }

    /// Subscribe to live records emitted from now on.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Arc<LogEntry>> {
        self.collector.subscribe()
    }

    /// Whether debug-level records are currently captured.
    pub fn is_debug(&self) -> bool {
        self.debug.load(Ordering::Relaxed)
    }

    /// Switch the capture level at runtime.
    pub fn set_debug(&self, enabled: bool) {
        self.debug.store(enabled, Ordering::Relaxed);
        if let Err(error) = self.filter.reload(level_filter(enabled)) {
            // Not fatal: the switch still applies to the next process start.
            tracing::warn!(%error, "无法切换日志级别");
        }
    }

    /// Newest-first page of records matching `filter`.
    ///
    /// Answers from the in-memory ring. Older records live on in the rotating
    /// text files, which the interface downloads rather than queries.
    pub fn query(&self, filter: &LogFilter, offset: usize, limit: usize) -> LogPage {
        let (items, total) = self.buffer.query(filter, offset, limit);
        let next_offset = offset.checked_add(items.len()).filter(|next| *next < total);
        LogPage {
            items,
            offset,
            limit,
            total,
            next_offset,
        }
    }

    /// Aggregate counters for the log page header.
    pub fn stats(&self) -> LogStats {
        let summary = self.buffer.summary();
        LogStats {
            total: summary.total,
            by_level: summary.by_level,
            buffer_capacity: self.buffer.capacity(),
            buffer_len: self.buffer.len(),
            oldest_ms: summary.oldest_ms,
            newest_ms: summary.newest_ms,
            files: self.log_files(),
        }
    }

    /// Clear the in-memory buffer. Text files on disk are kept.
    pub fn clear(&self) -> Result<u64> {
        Ok(self.buffer.clear() as u64)
    }

    /// Rotating text logs, newest name last.
    pub fn log_files(&self) -> Vec<LogFileInfo> {
        list_log_files(&self.log_directory)
    }

    /// Where `QingLinkService` writes its rotating logs.
    pub fn service_log_directory(&self) -> &Path {
        &self.service_log_directory
    }

    /// Where this desktop writes its rotating logs.
    pub fn log_directory(&self) -> &Path {
        &self.log_directory
    }

    /// Tail of a log file from either source, as lossy UTF-8.
    ///
    /// Used by the diagnostic bundle, which must stay bounded no matter how
    /// large the logs have grown.
    pub fn read_tail_text(
        &self,
        service: bool,
        name: Option<&str>,
        max_bytes: u64,
    ) -> Result<String> {
        let path = if service {
            self.resolve_service_log_file(name)?
        } else {
            self.resolve_log_file(name)?
        };
        let bytes = read_tail(&path, max_bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Rotating service logs, newest name last.
    pub fn service_log_files(&self) -> Vec<LogFileInfo> {
        list_log_files(&self.service_log_directory)
    }

    /// Resolve a service log download target.
    ///
    /// Same validation as [`Self::resolve_log_file`], against the service
    /// directory instead.
    pub fn resolve_service_log_file(&self, name: Option<&str>) -> Result<PathBuf> {
        let requested = match name {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => self.current_service_log_file_name(),
        };
        resolve_log_path(&self.service_log_directory, &requested)
    }

    /// Newest-first page of records parsed from a service log file.
    ///
    /// Only the tail of the file is read and at most 5000 records are kept (see
    /// [`service_log`]), so a long-running service cannot turn a page request
    /// into an unbounded read.
    pub fn read_service_log_page(
        &self,
        file: Option<&str>,
        filter: &LogFilter,
        offset: usize,
        limit: usize,
    ) -> Result<ServiceLogPage> {
        let path = self.resolve_service_log_file(file)?;
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| SERVICE_LOG_STEM.to_string());
        let records = service_log::read_page(&path, filter, offset, limit)?;
        let next_offset = offset
            .checked_add(records.items.len())
            .filter(|next| *next < records.total);
        Ok(ServiceLogPage {
            file: name,
            items: records.items,
            offset,
            limit,
            total: records.total,
            next_offset,
        })
    }

    /// Resolve a download target.
    ///
    /// `None` selects the most recent rotating file. The name is validated
    /// against a strict character set and the resolved path is re-checked
    /// against the log directory, so a crafted name cannot escape it.
    pub fn resolve_log_file(&self, name: Option<&str>) -> Result<PathBuf> {
        let requested = match name {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => self.current_log_file_name(),
        };
        resolve_log_path(&self.log_directory, &requested)
    }

    /// Name of the newest service log file.
    fn current_service_log_file_name(&self) -> String {
        newest_log_name(self.service_log_files(), SERVICE_LOG_STEM)
            .unwrap_or_else(|| SERVICE_LOG_STEM.to_string())
    }

    /// Name of the newest rotating log file.
    fn current_log_file_name(&self) -> String {
        newest_log_name(self.log_files(), LOG_FILE_STEM)
            .unwrap_or_else(|| LOG_FILE_FALLBACK.to_string())
    }
}
