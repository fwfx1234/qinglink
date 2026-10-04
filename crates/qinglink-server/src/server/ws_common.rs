//! WebSocket 处理共享逻辑：心跳与认证超时。

use std::time::Duration;

use tokio::time::Interval;

/// 服务端心跳间隔（秒）。
pub const HEARTBEAT_INTERVAL_SECS: u64 = 15;
/// 连续丢失 Pong 的阈值，超过则判定连接超时。
pub const MISSED_PONGS_THRESHOLD: u32 = 2;
/// 认证超时（秒），未认证连接会被关闭。
pub const AUTH_TIMEOUT_SECS: u64 = 10;

/// 服务端心跳状态机，跟踪连续丢失的 Pong 数。
///
/// `events` 与 `input` 两个 WebSocket 通道共用该逻辑，避免重复实现。
#[derive(Debug)]
pub struct Heartbeat {
    interval: Interval,
    missed_pongs: u32,
}

impl Heartbeat {
    /// 按 [`HEARTBEAT_INTERVAL_SECS`] 创建心跳，并消耗首次立即到期的 tick。
    pub async fn start() -> Self {
        let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        interval.tick().await;
        Self {
            interval,
            missed_pongs: 0,
        }
    }

    /// 返回下一次心跳到期的 future，用于 `tokio::select!`。
    pub fn tick(&mut self) -> impl std::future::Future<Output = tokio::time::Instant> + '_ {
        self.interval.tick()
    }

    /// 收到 Pong 后清零丢失计数。
    pub fn on_pong(&mut self) {
        self.missed_pongs = 0;
    }

    /// 记录一次心跳 tick，返回是否已达到超时阈值。
    pub fn note_tick(&mut self) -> bool {
        self.missed_pongs = self.missed_pongs.saturating_add(1);
        self.missed_pongs >= MISSED_PONGS_THRESHOLD
    }

    /// 当前连续丢失的 Pong 数（用于心跳标签等诊断信息）。
    pub fn missed_pongs(&self) -> u32 {
        self.missed_pongs
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn heartbeat_tracks_missed_pongs() {
        let mut heartbeat = Heartbeat::start().await;
        assert!(!heartbeat.note_tick());
        heartbeat.on_pong();
        assert!(!heartbeat.note_tick());
        assert!(heartbeat.note_tick());
    }
}
