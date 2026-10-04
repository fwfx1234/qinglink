use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use thiserror::Error;
use uuid::Uuid;

const REQUEST_TTL: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u8 = 5;
const PIN_LEN: usize = 6;
/// Bounds untrusted device metadata before it enters the pending-request map.
pub const MAX_DEVICE_ID_LEN: usize = 128;
/// 设备展示名称的最大字节长度。
pub const MAX_DEVICE_NAME_LEN: usize = 128;
/// Limits memory retained by unauthenticated pairing requests.
pub const MAX_PENDING_REQUESTS: usize = 128;

/// 一次待确认的配对请求，携带一次性 PIN 与过期时刻。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingRequest {
    /// 本次配对请求的唯一标识。
    pub request_id: Uuid,
    /// 请求方上报的设备标识。
    pub device_id: String,
    /// 设备的展示名称。
    pub device_name: String,
    /// 用于确认配对的六位数字 PIN。
    pub pin: String,
    /// 请求的过期时刻，超过后确认会返回 `Expired`。
    pub expires_at: Instant,
}

/// 配对确认成功后的结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingResult {
    /// 被确认的配对请求标识。
    pub request_id: Uuid,
    /// 完成配对的设备标识。
    pub device_id: String,
    /// 完成配对的设备名称。
    pub device_name: String,
    /// 确认时使用的六位数字 PIN。
    pub pin: String,
}

/// 配对流程可能返回的错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PairingError {
    /// 请求不存在，可能已确认、已取消或从未创建。
    #[error("pairing request not found")]
    NotFound,
    /// 请求已超过有效期。
    #[error("pairing request expired")]
    Expired,
    /// PIN 不正确，剩余尝试次数已减少。
    #[error("invalid pairing code")]
    InvalidPin,
    /// PIN 尝试次数已达上限，请求被限流。
    #[error("pairing request is rate limited")]
    RateLimited,
    /// 请求不属于该设备。
    #[error("pairing request device does not match")]
    DeviceMismatch,
    /// 设备元数据或 PIN 格式非法。
    #[error("pairing request metadata is invalid")]
    InvalidRequest,
    /// 待处理请求数已达上限。
    #[error("too many pairing requests are pending")]
    TooManyPending,
}

struct PendingRequest {
    request: PairingRequest,
    attempts: u8,
}

/// 设备配对状态机：保存待确认的配对请求、校验 PIN 并限制尝试次数。
///
/// 刻意不 `derive(Debug)`：内部请求携带配对 PIN，调试输出只暴露待处理数量。
#[derive(Default)]
pub struct PairingManager {
    pending: HashMap<Uuid, PendingRequest>,
}

impl std::fmt::Debug for PairingManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 见 docs/development.md 第 3 条：携带敏感字段的类型手写 Debug，PIN 不进日志。
        formatter
            .debug_struct("PairingManager")
            .field("pending", &self.pending.len())
            .finish()
    }
}

impl PairingManager {
    /// 为设备创建配对请求并返回含 PIN 的请求；同名设备的旧请求会被替换。
    ///
    /// 供可信的进程内调用者使用，不可信输入请走
    /// [`PairingManager::try_request`]。
    pub fn request(
        &mut self,
        device_id: impl Into<String>,
        device_name: impl Into<String>,
    ) -> PairingRequest {
        // Kept as a compatibility helper for trusted in-process callers.  The
        // HTTP endpoint must use try_request so untrusted metadata and pending
        // count are checked before allocation.
        self.purge_expired();
        let device_id = device_id.into();
        self.pending
            .retain(|_, pending| pending.request.device_id != device_id);
        let request = PairingRequest {
            request_id: Uuid::new_v4(),
            device_id,
            device_name: device_name.into(),
            pin: format!("{:06}", uuid::Uuid::new_v4().as_u128() % 1_000_000),
            expires_at: Instant::now() + REQUEST_TTL,
        };
        self.pending.insert(
            request.request_id,
            PendingRequest {
                request: request.clone(),
                attempts: 0,
            },
        );
        request
    }

