//! 纯粹的帧编解码与有界 JSON 转换，不依赖 Windows 句柄。

use std::io::{self, Write};

use anyhow::{Result, bail, ensure};
use serde::{Serialize, de::DeserializeOwned};

use super::MAX_FRAME_SIZE;

pub(super) fn encode_length(length: usize) -> Result<[u8; 4]> {
    ensure!(length != 0, "pipe frame payload must not be empty");
    ensure!(
        length <= MAX_FRAME_SIZE,
        "pipe frame payload exceeds 64 KiB"
    );
    Ok((length as u32).to_le_bytes())
}

pub(super) fn decode_length(header: &[u8]) -> Result<usize> {
    let header: [u8; 4] = header
        .try_into()
        .map_err(|_| anyhow::anyhow!("truncated pipe frame length header"))?;
    let length = u32::from_le_bytes(header) as usize;
    encode_length(length)?;
    Ok(length)
}

pub(super) fn serialize(value: &(impl Serialize + ?Sized)) -> Result<Vec<u8>> {
    // Bound allocation while serializing rather than allocating an arbitrarily
    // large JSON document before checking its length.
    let mut output = LimitedWriter {
        bytes: Vec::new(),
        exceeded: false,
    };
    if serde_json::to_writer(&mut output, value).is_err() {
        if output.exceeded {
            bail!("pipe JSON payload exceeds 64 KiB");
        }
        // Custom Serialize implementations may put credentials in error strings.
        bail!("cannot serialize pipe JSON payload");
    }
    encode_length(output.bytes.len())?;
    Ok(output.bytes)
}

pub(super) fn deserialize<T: DeserializeOwned>(payload: &[u8]) -> Result<T> {
    encode_length(payload.len())?;
    serde_json::from_slice(payload).map_err(|error| {
        // In particular, serde's invalid-value errors can quote secret values.
        anyhow::anyhow!(
            "cannot deserialize pipe JSON payload (line {}, column {})",
            error.line(),
            error.column()
        )
    })
}

struct LimitedWriter {
    bytes: Vec<u8>,
    exceeded: bool,
}

impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_FRAME_SIZE.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("pipe JSON payload exceeds 64 KiB"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_complete_frame<T: DeserializeOwned>(frame: &[u8]) -> Result<T> {
        let header = frame
            .get(..4)
            .ok_or_else(|| anyhow::anyhow!("truncated pipe frame length header"))?;
        let length = decode_length(header)?;
        let payload = frame
            .get(4..4 + length)
            .ok_or_else(|| anyhow::anyhow!("truncated pipe frame payload"))?;
        ensure!(frame.len() == 4 + length, "unexpected trailing frame bytes");
        deserialize(payload)
    }

    #[test]
    fn length_is_little_endian_and_accepts_exact_limit() -> Result<()> {
        assert_eq!(encode_length(256)?, [0, 1, 0, 0]);
        assert_eq!(encode_length(MAX_FRAME_SIZE)?, [0, 0, 1, 0]);
        assert_eq!(decode_length(&[0, 0, 1, 0])?, MAX_FRAME_SIZE);
        Ok(())
    }

    #[test]
    fn rejects_empty_oversized_and_truncated_lengths() {
        for header in [vec![], vec![1], vec![1, 0], vec![1, 0, 0]] {
            assert!(decode_length(&header).is_err());
        }
        assert!(decode_length(&[0, 0, 0, 0]).is_err());
        assert!(decode_length(&[1, 0, 1, 0]).is_err());
        assert!(decode_length(&u32::MAX.to_le_bytes()).is_err());
        assert!(encode_length(MAX_FRAME_SIZE + 1).is_err());
        assert!(encode_length(usize::MAX).is_err());
    }

    #[test]
    fn frame_round_trip_and_every_truncation() -> Result<()> {
        let value = vec!["hello", "世界"];
        let payload = serialize(&value)?;
        let mut frame = encode_length(payload.len())?.to_vec();
        frame.extend_from_slice(&payload);
        assert_eq!(decode_complete_frame::<Vec<String>>(&frame)?, value);
        for length in 0..frame.len() {
            assert!(decode_complete_frame::<Vec<String>>(&frame[..length]).is_err());
        }
        frame.push(0);
        assert!(decode_complete_frame::<Vec<String>>(&frame).is_err());
        Ok(())
    }

    #[test]
    fn serialization_enforces_limit_including_json_escaping() -> Result<()> {
        let at_limit = "a".repeat(MAX_FRAME_SIZE - 2);
        assert_eq!(serialize(&at_limit)?.len(), MAX_FRAME_SIZE);
        assert!(serialize(&"a".repeat(MAX_FRAME_SIZE - 1)).is_err());
        assert!(serialize(&"\n".repeat(MAX_FRAME_SIZE / 2)).is_err());
        Ok(())
    }

    #[test]
    fn rejects_invalid_json_and_does_not_echo_secret_values() {
        assert!(deserialize::<serde_json::Value>(b"{\"unfinished\":").is_err());
        let result = deserialize::<u32>(b"\"sensitive-test-value\"");
        let Err(error) = result else {
            panic!("a string cannot deserialize as u32");
        };
        assert!(!error.to_string().contains("sensitive-test-value"));
        assert!(deserialize::<serde_json::Value>(&vec![b' '; MAX_FRAME_SIZE + 1]).is_err());
    }
}
