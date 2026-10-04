//! 输入门控与保留输入：输入工作线程、输入代际、暂停开关与按键释放。
//!
//! 作为 `state` 的子模块，可直接访问 `AppState` 的私有输入字段，
//! 因此拆出该职责无需放宽字段可见性。

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::AppState;

impl AppState {
    /// 启动鼠标位移工作线程；重复调用不会创建第二个工作线程。
    pub(in crate::server) fn start_input_worker(&self) {
        if self.input_worker_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut input_move_rx = self.input_move_tx.subscribe();
        let control = Arc::clone(&self.control);
        let generation = Arc::clone(&self.input_generation);
        tokio::spawn(async move {
            while input_move_rx.changed().await.is_ok() {
                let (queued_generation, dx, dy) = *input_move_rx.borrow_and_update();
                if (dx != 0 || dy != 0) && !control.is_paused() {
                    let control = Arc::clone(&control);
                    let generation = Arc::clone(&generation);
                    let _ = tokio::task::spawn_blocking(move || {
                        let Ok(_guard) = control.input_gate.lock() else {
                            return;
                        };
                        if control.is_paused()
                            || generation.load(Ordering::Acquire) != queued_generation
                        {
                            return;
                        }
                        if let Err(error) = crate::platform::mouse_move(dx, dy) {
                            tracing::warn!(dx, dy, %error, "binary mouse move failed");
                        }
                    })
                    .await;
                }
            }
        });
    }

    /// Identifies the network/input lifetime; stale work cannot cross a pause.
    pub fn input_generation(&self) -> u64 {
        self.input_generation.load(Ordering::Acquire)
    }

    /// Reject work from previous connections before a listener is replaced.
    pub fn invalidate_input(&self) {
        self.input_generation.fetch_add(1, Ordering::AcqRel);
        self.input_move_tx
            .send_replace((self.input_generation(), 0, 0));
    }

    /// Persist the desired state; the network supervisor reconciles listeners.
    ///
    /// # Errors
    /// 目录数据库锁中毒、写入 `service.paused` 设置失败，或在暂停分支中配对
    /// 状态机锁中毒时返回 `Err`。
    pub fn set_paused(&self, paused: bool) -> anyhow::Result<()> {
        self.app_db
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?
            .set_setting("service.paused", &paused.to_string())
            .map_err(anyhow::Error::msg)?;
        if self.control.is_paused() != paused {
            self.input_generation.fetch_add(1, Ordering::AcqRel);
        }
        self.control.set_paused(paused);
        if paused {
            self.service
                .set_server_running(false, self.service.server_port());
            self.input_move_tx
                .send_replace((self.input_generation(), 0, 0));
            self.control.clear_pairing_desktop();
            self.connection_registry.close_all("SERVICE_PAUSED");
            self.input_connection_registry.close_all("SERVICE_PAUSED");
            let mut manager = self
                .pairing
                .lock()
                .map_err(|_| anyhow::anyhow!("pairing lock poisoned"))?;
            let ids: Vec<_> = manager
                .pending()
                .map(|request| request.request_id)
                .collect();
            for id in ids {
                manager.cancel(id);
            }
        }
        Ok(())
    }

    /// Wait for already-started input before releasing held state. Called off IPC.
    ///
    /// # Errors
    /// 输入门闩锁中毒、已按下输入记录锁中毒，或释放按键 / 鼠标键失败时返回 `Err`。
    pub fn release_held_input(&self) -> anyhow::Result<()> {
        let _guard = self
            .control
            .input_gate
            .lock()
            .map_err(|_| anyhow::anyhow!("input gate poisoned"))?;
        self.held_input
            .lock()
            .map_err(|_| anyhow::anyhow!("held input lock poisoned"))?
            .release()
    }
}
