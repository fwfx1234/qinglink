use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde::Serialize;
use tokio::sync::mpsc;

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

pub type ConnectionId = u64;

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub id: u64,
    pub connected_at_secs: u64,
    pub last_activity_secs: u64,
}

pub struct ConnectionRegistry {
    inner: Mutex<HashMap<u64, ConnectionInfoInner>>,
}

#[derive(Debug)]
struct ConnectionInfoInner {
    connected_at: Instant,
    last_activity: Instant,
    token: Option<String>,
    close_tx: mpsc::UnboundedSender<String>,
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self) -> (ConnectionId, mpsc::UnboundedReceiver<String>) {
        let id = NEXT_CONN_ID.fetch_add(1, Ordering::SeqCst);
        let now = Instant::now();
        let (close_tx, close_rx) = mpsc::unbounded_channel();
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.insert(
            id,
            ConnectionInfoInner {
                connected_at: now,
                last_activity: now,
                token: None,
                close_tx,
            },
        );
        (id, close_rx)
    }

    /// Binds an authenticated token and closes older sessions with the same
    /// token. Removing them immediately keeps listings single-session while
    /// their WebSocket handlers process the close signal.
    pub fn bind_token(&self, id: ConnectionId, token: &str) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        let old_ids: Vec<_> = inner
            .iter()
            .filter(|(other_id, info)| **other_id != id && info.token.as_deref() == Some(token))
            .map(|(other_id, _)| *other_id)
            .collect();
        for old_id in old_ids {
            if let Some(info) = inner.remove(&old_id) {
                let _ = info.close_tx.send("REPLACED".to_string());
            }
        }
        if let Some(info) = inner.get_mut(&id) {
            info.token = Some(token.to_string());
        }
    }

    pub fn close_token(&self, token: &str, reason: &str) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        let matching_ids: Vec<_> = inner
            .iter()
            .filter(|(_, info)| info.token.as_deref() == Some(token))
            .map(|(id, _)| *id)
            .collect();
        for id in matching_ids {
            if let Some(info) = inner.remove(&id) {
                let _ = info.close_tx.send(reason.to_string());
            }
        }
    }

    pub fn unregister(&self, id: ConnectionId) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        inner.remove(&id);
    }

    pub fn activity(&self, id: ConnectionId) {
        let mut inner = crate::lock_unpoisoned(&self.inner);
        if let Some(info) = inner.get_mut(&id) {
            info.last_activity = Instant::now();
        }
    }

    pub fn count(&self) -> usize {
        crate::lock_unpoisoned(&self.inner).len()
    }

    pub fn list_connections(&self) -> Vec<ConnectionInfo> {
        let inner = crate::lock_unpoisoned(&self.inner);
        let now = Instant::now();
        inner
            .iter()
            .map(|(id, info)| ConnectionInfo {
                id: *id,
                connected_at_secs: now.duration_since(info.connected_at).as_secs(),
                last_activity_secs: now.duration_since(info.last_activity).as_secs(),
            })
            .collect()
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
}
