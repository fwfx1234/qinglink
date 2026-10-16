//! 进程图标缓存：按可执行文件路径复用解析结果。
//!
//! 同一个 exe 在进程列表里可能有几十个 PID（chrome.exe 之类），逐行按需取图时
//! 必须让重复请求命中内存；否则一次滑动就会把同一次解析重跑几十遍，并且每次
//! 都要读文件资源、解码位图再重新编码。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::protocol::responses::ProcessIconResponse;
use crate::server::AppState;

/// 负结果（这个路径没有图标）的有效期：程序后来补上自带图标后能自愈。
const NEGATIVE_TTL: Duration = Duration::from_secs(300);

/// 缓存条目上限；超出后按写入时间淘汰最旧的条目。
const MAX_ENTRIES: usize = 512;

/// 缓存载荷字节上限（图标是 128×128 PNG，典型 10–20 KiB，512 条约 10 MiB）。
const MAX_BYTES: usize = 16 * 1024 * 1024;

/// 解析不到图标时报告的来源标识，与平台层的 `none` 一致。
const SOURCE_NONE: &str = "none";

/// 一次解析的结果，正结果与负结果共用同一条缓存记录。
#[derive(Debug)]
struct CachedIcon {
    source: String,
    icon_path: Option<String>,
    icon_base64: Option<String>,
    stored_at: Instant,
}

impl CachedIcon {
    /// 正结果长期有效；负结果只在 [`NEGATIVE_TTL`] 内有效。
    fn is_fresh(&self) -> bool {
        self.icon_base64.is_some() || self.stored_at.elapsed() < NEGATIVE_TTL
    }

    fn payload_bytes(&self) -> usize {
        self.icon_base64.as_ref().map_or(0, String::len)
    }
}

/// 按可执行文件路径（统一小写）缓存的进程图标。
#[derive(Debug, Default)]
pub struct ProcessIconCache {
    entries: HashMap<String, CachedIcon>,
    bytes: usize,
}

impl ProcessIconCache {
    /// 读取尚未过期的缓存。
    fn get(&self, key: &str) -> Option<&CachedIcon> {
        self.entries.get(key).filter(|icon| icon.is_fresh())
    }

    /// 写入一条结果，并在超出条数或字节上限时淘汰最旧的条目。
    fn insert(&mut self, key: String, icon: CachedIcon) {
        let added = icon.payload_bytes();
        let removed = self
            .entries
            .insert(key, icon)
            .map_or(0, |previous| previous.payload_bytes());
        self.bytes = self.bytes.saturating_sub(removed).saturating_add(added);
        while self.entries.len() > MAX_ENTRIES || self.bytes > MAX_BYTES {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, icon)| icon.stored_at)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if let Some(removed) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(removed.payload_bytes());
            }
        }
    }
}

impl AppState {
    /// 解析一个进程的图标；同一可执行文件路径只解析一次。
    ///
    /// 进程不存在、路径不可读或没有自带图标时返回 `source: "none"`，不是错误。
    pub fn process_icon(&self, pid: u32) -> ProcessIconResponse {
        let Some(path) = self.pid_exe_path(pid) else {
            return missing_response(pid);
        };

        let key = path.to_lowercase();
        if let Ok(cache) = self.process_icon_cache.lock()
            && let Some(icon) = cache.get(&key)
        {
            return response(pid, icon);
        }

        // 解析会读文件资源、解码位图并重新编码，代价远高于查表，因此拿到路径后
        // 先放掉进程快照锁，也绝不在图标缓存锁内调用平台层。
        let resolution = crate::platform::resolve_process_icon(&path);
        let icon = CachedIcon {
            source: resolution.source,
            icon_path: resolution.source_path,
            icon_base64: resolution.icon_base64,
            stored_at: Instant::now(),
        };
        let response = response(pid, &icon);
        if let Ok(mut cache) = self.process_icon_cache.lock() {
            cache.insert(key, icon);
        }
        response
    }

    /// 从进程快照里读取 PID 对应的可执行文件路径。
    fn pid_exe_path(&self, pid: u32) -> Option<String> {
        crate::lock_unpoisoned(&self.process_manager).exe_path(pid)
    }
}

/// 没有可解析路径时的应答。
fn missing_response(pid: u32) -> ProcessIconResponse {
    ProcessIconResponse {
        pid,
        source: SOURCE_NONE.to_string(),
        icon_path: None,
        icon_base64: None,
    }
}

/// 把一条缓存记录转成协议应答。
fn response(pid: u32, icon: &CachedIcon) -> ProcessIconResponse {
    ProcessIconResponse {
        pid,
        source: icon.source.clone(),
        icon_path: icon.icon_path.clone(),
        icon_base64: icon.icon_base64.clone(),
    }
}
