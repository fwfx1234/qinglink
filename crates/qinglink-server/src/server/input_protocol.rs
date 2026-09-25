use std::time::{SystemTime, UNIX_EPOCH};

use crate::protocol::requests::MAX_MOUSE_MOVE_DELTA;

pub const FRAME_LEN: usize = 12;
pub const PROTOCOL_VERSION: u8 = 1;
pub const EVENT_MOUSE_MOVE: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseMoveFrame {
    pub sequence: u16,
    pub dx: i16,
    pub dy: i16,
    pub timestamp_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    InvalidLength,
    InvalidVersion,
    InvalidEventType,
    OutOfRange,
}

impl MouseMoveFrame {
    pub fn parse(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() != FRAME_LEN {
            return Err(FrameError::InvalidLength);
        }
        if bytes[0] != PROTOCOL_VERSION {
            return Err(FrameError::InvalidVersion);
        }
        if bytes[1] != EVENT_MOUSE_MOVE {
            return Err(FrameError::InvalidEventType);
        }
        let dx = i16::from_le_bytes([bytes[4], bytes[5]]);
        let dy = i16::from_le_bytes([bytes[6], bytes[7]]);
        if !(-(MAX_MOUSE_MOVE_DELTA as i16)..=(MAX_MOUSE_MOVE_DELTA as i16)).contains(&dx)
            || !(-(MAX_MOUSE_MOVE_DELTA as i16)..=(MAX_MOUSE_MOVE_DELTA as i16)).contains(&dy)
        {
            return Err(FrameError::OutOfRange);
        }
        Ok(Self {
            sequence: u16::from_le_bytes([bytes[2], bytes[3]]),
            dx,
            dy,
            timestamp_ms: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        })
    }
}

pub fn now_millis_u32() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn frame() -> [u8; FRAME_LEN] {
        [1, 1, 0x34, 0x12, 0xfe, 0xff, 2, 0, 0x78, 0x56, 0x34, 0x12]
    }

    #[test]
    fn parses_little_endian_frame() {
        assert_eq!(
            MouseMoveFrame::parse(&frame()).unwrap(),
            MouseMoveFrame {
                sequence: 0x1234,
                dx: -2,
                dy: 2,
                timestamp_ms: 0x12345678,
            }
        );
    }

    #[test]
    fn rejects_invalid_frames() {
        let mut bytes = frame();
        bytes[0] = 2;
        assert_eq!(
            MouseMoveFrame::parse(&bytes),
            Err(FrameError::InvalidVersion)
        );
        bytes = frame();
        bytes[1] = 2;
        assert_eq!(
            MouseMoveFrame::parse(&bytes),
            Err(FrameError::InvalidEventType)
        );
        bytes = frame();
        bytes[4] = 0xf5;
        bytes[5] = 0x01;
        assert_eq!(MouseMoveFrame::parse(&bytes), Err(FrameError::OutOfRange));
        assert_eq!(
            MouseMoveFrame::parse(&frame()[..11]),
            Err(FrameError::InvalidLength)
        );
    }

    #[test]
    fn accepts_maximum_mouse_delta() {
        let mut bytes = frame();
        bytes[4..6].copy_from_slice(&500_i16.to_le_bytes());
        bytes[6..8].copy_from_slice(&(-500_i16).to_le_bytes());
        assert!(MouseMoveFrame::parse(&bytes).is_ok());
    }
}
