//! 一条已采集的日志记录及其严重级别。

use serde::Serialize;

/// Severity of a captured record.
///
/// Ordering is meaningful: `Trace < Debug < Info < Warn < Error`. The web API
/// filters by "this level or higher", which is a `>=` on this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Uppercase name, matching the serialized form and the SQLite column.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    /// Parse a case-insensitive level name, as sent by the web client.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" | "WARNING" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }

    /// Map a `tracing` level onto the stored representation.
    pub fn from_tracing(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::TRACE => Self::Trace,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::INFO => Self::Info,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::ERROR => Self::Error,
        }
    }
}

/// A single log record.
///
/// `id` is assigned by the collector from a process-wide monotonic counter, so
/// clients can poll for "entries newer than the highest id I have seen"
/// without relying on timestamps, which can repeat within a millisecond.
///
/// The counter restarts at 1 in every process, so `id` is comparable only
/// within one run; records are ordered by `timestamp_ms` (then `id`) everywhere
/// they are paged, so a restart cannot reorder them.
#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    /// Monotonic identifier, starting at 1 for each process.
    pub id: u64,
    /// Unix milliseconds.
    pub timestamp_ms: i64,
    /// Unix seconds, precomputed for clients that only need coarse times.
    pub timestamp: i64,
    pub level: LogLevel,
    /// `tracing` target, i.e. the module path that emitted the record.
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

impl LogEntry {
    /// Build an entry with a caller-supplied id and level.
    pub fn new(
        id: u64,
        timestamp_ms: i64,
        level: LogLevel,
        target: String,
        message: String,
    ) -> Self {
        Self {
            id,
            timestamp_ms,
            timestamp: timestamp_ms.div_euclid(1000),
            level,
            target,
            message,
            thread_name: None,
            file: None,
            line: None,
        }
    }

    /// Attach the emitting thread name.
    pub fn with_thread(mut self, thread_name: Option<String>) -> Self {
        self.thread_name = thread_name;
        self
    }

    /// Attach the source location.
    pub fn with_location(mut self, file: Option<String>, line: Option<u32>) -> Self {
        self.file = file;
        self.line = line;
        self
    }
}
