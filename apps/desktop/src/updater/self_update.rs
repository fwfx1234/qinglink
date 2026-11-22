//! 自更新执行器：新 exe 副本以 `--self-update` 启动，替换已安装的旧 exe。
//!
//! Windows 下运行中的 exe 无法被覆盖，所以必须由另一个进程完成替换。这里复用了
//! 项目已有的模式（`--restart-parent`）：下载好的新 `QingLink.exe` 副本从更新目录
//! 启动，等待旧进程退出后覆盖目标、提权重登记服务信任，再拉起新进程。
//!
//! 与旧的独立 `QingLinkUpdater.exe` 相比，执行器逻辑已经并入桌面端本体，发布资产
//! 因此不再需要单独一个更新器 exe。
//!
//! 结果通过 `--result` 文件回传：**先写结果、后拉起新进程**，避免新进程启动时结果
//! 尚未落盘；新进程在 `bootstrap` 阶段消费该文件，把失败原因推送到更新状态。

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};

use super::SharedUpdateState;

/// 命令行参数：由 `install::launch` 构造，经 `cli::classify` 在启动早期分发。
#[derive(Debug)]
struct SelfUpdateArgs {
    parent_pid: u32,
    source: PathBuf,
    target: PathBuf,
    result: PathBuf,
}

/// 检测 `--self-update` 并就地执行自更新；不是自更新调用则返回 `None`。
///
/// 本函数在任何子系统装配之前运行（`cli::classify` 的 preflight 阶段），因此不会
/// 占用单实例、不会启动 Web 或服务，适合做"等待旧进程退出 → 替换文件"这类操作。
pub fn handle_self_update_command(args: &[String]) -> Option<Result<()>> {
    if !args.iter().any(|arg| arg == "--self-update") {
        return None;
    }
    Some(execute(args))
}

fn execute(args: &[String]) -> Result<()> {
    let options = parse_args(args)?;
    let source = options
        .source
        .canonicalize()
        .context("无法解析更新源路径")?;
    let target = options
        .target
        .canonicalize()
        .context("无法解析更新目标路径")?;
    // 不变式：更新器进程必须从更新目录里的新副本启动，绝不能覆盖正在运行的自己。
    ensure!(
        source != target,
        "更新源与目标相同，拒绝自覆盖：{}",
        source.display()
    );

    let outcome = run(options.parent_pid, &source, &target);
    let text = match &outcome {
        Ok(()) => "ok".to_string(),
        Err(error) => format!("error:{error:#}"),
    };
    // 关键顺序：先写结果文件，再拉起新进程，保证新进程读取时结果已完整落盘。
    let _ = std::fs::write(&options.result, text);
    let _ = relaunch(&target, &options.result);
    outcome
}

/// 解析 `--parent-pid N [--source PATH] --target PATH --result PATH`。
fn parse_args(args: &[String]) -> Result<SelfUpdateArgs> {
    let get = |flag: &str| -> Result<String> {
        args.iter()
            .position(|item| item == flag)
            .and_then(|index| args.get(index + 1))
            .cloned()
            .with_context(|| format!("缺少参数 {flag}"))
    };
    let parent_pid = get("--parent-pid")?
        .parse::<u32>()
        .context("--parent-pid 不是有效数字")?;
    // --source 缺省为当前 exe：新副本以自身为更新源。
    let source = match get("--source") {
        Ok(value) => PathBuf::from(value),
        Err(_) => std::env::current_exe().context("无法读取当前程序路径")?,
    };
    Ok(SelfUpdateArgs {
        parent_pid,
        source,
        target: PathBuf::from(get("--target")?),
        result: PathBuf::from(get("--result")?),
    })
}

/// 等待父进程退出 → 覆盖目标 exe → 提权重登记服务信任。
fn run(parent_pid: u32, source: &Path, target: &Path) -> Result<()> {
    wait_for_exit(parent_pid);
    replace_exe(source, target)?;
    reregister_service(target)?;
    Ok(())
}

