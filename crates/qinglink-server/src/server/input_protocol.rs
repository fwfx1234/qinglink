//! 输入通道的二进制帧格式：鼠标相对位移帧的布局与解析。

use crate::protocol::requests::MAX_MOUSE_MOVE_DELTA;

/// 一帧的固定字节数。
pub const FRAME_LEN: usize = 12;
/// 二进制输入协议版本，写在首字节。
pub const PROTOCOL_VERSION: u8 = 1;
/// 鼠标相对位移事件类型，写在第二字节。
pub const EVENT_MOUSE_MOVE: u8 = 1;

/// 客户端上报的一次鼠标相对位移。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseMoveFrame {
    /// 客户端递增的序号，用于丢弃乱序帧。
    pub sequence: u16,
    /// 水平位移，范围受 `MAX_MOUSE_MOVE_DELTA` 限制。
    pub dx: i16,
    /// 垂直位移，范围受 `MAX_MOUSE_MOVE_DELTA` 限制。
    pub dy: i16,
    /// 客户端时间戳（毫秒）。
    pub timestamp_ms: u32,
}

/// 解析输入帧时的失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// 字节数不等于 [`FRAME_LEN`]。
    InvalidLength,
    /// 版本字节与 [`PROTOCOL_VERSION`] 不符。
    InvalidVersion,
    /// 事件类型字节与 [`EVENT_MOUSE_MOVE`] 不符。
    InvalidEventType,
    /// 位移超出允许范围。
    OutOfRange,
}

impl MouseMoveFrame {
    /// 从小端字节序的定长帧解析一次鼠标位移。
    ///
    /// # Errors
    /// 字节数不等于 [`FRAME_LEN`]（[`FrameError::InvalidLength`]）、版本字节不符
    /// （[`FrameError::InvalidVersion`]）、事件类型不符
    /// （[`FrameError::InvalidEventType`]），或位移超出 `MAX_MOUSE_MOVE_DELTA` 允许
    /// 范围（[`FrameError::OutOfRange`]）时返回 `Err`。
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
