//! worker 线程与调用方共享的运行时内核。

use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use qinglink_protocol::service_ipc::DesktopCommand;
use tokio::sync::{broadcast, oneshot};

use super::{ServiceEvent, ServiceState};
use crate::service::ServiceError;

/// Maximum queued calls before the web layer starts rejecting them.
pub(super) const QUEUE_CAPACITY: usize = 64;

/// 一次调用的应答通道：worker 把结果送回等待中的调用方。
pub(super) type Reply = oneshot::Sender<Result<serde_json::Value, ServiceError>>;

/// Queue item: one command plus where to send its answer.
#[derive(Debug)]
pub(super) struct Job {
    pub(super) command: DesktopCommand,
    pub(super) reply: Reply,
}

#[derive(Debug)]
pub(super) struct Inner {
    pub(super) state: Mutex<ServiceState>,
    pub(super) jobs: SyncSender<Job>,
    pub(super) stop: Arc<AtomicBool>,
    /// Bumped by [`ServiceClient::reconnect`](super::handle::ServiceClient::reconnect) to cut the
    /// reconnect backoff short.
    pub(super) wake: AtomicU64,
    pub(super) events: broadcast::Sender<ServiceEvent>,
    pub(super) next_request: AtomicU64,
}