/// 轮询等待父进程退出，最多约 30 秒；超时后继续（旧进程可能已僵死）。
fn wait_for_exit(pid: u32) {
    for _ in 0..300 {
        if !process_alive(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "windows")]
fn process_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    // SAFETY: OpenProcess 以受限查询权限打开给定 PID；句柄在同一作用域内由 CloseHandle 释放。
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let alive = !handle.is_invalid();
        let _ = CloseHandle(handle);
        alive
    }
}

#[cfg(not(target_os = "windows"))]
fn process_alive(_pid: u32) -> bool {
    false
}

/// 把新 exe 覆盖到目标路径。先写同目录暂存文件，再原子替换，避免写一半损坏目标。
#[cfg(target_os = "windows")]
fn replace_exe(source: &Path, target: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;

    let staged = target.with_extension("qinglink-new");
    copy_with_retry(source, &staged)?;
    let staged_wide: Vec<u16> = staged
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let target_wide: Vec<u16> = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: 两个路径均为以 NUL 结尾的合法 UTF-16 缓冲区；MoveFileExW 以替换模式
    // 原子覆盖目标，失败时原目标保持完好。
    unsafe {
        MoveFileExW(
            PCWSTR(staged_wide.as_ptr()),
            PCWSTR(target_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| anyhow::anyhow!("原子替换桌面端失败：{error}"))?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_exe(source: &Path, target: &Path) -> Result<()> {
    copy_with_retry(source, target)
}

/// 带占用重试的文件复制（旧进程刚退出时句柄可能尚未完全释放）。
fn copy_with_retry(source: &Path, destination: &Path) -> Result<()> {
    ensure!(source.is_file(), "新桌面端不存在：{}", source.display());
    let mut attempt = 0u32;
    let last = loop {
        match std::fs::copy(source, destination) {
            Ok(_) => return Ok(()),
            Err(_) if attempt < 29 => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(error) => break error,
        }
    };
    Err(last).context("覆盖桌面端失败：目标文件长时间被占用")
}

/// 提权重跑"修复服务安装"，登记新 exe 的 SHA256 并刷新内嵌的服务二进制。
fn reregister_service(target: &Path) -> Result<()> {
    let result_file = std::env::temp_dir().join(format!(
        "qinglink-update-register-{}.txt",
        std::process::id()
    ));
    let args = format!(
        "--input-component install --result \"{}\"",
        result_file.display()
    );
    let result = qinglink_windows::elevation::run_elevated(&target.to_string_lossy(), &args);
    let detail = std::fs::read_to_string(&result_file).unwrap_or_default();
    let _ = std::fs::remove_file(&result_file);
    match result {
        Ok(()) => {
            if let Some(error) = detail.strip_prefix("error:") {
                bail!("重新登记服务失败：{error}");
            }
            Ok(())
        }
        Err(qinglink_windows::elevation::ElevationError::Cancelled) => {
            bail!("用户取消了服务重新登记的 UAC 授权")
        }
        Err(error) => bail!("重新登记服务失败：{error}"),
    }
}

/// 拉起新桌面端，并带上结果文件路径供其启动时消费。
fn relaunch(target: &Path, result: &Path) -> Result<()> {
    std::process::Command::new(target)
        .arg("--update-result")
        .arg(result)
        .spawn()
        .with_context(|| format!("无法启动新桌面端 {}", target.display()))?;
    Ok(())
}

/// 新进程启动时消费更新结果文件：删文件，失败则推送到更新状态提示用户。
///
/// 返回是否消费到了错误（供调用方决定是否跳过启动时的自动检查，避免错误提示被冲掉）。
pub fn consume_update_result(state: &SharedUpdateState, path: Option<&PathBuf>) -> bool {
    let Some(path) = path else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let _ = std::fs::remove_file(path);
    if text == "ok" {
        return false;
    }
    let detail = text
        .strip_prefix("error:")
        .unwrap_or(&text)
        .trim()
        .to_string();
    let detail = if detail.is_empty() {
        "未知错误".to_string()
    } else {
        detail
    };
    state.fail(format!(
        "更新已完成，但服务重新登记失败：{detail}。请在设置中修复服务安装。"
    ));
    true
}