    /// Creates a request after validating attacker-controlled metadata and the
    /// bounded pending map.  Existing requests for the same device are
    /// replaced, even when the map is otherwise full.
    ///
    /// # Errors
    ///
    /// 设备标识或名称非法时返回 [`PairingError::InvalidRequest`]；挂起请求已达上限
    /// 且该设备没有既有请求时返回 [`PairingError::TooManyPending`]。
    pub fn try_request(
        &mut self,
        device_id: impl Into<String>,
        device_name: impl Into<String>,
    ) -> Result<PairingRequest, PairingError> {
        self.purge_expired();
        let device_id = device_id.into();
        let device_name = device_name.into();
        if device_id.is_empty()
            || device_id.len() > MAX_DEVICE_ID_LEN
            || device_name.is_empty()
            || device_name.len() > MAX_DEVICE_NAME_LEN
        {
            return Err(PairingError::InvalidRequest);
        }
        let existing_device = self
            .pending
            .values()
            .any(|pending| pending.request.device_id == device_id);
        if !existing_device && self.pending.len() >= MAX_PENDING_REQUESTS {
            return Err(PairingError::TooManyPending);
        }
        Ok(self.request(device_id, device_name))
    }

    /// 校验 PIN 并确认配对，成功后该请求被消耗。
    ///
    /// # Errors
    ///
    /// 请求不存在、已过期、PIN 不匹配或尝试次数超限时返回 [`PairingError`]。
    pub fn confirm(&mut self, request_id: Uuid, pin: &str) -> Result<PairingResult, PairingError> {
        self.confirm_inner(request_id, None, pin)
    }

    /// Confirms only the request belonging to `device_id`.  A mismatch is
    /// checked before PIN attempts are consumed or the request is removed.
    ///
    /// # Errors
    ///
    /// `device_id` 与请求不匹配时返回 [`PairingError::DeviceMismatch`]；
    /// 其余失败原因同 [`PairingManager::confirm`]。
    pub fn confirm_for_device(
        &mut self,
        request_id: Uuid,
        device_id: &str,
        pin: &str,
    ) -> Result<PairingResult, PairingError> {
        self.confirm_inner(request_id, Some(device_id), pin)
    }

