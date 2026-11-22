//! 桌面端更新状态机。
//!
//! 流程：`idle` →（`check`）→ `available` →（`download`）→ `downloading` → `ready`
//! →（`install`）→ `installing` → 进程退出。任何一步失败进入 `error`，但保留可重试能力。
//! 检查与下载都在后台线程进行，界面通过 `update_state` WebSocket 事件收敛状态；
//! REST 只负责触发动作与读取当前快照。

use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

use github::UpdateInfo;

pub mod checksum;
pub mod download;
pub mod github;
pub mod http;
pub mod install;
pub mod self_update;

/// 更新流程阶段，序列化给前端的稳定字符串。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    /// 未检查过。
    #[default]
    Idle,
    /// 正在请求 GitHub。
    Checking,
    /// 已是最新版本。
    UpToDate,
    /// 发现新版本，尚未下载。
    Available,
    /// 正在后台下载。
    Downloading,
    /// 已下载并校验，等待用户触发安装。
    Ready,
    /// 更新器已启动，主程序即将退出。
    Installing,
    /// 最近一次操作失败。
    Error,
}

/// 面向前端的完整状态快照。
#[derive(Clone, Debug, Serialize)]
pub struct UpdateView {
    pub phase: UpdatePhase,
    /// 当前运行版本（`qinglink_version::VERSION`）。
    pub current_version: String,
    /// 发现的最新版本（available/downloading/ready 时有值）。
    pub latest_version: Option<String>,
    /// release 页面地址。
    pub release_url: Option<String>,
    /// 更新说明。
    pub notes: String,
    /// 下载进度百分比 0-100。
    pub progress: u8,
    /// 已下载字节数。
    pub downloaded_bytes: u64,
    /// 总字节数（0 表示未知）。
    pub total_bytes: u64,
    /// 最近一次错误（phase 为 Error 时有值）。
    pub error: Option<String>,
    /// 当前是否有后台任务在跑，前端据此禁用按钮。
    pub busy: bool,
    /// 上次成功检查距今的秒数，用于界面提示"多久前检查过"。
    pub last_check_secs: Option<u64>,
}

impl Default for UpdateView {
    fn default() -> Self {
        Self {
            phase: UpdatePhase::Idle,
            current_version: qinglink_version::VERSION.to_string(),
            latest_version: None,
            release_url: None,
            notes: String::new(),
            progress: 0,
            downloaded_bytes: 0,
            total_bytes: 0,
            error: None,
            busy: false,
            last_check_secs: None,
        }
    }
}

/// 内部可变状态：快照 + 已发现的更新 + 上次检查时刻。
#[derive(Default)]
struct Inner {
    view: UpdateView,
    discovered: Option<UpdateInfo>,
    last_check: Option<Instant>,
}

/// 线程安全的更新状态句柄；克隆后共享同一份数据。
#[derive(Clone, Default)]
pub struct SharedUpdateState {
    inner: Arc<Mutex<Inner>>,
}

impl SharedUpdateState {
    /// 读取面向前端的快照，按需刷新"上次检查时间"。
    pub fn view(&self) -> UpdateView {
        let Ok(mut inner) = self.inner.lock() else {
            return UpdateView::default();
        };
        if let Some(at) = inner.last_check {
            inner.view.last_check_secs = Some(at.elapsed().as_secs());
        }
        inner.view.clone()
    }

    /// 标记检查开始。
    pub fn begin_check(&self) {
        self.mutate(|inner| {
            inner.view.phase = UpdatePhase::Checking;
            inner.view.busy = true;
            inner.view.error = None;
        });
    }

    /// 检查完成：无更新。
    pub fn finish_no_update(&self) {
        self.mutate(|inner| {
            inner.last_check = Some(Instant::now());
            inner.view.phase = UpdatePhase::UpToDate;
            inner.view.busy = false;
            inner.view.latest_version = None;
            inner.discovered = None;
        });
    }

    /// 检查完成：发现更新。返回是否真的发现了新版本。
    pub fn finish_check(&self, info: UpdateInfo) -> bool {
        let mut changed = false;
        self.mutate(|inner| {
            inner.last_check = Some(Instant::now());
            changed = inner
                .discovered
                .as_ref()
                .is_none_or(|old| old.version != info.version);
            inner.view.phase = UpdatePhase::Available;
            inner.view.busy = false;
            inner.view.latest_version = Some(info.version.clone());
            inner.view.release_url = Some(info.release_url.clone());
            inner.view.notes = info.notes.clone();
            // 换版本时清零旧的下载进度。
            if changed {
                inner.view.progress = 0;
                inner.view.downloaded_bytes = 0;
                inner.view.total_bytes = 0;
            }
            inner.discovered = Some(info);
        });
        changed
    }

    /// 取出当前发现的更新（下载线程用）。
    pub fn discovered(&self) -> Option<UpdateInfo> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.discovered.clone())
    }

    /// 标记下载开始。
    pub fn begin_download(&self) {
        self.mutate(|inner| {
            inner.view.phase = UpdatePhase::Downloading;
            inner.view.busy = true;
            inner.view.progress = 0;
            inner.view.error = None;
        });
    }

    /// 更新下载进度。
    pub fn set_progress(&self, downloaded: u64, total: u64) {
        self.mutate(|inner| {
            inner.view.downloaded_bytes = downloaded;
            inner.view.total_bytes = total;
            inner.view.progress = if total == 0 {
                0
            } else {
                ((downloaded * 100) / total).min(100) as u8
            };
        });
    }

    /// 标记就绪：已下载并校验。
    pub fn finish_download(&self) {
        self.mutate(|inner| {
            inner.view.phase = UpdatePhase::Ready;
            inner.view.busy = false;
            inner.view.progress = 100;
        });
    }

    /// 标记安装已启动。
    pub fn finish_install(&self) {
        self.mutate(|inner| {
            inner.view.phase = UpdatePhase::Installing;
            inner.view.busy = true;
        });
    }

    /// 记录错误。
    pub fn fail(&self, message: impl Into<String>) {
        self.mutate(|inner| {
            inner.view.phase = UpdatePhase::Error;
            inner.view.busy = false;
            inner.view.error = Some(message.into());
        });
    }

    fn mutate(&self, action: impl FnOnce(&mut Inner)) {
        if let Ok(mut inner) = self.inner.lock() {
            action(&mut inner);
        }
    }
}
