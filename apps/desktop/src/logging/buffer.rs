//! 近期记录的内存环形缓冲区。
//!
//! 缓冲区是日志页面的数据来源：无需读盘即可回答"刚刚发生了什么"，且在日志文件
//! 打不开时仍然可用。更早的记录存放在滚动文本文件中，界面以下载而非查询的方式
//! 获取。

use std::collections::{BTreeMap, VecDeque};
use std::sync::RwLock;

use super::entry::{LogEntry, LogLevel};

/// Aggregate view of everything the buffer currently holds.
#[derive(Debug, Default)]
pub struct BufferSummary {
    /// Records held, matching [`LogBuffer::len`] at the same instant.
    pub total: u64,
    /// Count per level name, e.g. `WARN` → 12.
    pub by_level: BTreeMap<String, u64>,
    pub oldest_ms: Option<i64>,
    pub newest_ms: Option<i64>,
}

/// Query predicate shared by the buffer and the persistent store.
#[derive(Clone, Debug, Default)]
pub struct LogFilter {
    /// Keep records at this level or more severe.
    pub min_level: Option<LogLevel>,
    /// Case-insensitive substring match against the emitting module path.
    pub target: Option<String>,
    /// Case-insensitive substring match against the message.
    pub search: Option<String>,
    /// Inclusive lower bound, Unix milliseconds.
    pub since_ms: Option<i64>,
    /// Inclusive upper bound, Unix milliseconds.
    pub until_ms: Option<i64>,
}

impl LogFilter {
    /// Whether `entry` satisfies every configured constraint.
    pub fn matches(&self, entry: &LogEntry) -> bool {
        if self.min_level.is_some_and(|level| entry.level < level) {
            return false;
        }
        if let Some(target) = &self.target
            && !entry.target.to_ascii_lowercase().contains(target)
        {
            return false;
        }
        if let Some(search) = &self.search
            && !entry.message.to_ascii_lowercase().contains(search)
        {
            return false;
        }
        if self
            .since_ms
            .is_some_and(|since| entry.timestamp_ms < since)
        {
            return false;
        }
        if self
            .until_ms
            .is_some_and(|until| entry.timestamp_ms > until)
        {
            return false;
        }
        true
    }
}

/// Fixed-capacity ring of the most recent records.
#[derive(Debug)]
pub struct LogBuffer {
    entries: RwLock<VecDeque<LogEntry>>,
    capacity: usize,
}

impl LogBuffer {
    /// Create a buffer holding at most `capacity` records.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            entries: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    /// Append a record, evicting the oldest one when full.
    pub fn push(&self, entry: LogEntry) {
        let Ok(mut entries) = self.entries.write() else {
            return;
        };
        while entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Number of records currently held.
    pub fn len(&self) -> usize {
        self.entries
            .read()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    /// Configured maximum size.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Counts and time span of everything currently held, in one pass.
    ///
    /// Deliberately a single snapshot: computing the counters, the span and the
    /// size from separate locks would let the header disagree with itself.
    pub fn summary(&self) -> BufferSummary {
        let Ok(entries) = self.entries.read() else {
            return BufferSummary::default();
        };
        let mut summary = BufferSummary {
            total: entries.len() as u64,
            ..BufferSummary::default()
        };
        for entry in entries.iter() {
            *summary
                .by_level
                .entry(entry.level.as_str().to_string())
                .or_insert(0) += 1;
            summary.oldest_ms = Some(
                summary
                    .oldest_ms
                    .map_or(entry.timestamp_ms, |oldest| oldest.min(entry.timestamp_ms)),
            );
            summary.newest_ms = Some(
                summary
                    .newest_ms
                    .map_or(entry.timestamp_ms, |newest| newest.max(entry.timestamp_ms)),
            );
        }
        summary
    }

    /// Newest-first page of matching records plus the total match count.
    pub fn query(&self, filter: &LogFilter, offset: usize, limit: usize) -> (Vec<LogEntry>, usize) {
        let Ok(entries) = self.entries.read() else {
            return (Vec::new(), 0);
        };
        let matching: Vec<&LogEntry> = entries.iter().rev().filter(|e| filter.matches(e)).collect();
        let total = matching.len();
        let page = matching
            .into_iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        (page, total)
    }

    /// Drop every held record and report how many were removed.
    pub fn clear(&self) -> usize {
        let Ok(mut entries) = self.entries.write() else {
            return 0;
        };
        let removed = entries.len();
        entries.clear();
        removed
    }
}
