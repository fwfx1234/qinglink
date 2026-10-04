//! 会话控制：注销、锁定、解锁状态查询、自动解锁与免密码唤醒配置。

use crate::platform::SystemController;

use super::SystemService;

impl SystemService {
    /// 注销当前用户会话。
    ///
    /// # Errors
    /// 在 Session 0 服务中取不到活动交互式会话、`WTSLogoffSession` 调用失败，
    /// 或 `ExitWindowsEx` 调用失败时返回 `Err`；非 Windows 平台恒返回
    /// “仅 Windows 支持”错误。
    pub fn logoff(&self) -> anyhow::Result<()> {
        SystemController::logoff()
    }

    /// 锁定当前用户会话。
    ///
    /// # Errors
    /// 在 Session 0 服务中会话代理不可用（`SESSION_AGENT_UNAVAILABLE`），
    /// 或 `LockWorkStation` 调用失败时返回 `Err`；非 Windows 平台恒返回
    /// “仅 Windows 支持”错误。
    pub fn lock(&self) -> anyhow::Result<()> {
        SystemController::lock()
    }

    /// 查询当前会话的锁定 / 解锁状态。
    ///
    /// # Errors
    /// Windows 实现当前不返回 `Err`：锁屏探测全部是尽力而为的，失败时退化为
    /// 保守取值（保留 `Result` 仅为接口稳定）。非 Windows 平台（存根）恒返回
    /// “仅 Windows 支持”错误。
    pub fn get_unlock_status(&self) -> anyhow::Result<crate::protocol::responses::UnlockStatus> {
        crate::platform::get_unlock_status()
    }

    /// 用给定 PIN 自动解锁当前会话；`None` 表示使用已保存的凭据。
    ///
    /// # Errors
    /// PIN 格式非法（`INVALID_PIN_FORMAT`，非 4–32 位数字）、当前是 UAC 提示
    /// （`UAC_PROMPT_ACTIVE`）、安全桌面输入代理未就绪
    /// （`SESSION_AGENT_NOT_READY`）、发送唤醒 / 确认按键失败、提交 PIN 后系统
    /// 仍未解锁（`PIN_SENT_NOT_UNLOCKED`）或等待解锁超时时返回 `Err`；
    /// 非 Windows 平台恒返回“仅 Windows 支持”错误。
    pub fn auto_unlock(&self, pin: Option<&str>) -> anyhow::Result<()> {
        crate::platform::auto_unlock(pin)
    }

    /// 配置唤醒后免密码登录所需的系统设置。
    ///
    /// # Errors
    /// 执行 `powercfg` 失败（返回非零状态）或系统未应用免密码唤醒设置时返回
    /// `Err`；非 Windows 平台恒返回“仅 Windows 支持”错误。
    pub fn configure_no_password_wake(&self) -> anyhow::Result<()> {
        crate::platform::configure_no_password_wake()
    }
}
