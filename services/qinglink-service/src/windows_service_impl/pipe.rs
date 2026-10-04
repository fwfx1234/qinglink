//! 仅限本地的重叠管道端点，句柄唯一归属。
use anyhow::{Context, Result, ensure};
use qinglink_windows::ipc::PipeConnection;
use serde::{Serialize, de::DeserializeOwned};
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::Storage::FileSystem::{FILE_FLAG_OVERLAPPED, FILE_FLAGS_AND_ATTRIBUTES};
use windows::Win32::System::Pipes::{
    CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::core::PCWSTR;

pub(super) struct Reply {
    pipe: PipeConnection,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    request_id: Option<String>,
}

impl Reply {
    pub(super) fn handle(&self) -> HANDLE {
        self.pipe.raw_handle()
    }

    pub(super) fn read_request<T: DeserializeOwned>(&mut self) -> Result<T> {
        let mut value: serde_json::Value = self.pipe.read_json(self.deadline, &self.cancelled)?;
        let map = value
            .as_object_mut()
            .context("request must be a JSON object")?;
        self.request_id = map
            .get("request_id")
            .and_then(|value| value.as_str())
            .map(str::to_owned);
        ensure!(
            self.request_id.as_ref().is_none_or(|id| id.len() <= 128),
            "request_id too long"
        );
        decode_request(value)
    }

    pub(super) fn send(&mut self, value: &impl Serialize) -> Result<()> {
        let mut value = serde_json::to_value(value).context("serialize service response")?;
        if let Some(map) = value.as_object_mut() {
            map.insert("version".into(), 2.into());
            if let Some(id) = &self.request_id {
                map.insert("request_id".into(), id.clone().into());
            }
        }
        self.pipe.write_json(&value, self.deadline, &self.cancelled)
    }
}

fn decode_request<T: DeserializeOwned>(mut value: serde_json::Value) -> Result<T> {
    let map = value
        .as_object_mut()
        .context("request must be a JSON object")?;
    if let Some(payload) = map.remove("payload") {
        let payload = payload
            .as_object()
            .context("payload must be a JSON object")?;
        for (key, value) in payload {
            ensure!(
                !matches!(key.as_str(), "version" | "command" | "request_id"),
                "payload contains reserved header"
            );
            ensure!(!map.contains_key(key), "ambiguous duplicated request field");
            map.insert(key.clone(), value.clone());
        }
    }
    serde_json::from_value(value).map_err(|_| anyhow::anyhow!("invalid request fields"))
}

pub(super) fn accept(name: &str, sddl: &str, cancelled: Arc<AtomicBool>) -> Result<Reply> {
    let name: Vec<u16> = name.encode_utf16().chain([0]).collect();
    let sddl: Vec<u16> = sddl.encode_utf16().chain([0]).collect();
    let mut descriptor = windows::Win32::Security::PSECURITY_DESCRIPTOR::default();
    // SAFETY: terminated UTF-16 and writable output; descriptor is freed below.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )?;
    }
    let security = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    // SAFETY: the security descriptor is valid throughout CreateNamedPipeW.
    let handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(3) | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            255,
            64 * 1024,
            64 * 1024,
            0,
            Some(&security),
        )
    };
    let error = windows::core::Error::from_win32();
    // SAFETY: descriptor was allocated by the conversion API, not by Rust.
    unsafe {
        LocalFree(Some(HLOCAL(descriptor.0)));
    }
    if handle.is_invalid() {
        return Err(error).context("create local service pipe");
    }
    // SAFETY: the new valid endpoint has one owner and no outstanding I/O.
    let owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    // Idle accepts are cancellable by SCM; a connected client's complete frame
    // exchange has its own three-second budget, independent of idle wait time.
    let pipe = unsafe {
        PipeConnection::accept(
            owned,
            Instant::now() + Duration::from_secs(86400),
            &cancelled,
        )?
    };
    Ok(Reply {
        pipe,
        deadline: Instant::now() + Duration::from_secs(3),
        cancelled,
        request_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn flat_and_enveloped_requests_share_one_normalized_form() {
        let flat: Value = decode_request(json!({"version":2,"command":"set_paused","paused":true}))
            .expect("flat");
        let wrapped: Value =
            decode_request(json!({"version":2,"command":"set_paused","payload":{"paused":true}}))
                .expect("wrapped");
        assert_eq!(flat, wrapped);
    }
    #[test]
    fn malformed_payload_and_header_shadowing_are_rejected() {
        for payload in [
            json!(null),
            json!("x"),
            json!([]),
            json!({"version":1}),
            json!({"command":"exit"}),
            json!({"request_id":"shadow"}),
            json!({"paused":false}),
        ] {
            assert!(
                decode_request::<Value>(
                    json!({"version":2,"command":"set_paused","paused":true,"payload":payload})
                )
                .is_err()
            );
        }
    }
}
