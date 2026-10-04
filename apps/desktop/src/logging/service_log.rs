//! 服务端滚动文本日志的解析与分页。
//!
//! 服务是独立进程，有自己的 `tracing_subscriber` 格式化器，因此它的文件在这里
//! 读取和解析，而不是经管理管道传来。只读取文件尾部，且只保留有界数量的记录，
//! 所以运行了数月的服务也不会把一次分页请求变成无界读取。

use std::path::Path;

use anyhow::Result;

use super::buffer::LogFilter;
use super::entry::{LogEntry, LogLevel};
use super::log_file::read_tail;

/// Source shown for service records that carry no parsable target.
const SERVICE_LOG_TARGET: &str = "QingLinkService";

/// Bytes read from the tail of one service log file at most.
///
/// The service writes a file per day and keeps them forever, so a full read
/// would be unbounded; the tail is what "just now" needs.
const SERVICE_LOG_TAIL_BYTES: u64 = 4 * 1024 * 1024;

/// Parsed records held from one service log file at most.
const SERVICE_LOG_MAX_RECORDS: usize = 5_000;

/// One page of records parsed from a service log file, newest first.
pub(crate) struct ServiceLogRecords {
    pub items: Vec<LogEntry>,
    pub total: usize,
}

/// Parse the tail of `path` and return the newest-first page of matching records.
pub(crate) fn read_page(
    path: &Path,
    filter: &LogFilter,
    offset: usize,
    limit: usize,
) -> Result<ServiceLogRecords> {
    let bytes = read_tail(path, SERVICE_LOG_TAIL_BYTES)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(SERVICE_LOG_MAX_RECORDS);
    let mut entries: Vec<LogEntry> = Vec::new();
    for line in &lines[start..] {
        if line.trim().is_empty() {
            continue;
        }
        match parse_service_line(entries.len() as u64 + 1, line) {
            Some(entry) => entries.push(entry),
            // A continuation of the previous record (a multi-line message
            // or a panic backtrace); keep it readable instead of dropping it.
            None => {
                if let Some(previous) = entries.last_mut() {
                    previous.message.push('\n');
                    previous.message.push_str(line);
                }
            }
        }
    }
    let matching: Vec<LogEntry> = entries
        .into_iter()
        .filter(|entry| filter.matches(entry))
        .collect();
    let total = matching.len();
    let items: Vec<LogEntry> = matching
        .into_iter()
        .rev()
        .skip(offset)
        .take(limit)
        .collect();
    Ok(ServiceLogRecords { items, total })
}

/// Parse one line of a `tracing_subscriber` formatted text log.
///
/// `None` means "this line continues the previous record". The parser is
/// deliberately tolerant: the service's log format is an implementation detail,
/// so an unrecognized line is preserved verbatim rather than dropped.
fn parse_service_line(id: u64, line: &str) -> Option<LogEntry> {
    let line = line.trim_end_matches(['\r', '\n']);
    let after_timestamp_offset = line.find(char::is_whitespace)?;
    let timestamp_token = &line[..after_timestamp_offset];
    let remainder = line[after_timestamp_offset..].trim_start();
    let level_end = remainder
        .find(char::is_whitespace)
        .unwrap_or(remainder.len());
    let level = LogLevel::parse(&remainder[..level_end])?;
    let rest = remainder[level_end..].trim_start();
    let timestamp_ms = parse_service_timestamp(timestamp_token).unwrap_or_default();
    let (target, message) = match rest.split_once(": ") {
        Some((target, message)) => (target.to_string(), message.to_string()),
        None => (SERVICE_LOG_TARGET.to_string(), rest.to_string()),
    };
    Some(LogEntry::new(id, timestamp_ms, level, target, message))
}

/// Convert the RFC 3339 timestamp prefix of a service log line into Unix ms.
fn parse_service_timestamp(token: &str) -> Option<i64> {
    let parsed =
        time::OffsetDateTime::parse(token, &time::format_description::well_known::Rfc3339).ok()?;
    i64::try_from(parsed.unix_timestamp_nanos() / 1_000_000).ok()
}
