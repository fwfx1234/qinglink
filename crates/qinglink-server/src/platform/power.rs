//! 系统电源与会话控制：关机、重启、睡眠、注销、锁屏、保持唤醒，以及唤醒免密与
//! 服务退出清理。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::platform_impl;

#[cfg(target_os = "windows")]
use super::windows;

/// 系统电源与会话控制入口，提供关机、重启、休眠、注销与锁屏操作。
#[derive(Debug)]
pub struct SystemController;

impl SystemController {
    /// 关机，`force` 表示强制关闭未响应的程序，`delay_secs` 为延迟秒数。
    ///
    /// # Errors
    ///
    /// 当延迟秒数超出 Windows 可接受上限（`u32`）、启用 `SeShutdownPrivilege`
    /// 权限失败或 `InitiateSystemShutdownExW` 调用失败时返回 `Err`；非 Windows
    /// 平台恒返回不支持错误。
    pub fn shutdown(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        platform_impl::shutdown(force, delay_secs)
    }

    /// 进入睡眠；`hibernate` 为真时改为休眠。
    ///
    /// # Errors
    ///
    /// 当 `SetSuspendState` 调用失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn sleep(hibernate: bool) -> anyhow::Result<()> {
        platform_impl::sleep(hibernate)
    }

    /// 重启，`force` 表示强制关闭未响应的程序，`delay_secs` 为延迟秒数。
    ///
    /// # Errors
    ///
    /// 当延迟秒数超出 Windows 可接受上限（`u32`）、启用 `SeShutdownPrivilege`
    /// 权限失败或 `InitiateSystemShutdownExW` 调用失败时返回 `Err`；非 Windows
    /// 平台恒返回不支持错误。
    pub fn restart(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        platform_impl::restart(force, delay_secs)
    }

    /// 注销当前用户会话。
    ///
    /// # Errors
    ///
    /// 当 Session 0 下找不到活动的交互式会话、`WTSLogoffSession` 或
    /// `ExitWindowsEx` 调用失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn logoff() -> anyhow::Result<()> {
        platform_impl::logoff()
    }

    /// 锁定当前工作站。
    ///
    /// # Errors
    ///
    /// 当 Session 0 下会话代理不可用或 `LockWorkStation` 调用失败时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    pub fn lock() -> anyhow::Result<()> {
        platform_impl::lock()
    }
}

/// 注册控制台控制事件处理器，用于在服务被停止时执行清理。
///
/// # Errors
///
/// 当 `SetConsoleCtrlHandler` 注册控制台控制处理器失败时返回 `Err`。
#[cfg(target_os = "windows")]
pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    windows::install_console_ctrl_handler()
}

/// 非 Windows 平台无需注册控制台控制事件处理器。
///
/// # Errors
///
/// 当前实现恒返回 `Ok`；返回值保留是为了统一各平台的函数签名。
#[cfg(not(target_os = "windows"))]
pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    Ok(())
}

/// 关闭唤醒时需要输入密码的要求。
///
/// # Errors
///
/// 当 `powercfg` 无法启动、返回非零状态，或设置后仍未生效（唤醒仍需密码）时
/// 返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn configure_no_password_wake() -> anyhow::Result<()> {
    platform_impl::configure_no_password_wake()
}

// === 保持唤醒 ===

/// 保持唤醒的状态与命令通道。
///
/// `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)`
/// 是 **per-thread** 状态：哪个线程调用，请求就归哪个线程持有，线程退出时请求随即
/// 失效。WS 请求落在 tokio 的工作线程上，既不能保证每次都是同一条线程，也不能
/// 保证线程长期存活。因此由一条专用常驻线程串行执行所有开关，进程退出时由系统回收。
struct KeepAwake {
    /// 当前开关状态，供 [`keep_awake`] 读取。
    enabled: AtomicBool,
    /// 命令通道；`None` 表示常驻线程创建失败。
    commands: Option<Mutex<Sender<KeepAwakeCommand>>>,
}

/// 一次开关请求，`reply` 回传平台调用结果，使 WS 请求能同步拿到成败。
struct KeepAwakeCommand {
    enabled: bool,
    reply: Sender<anyhow::Result<()>>,
}

static KEEP_AWAKE: OnceLock<KeepAwake> = OnceLock::new();

fn keep_awake_state() -> &'static KeepAwake {
    KEEP_AWAKE.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<KeepAwakeCommand>();
        let commands = match std::thread::Builder::new()
            .name("qinglink-keep-awake".into())
            .spawn(move || run_keep_awake(receiver))
        {
            Ok(_) => Some(Mutex::new(sender)),
            Err(error) => {
                tracing::error!(%error, "创建保持唤醒线程失败");
                None
            }
        };
        KeepAwake {
            enabled: AtomicBool::new(false),
            commands,
        }
    })
}

/// 常驻线程：始终由同一条线程发起与释放 `SetThreadExecutionState` 请求。
fn run_keep_awake(receiver: Receiver<KeepAwakeCommand>) {
    for command in receiver {
        let result = platform_impl::set_keep_awake(command.enabled);
        let _ = command.reply.send(result);
    }
}

/// 读取服务进程当前的保持唤醒状态。
///
/// 这是桌面端与移动端共用的唯一状态源：开关的持久化与启动恢复由
/// `server::state::AppState` 负责，本模块只持有运行期请求。
pub fn keep_awake() -> bool {
    keep_awake_state().enabled.load(Ordering::Relaxed)
}

/// 开启或关闭保持唤醒。
///
/// # Errors
///
/// 常驻线程不可用、已退出、未返回结果，或 `SetThreadExecutionState` 调用失败时返回
/// `Err`；非 Windows 平台恒返回不支持错误。
pub fn set_keep_awake(enabled: bool) -> anyhow::Result<()> {
    let state = keep_awake_state();
    let commands = state
        .commands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("保持唤醒线程不可用"))?;
    let (reply_sender, reply_receiver) = mpsc::channel();
    let command = KeepAwakeCommand {
        enabled,
        reply: reply_sender,
    };
    commands
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .send(command)
        .map_err(|_| anyhow::anyhow!("保持唤醒线程已退出"))?;
    let result = reply_receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| anyhow::anyhow!("保持唤醒线程未返回结果"))?;
    result?;
    state.enabled.store(enabled, Ordering::Relaxed);
    Ok(())
}
