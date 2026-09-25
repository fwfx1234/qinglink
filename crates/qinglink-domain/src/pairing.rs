use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use thiserror::Error;
use uuid::Uuid;

const REQUEST_TTL: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u8 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingRequest {
    pub request_id: Uuid,
    pub device_id: String,
    pub device_name: String,
    pub pin: String,
    pub expires_at: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingResult {
    pub request_id: Uuid,
    pub device_id: String,
    pub device_name: String,
    pub pin: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PairingError {
    #[error("pairing request not found")]
    NotFound,
    #[error("pairing request expired")]
    Expired,
    #[error("invalid pairing code")]
    InvalidPin,
    #[error("pairing request is rate limited")]
    RateLimited,
}

struct PendingRequest {
    request: PairingRequest,
    attempts: u8,
}

#[derive(Default)]
pub struct PairingManager {
    pending: HashMap<Uuid, PendingRequest>,
}

impl PairingManager {
    pub fn request(
        &mut self,
        device_id: impl Into<String>,
        device_name: impl Into<String>,
    ) -> PairingRequest {
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

    pub fn confirm(&mut self, request_id: Uuid, pin: &str) -> Result<PairingResult, PairingError> {
        self.purge_expired();
        let Some(pending) = self.pending.get_mut(&request_id) else {
            return Err(PairingError::NotFound);
        };
        if pending.request.expires_at <= Instant::now() {
            self.pending.remove(&request_id);
            return Err(PairingError::Expired);
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

    pub fn cancel(&mut self, request_id: Uuid) -> bool {
        self.pending.remove(&request_id).is_some()
    }

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
        for _ in 0..4 {
            assert_eq!(
                manager.confirm(request.request_id, "000000"),
                Err(PairingError::InvalidPin)
            );
        }
        assert_eq!(
            manager.confirm(request.request_id, "000000"),
            Err(PairingError::RateLimited)
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
}
