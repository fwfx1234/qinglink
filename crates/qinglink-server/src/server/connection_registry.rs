//! WebSocket 会话注册表：跟踪连接、绑定令牌摘要并支持定向关闭。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::sync::mpsc;

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

/// 连接的单调递增标识。
pub type ConnectionId = u64;

/// 当前服务端的 WebSocket 会话注册表。
///
/// 每个会话只保留令牌摘要与关闭通知通道，因此手写 `Debug` 而不派生。
pub struct ConnectionRegistry {
    inner: Mutex<HashMap<u64, ConnectionInfoInner>>,
}

impl std::fmt::Debug for ConnectionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionRegistry").finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct ConnectionInfoInner {
    last_activity: Instant,
    // Store only the digest. Raw bearer tokens must not remain in the registry.
    token_hash: Option<String>,
    close_tx: mpsc::UnboundedSender<String>,
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionRegistry {
    /// 创建一个空注册表。
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// 登记一个新会话，返回其标识与关闭通知接收端。
    pub fn register(&self) -> (ConnectionId, mpsc::UnboundedReceiver<String>) {
        let id = NEXT_CONN_ID.fetch_add(1, Ordering::SeqCst);
        let now = Instant::now();
        let (close_tx, close_rx) = mpsc::unbounded_channel();
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.insert(
            id,
            ConnectionInfoInner {
                last_activity: now,
                token_hash: None,
                close_tx,
            },
        );
        (id, close_rx)
    }

    /// Binds an authenticated token and closes older sessions with the same
    /// token. Only its digest is retained after this call returns.
    pub fn bind_token(&self, id: ConnectionId, token: &str) {
        let hash = crate::server::auth::token_hash(token);
        let mut inner = crate::lock_unpoisoned(&self.inner);
        let old_ids: Vec<_> = inner
            .iter()
            .filter(|(other_id, info)| {
                **other_id != id && info.token_hash.as_deref() == Some(&hash)
            })
            .map(|(other_id, _)| *other_id)
            .collect();
        for old_id in old_ids {
            if let Some(info) = inner.remove(&old_id) {
                let _ = info.close_tx.send("REPLACED".to_string());
            }
        }
        if let Some(info) = inner.get_mut(&id) {
            info.token_hash = Some(hash);
        }
    }

    /// Closes all sessions whose stored digest is in `hashes`.
    pub fn close_hashes<I, S>(&self, hashes: I, reason: &str)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let hashes: std::collections::HashSet<String> = hashes
            .into_iter()
            .map(|hash| hash.as_ref().to_owned())
            .collect();
        if hashes.is_empty() {
            return;
        }
        let mut inner = crate::lock_unpoisoned(&self.inner);
        let matching_ids: Vec<_> = inner
            .iter()
            .filter(|(_, info)| {
                info.token_hash
                    .as_deref()
                    .is_some_and(|hash| hashes.contains(hash))
            })
            .map(|(id, _)| *id)
            .collect();
        for id in matching_ids {
            if let Some(info) = inner.remove(&id) {
                let _ = info.close_tx.send(reason.to_string());
            }
        }
    }

    /// Closes every registered session.
    pub fn close_all(&self, reason: &str) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        let sessions: Vec<_> = inner.drain().map(|(_, info)| info).collect();
        for info in sessions {
            let _ = info.close_tx.send(reason.to_string());
        }
    }

    /// Legacy raw-token API retained for callers that have not migrated.
    pub fn close_token(&self, token: &str, reason: &str) {
        let hash = crate::server::auth::token_hash(token);
        self.close_hashes([hash], reason);
    }

    /// 注销会话；连接结束后调用。
    pub fn unregister(&self, id: ConnectionId) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.remove(&id);
    }

    /// 刷新会话的最近活动时间。
    pub fn activity(&self, id: ConnectionId) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        if let Some(info) = inner.get_mut(&id) {
            info.last_activity = Instant::now();
        }
    }

    /// 当前已登记的会话数量。
    pub fn count(&self) -> usize {
        crate::lock_unpoisoned(&self.inner).len()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn new_session_replaces_an_existing_token_session() {
        let registry = ConnectionRegistry::new();
        let (old_id, mut old_close) = registry.register();
        let (new_id, _new_close) = registry.register();

        registry.bind_token(old_id, "token-1");
        registry.bind_token(new_id, "token-1");

        assert_eq!(old_close.try_recv().unwrap(), "REPLACED");
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn close_token_notifies_all_matching_sessions() {
        let registry = ConnectionRegistry::new();
        let (first, mut first_close) = registry.register();
        let (second, mut second_close) = registry.register();
        registry.bind_token(first, "token-1");
        registry.bind_token(second, "token-2");

        registry.close_token("token-1", "REVOKED");

        assert_eq!(first_close.try_recv().unwrap(), "REVOKED");
        assert!(second_close.try_recv().is_err());
    }

    #[test]
    fn close_hashes_isolates_same_name_devices_and_close_all_drains_registry() {
        let registry = ConnectionRegistry::new();
        let (first, mut first_close) = registry.register();
        let (second, mut second_close) = registry.register();
        registry.bind_token(first, "same-name-token-a");
        registry.bind_token(second, "same-name-token-b");

        let first_hash = crate::server::auth::token_hash("same-name-token-a");
        registry.close_hashes([first_hash], "REVOKED");
        assert_eq!(first_close.try_recv().unwrap(), "REVOKED");
        assert!(second_close.try_recv().is_err());
        assert_eq!(registry.count(), 1);

        registry.close_all("SHUTDOWN");
        assert_eq!(second_close.try_recv().unwrap(), "SHUTDOWN");
        assert_eq!(registry.count(), 0);
    }
}