    fn confirm_inner(
        &mut self,
        request_id: Uuid,
        expected_device_id: Option<&str>,
        pin: &str,
    ) -> Result<PairingResult, PairingError> {
        // Do not purge before looking up this request: an expired request must
        // remain observable long enough to return the specific Expired error.
        let Some(pending) = self.pending.get_mut(&request_id) else {
            return Err(PairingError::NotFound);
        };
        if pending.request.expires_at <= Instant::now() {
            self.pending.remove(&request_id);
            return Err(PairingError::Expired);
        }
        if let Some(expected_device_id) = expected_device_id {
            if expected_device_id.is_empty() || expected_device_id.len() > MAX_DEVICE_ID_LEN {
                return Err(PairingError::InvalidRequest);
            }
            if pending.request.device_id != expected_device_id {
                return Err(PairingError::DeviceMismatch);
            }
        }
        if pin.len() != PIN_LEN || !pin.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(PairingError::InvalidRequest);
        }
        if pending.attempts >= MAX_ATTEMPTS {
            return Err(PairingError::RateLimited);
        }
        if pending.request.pin != pin {
            pending.attempts = pending.attempts.saturating_add(1);
            return Err(if pending.attempts >= MAX_ATTEMPTS {
                PairingError::RateLimited
            } else {
                PairingError::InvalidPin
            });
        }
        let result = PairingResult {
            request_id,
            device_id: pending.request.device_id.clone(),
            device_name: pending.request.device_name.clone(),
            pin: pending.request.pin.clone(),
        };
        self.pending.remove(&request_id);
        Ok(result)
    }

    /// 取消指定配对请求，返回它原本是否存在。
    pub fn cancel(&mut self, request_id: Uuid) -> bool {
        self.pending.remove(&request_id).is_some()
    }

    /// 遍历当前保留的配对请求；过期项会在下次创建请求时清理。
    pub fn pending(&self) -> impl Iterator<Item = &PairingRequest> {
        self.pending.values().map(|entry| &entry.request)
    }

    fn purge_expired(&mut self) {
        let now = Instant::now();
        self.pending
            .retain(|_, pending| pending.request.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_confirmation_consumes_request() {
        let mut manager = PairingManager::default();
        let request = manager.request("phone-1", "Phone");
        let result = manager
            .confirm(request.request_id, &request.pin)
            .expect("confirm");
        assert_eq!(result.device_id, "phone-1");
        assert_eq!(
            manager.confirm(request.request_id, &request.pin),
            Err(PairingError::NotFound)
        );
    }

    #[test]
    fn wrong_code_is_limited_after_five_attempts() {
        let mut manager = PairingManager::default();
        let request = manager.request("phone-1", "Phone");
        // Avoid relying on the random PIN not being this test value.
        let wrong_pin = if request.pin == "000000" {
            "000001"
        } else {
            "000000"
        };
        for _ in 0..4 {
            assert_eq!(
                manager.confirm(request.request_id, wrong_pin),
                Err(PairingError::InvalidPin)
            );
        }
        assert_eq!(
            manager.confirm(request.request_id, wrong_pin),
            Err(PairingError::RateLimited)
        );
    }

    #[test]
    fn expired_request_returns_expired_before_being_purged() {
        let mut manager = PairingManager::default();
        let request = manager.request("phone-1", "Phone");
        manager
            .pending
            .get_mut(&request.request_id)
            .expect("pending request")
            .request
            .expires_at = Instant::now() - Duration::from_secs(1);

        assert_eq!(
            manager.confirm(request.request_id, &request.pin),
            Err(PairingError::Expired)
        );
        assert_eq!(
            manager.confirm(request.request_id, &request.pin),
            Err(PairingError::NotFound)
        );
    }

    #[test]
    fn one_device_has_one_active_request() {
        let mut manager = PairingManager::default();
        let first = manager.request("phone-1", "Phone");
        let second = manager.request("phone-1", "Phone");
        assert_eq!(manager.pending().count(), 1);
        assert_eq!(
            manager.confirm(first.request_id, &first.pin),
            Err(PairingError::NotFound)
        );
        assert!(manager.confirm(second.request_id, &second.pin).is_ok());
    }

    #[test]
    fn device_mismatch_does_not_consume_request_or_attempt() {
        let mut manager = PairingManager::default();
        let request = manager.request("phone-1", "Phone");
        assert_eq!(
            manager.confirm_for_device(request.request_id, "phone-2", "000000"),
            Err(PairingError::DeviceMismatch)
        );
        assert_eq!(manager.pending().count(), 1);
        assert!(
            manager
                .confirm_for_device(request.request_id, "phone-1", &request.pin)
                .is_ok()
        );
    }

    #[test]
    fn checked_request_rejects_unbounded_metadata_and_pending_flood() {
        let mut manager = PairingManager::default();
        assert_eq!(
            manager.try_request("", "Phone"),
            Err(PairingError::InvalidRequest)
        );
        assert_eq!(
            manager.try_request("phone-1", "x".repeat(MAX_DEVICE_NAME_LEN + 1)),
            Err(PairingError::InvalidRequest)
        );
        for index in 0..MAX_PENDING_REQUESTS {
            manager
                .try_request(format!("phone-{index}"), "Phone")
                .expect("within pending limit");
        }
        assert_eq!(
            manager.try_request("phone-overflow", "Phone"),
            Err(PairingError::TooManyPending)
        );
    }
}
