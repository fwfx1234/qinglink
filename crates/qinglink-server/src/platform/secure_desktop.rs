//! 锁屏解锁、锁屏输入与安全桌面（锁屏、UAC 提示）交互。

use super::platform_impl;

use crate::protocol::responses::UnlockStatus;

/// 获取当前锁屏状态与自动登录配置。
///
/// # Errors
///
/// 当前实现恒返回 `Ok`：安全桌面状态、锁屏状态与自动登录信息的读取失败都会被
/// 降级为“未知/按可用处理”的保守结果，不会向上报错；返回值保留是为了统一接口
/// 签名。非 Windows 平台恒返回不支持错误。
pub fn get_unlock_status() -> anyhow::Result<UnlockStatus> {
    platform_impl::get_unlock_status().map(|value| UnlockStatus {
        locked: value.locked,
        mode: value.mode,
        auto_login_enabled: value.auto_login_enabled,
        message: value.message,
        uac_detected: value.uac_detected,
        surface: value.surface,
    })
}

/// 使用可选 PIN 尝试自动解锁锁屏。
///
/// # Errors
///
/// 当 PIN 不是 4-32 位数字（`INVALID_PIN_FORMAT`）、当前处于 UAC 提示、
/// 安全桌面输入代理 10 秒内未就绪（`SESSION_AGENT_NOT_READY`）、PIN 或确认键
/// 注入失败，或 8 秒内仍处于锁屏状态（`PIN_SENT_NOT_UNLOCKED` /
/// `LOGIN_SURFACE_NOT_READY`）时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn auto_unlock(pin: Option<&str>) -> anyhow::Result<()> {
    platform_impl::auto_unlock(pin)
}

/// 查询锁屏界面输入通道是否可用。
///
/// # Errors
///
/// 当输入服务管道存在但请求写入或响应解析失败时返回 `Err`；输入服务未运行时
/// 会被转换成“服务未运行”的成功结果（`service_running = false`）。非 Windows
/// 平台恒返回不支持错误。
pub fn get_lock_input_status() -> anyhow::Result<crate::protocol::responses::LockInputStatus> {
    platform_impl::get_lock_input_status().map(|value| {
        crate::protocol::responses::LockInputStatus {
            available: value.available,
            service_running: value.service_running,
            message: value.message,
        }
    })
}

/// 向锁屏界面提交 PIN 码。
///
/// # Errors
///
/// 当 PIN 不是 4-32 位数字（`INVALID_PIN_FORMAT`）、安全桌面注入失败
/// （`SECURE_INPUT_FAILED`）、发送后 5 秒内仍停留在锁屏安全桌面
/// （`SECURE_INPUT_NOT_ACCEPTED`）、输入服务不可用或被拒绝时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn submit_lock_pin(pin: &str) -> anyhow::Result<crate::protocol::responses::LockInputResult> {
    platform_impl::submit_lock_pin(pin).map(|value| crate::protocol::responses::LockInputResult {
        code: value.code,
        message: value.message,
    })
}

/// 获取安全桌面（锁屏、UAC 提示）的当前状态。
///
/// # Errors
///
/// 当非 Session 0 路径下附着或切换输入桌面失败时返回 `Err`；Session 0 下会话
/// 代理查询失败与输入探测失败只会写入返回结构的状态字段，不会报错。非 Windows
/// 平台恒返回不支持错误。
pub fn get_secure_desktop_status() -> anyhow::Result<crate::protocol::responses::SecureDesktopStatus>
{
    platform_impl::get_secure_desktop_status().map(|value| {
        crate::protocol::responses::SecureDesktopStatus {
            desktop: value.desktop,
            locked: value.locked,
            input_ready: value.input_ready,
            sas_supported: value.sas_supported,
            session_id: value.session_id,
            agent_state: value.agent_state,
            uac_detected: value.uac_detected,
            token_integrity_level: value.token_integrity_level,
            last_error: value.last_error,
        }
    })
}

/// 向安全桌面发送纯 ASCII 文本。
///
/// # Errors
///
/// 当文本为空或超过 256 字节、含非可打印 ASCII 字符、当前键盘布局无法映射某个
/// 字符、会话代理不可用或注入失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn send_secure_ascii(text: &str) -> anyhow::Result<()> {
    platform_impl::send_secure_ascii(text)
}

/// 向安全桌面发送组合键唤醒信号（Ctrl+Alt+Del 等效操作）。
///
/// # Errors
///
/// Windows 实现恒返回 `Ok(())`：读写 `SoftwareSASGeneration` 注册表项与调用
/// `SendSAS` 的失败都会被忽略，返回值保留是为了统一接口签名。非 Windows 平台
/// 恒返回不支持错误。
pub fn send_secure_attention() -> anyhow::Result<()> {
    platform_impl::send_secure_attention()
}

/// 判断当前是否处于安全桌面。
///
/// # Errors
///
/// 当读取安全桌面状态失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn is_secure_desktop() -> anyhow::Result<bool> {
    platform_impl::is_secure_desktop()
}
