//! 输入服务管道协议：查询输入服务状态与提交锁屏 PIN。

use std::io::{Read, Write};

use anyhow::Context;

use super::auto_unlock::wait_for_session_unlock;
use super::session_lock::{is_secure_desktop, query_lock_state};

const LOCK_INPUT_PIPE: &str = r"\\.\pipe\QingLinkControlV2";

#[derive(Clone, serde::Serialize)]
pub(super) struct LockInputPipeRequest<'a> {
    pub(super) version: u32,
    pub(super) command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) pin: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) dx: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) dy: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) button: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) pressed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) delta: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) usage: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) modifiers: Option<u8>,
}

#[derive(serde::Deserialize)]
pub(super) struct LockInputPipeResponse {
    pub(super) ok: bool,
    pub(super) code: String,
    #[serde(default)]
    pub(super) message: String,
}

pub(super) fn lock_input_pipe(
    request: &LockInputPipeRequest<'_>,
) -> anyhow::Result<LockInputPipeResponse> {
    let mut pipe = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(LOCK_INPUT_PIPE)
        .map_err(|_| anyhow::anyhow!("LOCK_INPUT_SERVICE_UNAVAILABLE: 输入服务未运行"))?;
    let payload = serde_json::to_vec(request)?;
    pipe.write_all(&(payload.len() as u32).to_le_bytes())?;
    pipe.write_all(&payload)?;
    pipe.flush()?;
    let mut len = [0u8; 4];
    pipe.read_exact(&mut len)?;
    anyhow::ensure!(
        len[0] as usize
            | ((len[1] as usize) << 8)
            | ((len[2] as usize) << 16)
            | ((len[3] as usize) << 24)
            <= 64 * 1024,
        "LOCK_INPUT_INVALID_RESPONSE: 输入服务响应过大"
    );
    let mut response = vec![0u8; u32::from_le_bytes(len) as usize];
    pipe.read_exact(&mut response)?;
    Ok(serde_json::from_slice(&response)?)
}

/// 查询锁屏输入辅助服务是否可用及其运行状态。
///
/// # Errors
/// 输入服务管道不可用（`LOCK_INPUT_SERVICE_UNAVAILABLE`）不算错误，会返回
/// `available: false, service_running: false`；但当管道已建立而读写或 JSON 解析
/// 失败（请求序列化、写长度/负载、刷新、读响应或反序列化响应体失败）时返回错误。
pub fn get_lock_input_status() -> anyhow::Result<qinglink_protocol::LockInputStatus> {
    let response = lock_input_pipe(&LockInputPipeRequest {
        version: 1,
        command: "status",
        pin: None,
        dx: None,
        dy: None,
        button: None,
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    });
    match response {
        Ok(response) => Ok(qinglink_protocol::LockInputStatus {
            available: response.ok,
            service_running: true,
            message: response.message,
        }),
        Err(error)
            if error
                .to_string()
                .starts_with("LOCK_INPUT_SERVICE_UNAVAILABLE") =>
        {
            Ok(qinglink_protocol::LockInputStatus {
                available: false,
                service_running: false,
                message: error.to_string(),
            })
        }
        Err(error) => Err(error),
    }
}

/// 向锁屏提交 4-32 位数字 PIN 以尝试解锁，并返回注入结果。
///
/// # Errors
/// PIN 长度不在 4-32 位或含非数字字符（“INVALID_PIN_FORMAT: PIN 必须是 4-32
/// 位数字”）；`is_secure_desktop()` 探测失败；安全桌面路径下注入 PIN 或确认按键
/// 失败（带上下文“SECURE_INPUT_FAILED: …”），或 5 秒后 Windows 仍停留在锁屏安全
/// 桌面（“SECURE_INPUT_NOT_ACCEPTED: PIN 已发送，但 Windows 仍处于锁屏安全桌面”）；
/// 服务管道路径下管道不可用/读写或解析失败、服务返回 `ok == false`
/// （“{code}: {message}”），或 8 秒内仍未解锁
/// （“LOGIN_SURFACE_NOT_READY: 解锁操作未完成，可能需要 PIN 或密码，请使用控制页输入”）。
pub fn submit_lock_pin(pin: &str) -> anyhow::Result<qinglink_protocol::LockInputResult> {
    anyhow::ensure!(
        (4..=32).contains(&pin.len()) && pin.bytes().all(|byte| byte.is_ascii_digit()),
        "INVALID_PIN_FORMAT: PIN 必须是 4-32 位数字"
    );
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        // A secure-desktop request must stay on the SYSTEM session-agent
        // route. Never retry through the service pipe, which would make a
        // failed injection look successful while Session 0 is unable to see
        // Winlogon.
        crate::secure_desktop::send_secure_ascii(pin)
            .context("SECURE_INPUT_FAILED: 无法向安全桌面发送 PIN")?;
        crate::secure_desktop::send_hid_key(0x28, 0, true)
            .context("SECURE_INPUT_FAILED: 无法发送确认按键")?;
        crate::secure_desktop::send_hid_key(0x28, 0, false)
            .context("SECURE_INPUT_FAILED: 无法释放确认按键")?;

        // SendInput returning success only means that Windows accepted the
        // event. Confirm that the lock surface is no longer up; an unknown
        // state is treated as accepted so a detection blind spot does not
        // report a delivered PIN as failed.
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if query_lock_state() != Some(true) {
                return Ok(qinglink_protocol::LockInputResult {
                    code: "SECURE_INPUT_ACCEPTED".into(),
                    message: "PIN 已通过安全桌面输入并完成解锁".into(),
                });
            }
        }
        anyhow::bail!("SECURE_INPUT_NOT_ACCEPTED: PIN 已发送，但 Windows 仍处于锁屏安全桌面")
    }
    let response = lock_input_pipe(&LockInputPipeRequest {
        version: 1,
        command: "submit_pin",
        pin: Some(pin),
        dx: None,
        dy: None,
        button: None,
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    })?;
    anyhow::ensure!(response.ok, "{}: {}", response.code, response.message);
    wait_for_session_unlock(std::time::Duration::from_secs(8))?;
    Ok(qinglink_protocol::LockInputResult {
        code: response.code,
        message: response.message,
    })
}
