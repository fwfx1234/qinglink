//! QingLink 桌面端自更新助手。
//!
//! 由主程序在下载并校验完更新包后启动，参数见 [`Options::parse`]。它做四件事：
//!
//! 1. 等待父进程（旧桌面端）退出——运行中的 exe 在 Windows 上无法被覆盖；
//! 2. 把新 exe 复制到目标路径，带占用重试；
//! 3. 提权重跑"修复服务安装"——服务登记了桌面端 exe 的 SHA256，不重登记新 exe 会被拒绝；
//! 4. 拉起新桌面端后自行退出。
//!
//! 任何一步失败都写入 `--result` 文件（供诊断），但**仍尝试拉起目标 exe**，避免更新后
//! 程序起不来、用户只能手动找回。

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};

/// 命令行参数。
#[derive(Debug)]
struct Options {
    parent_pid: u32,
    source: PathBuf,
    target: PathBuf,
    result: PathBuf,
}

fn main() {
    let options = match Options::parse(std::env::args()) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("QingLink 更新器参数错误：{error:#}");
            std::process::exit(2);
        }
    };

    let outcome = run(&options);
    let _ = std::fs::write(&options.result, format!("{outcome:?}"));

    // 无论上一步结果如何都尝试拉起目标；失败只影响服务信任，不影响主程序可用。
    let _ = relaunch(&options.target);
    if let Err(error) = outcome {
        eprintln!("QingLink 更新失败：{error:#}");
        std::process::exit(1);
    }
}

fn run(options: &Options) -> Result<()> {
    wait_for_exit(options.parent_pid);
    replace_exe(&options.source, &options.target)?;
    reregister_service(&options.target)?;
    Ok(())
}

/// 解析 `--parent-pid N --source PATH --target PATH --result PATH`。
impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let values: Vec<String> = args.into_iter().collect();
        let get = |flag: &str| -> Result<String> {
            values
                .iter()
                .position(|item| item == flag)
                .and_then(|index| values.get(index + 1))
                .cloned()
                .with_context(|| format!("缺少参数 {flag}"))
        };
        let parent_pid = get("--parent-pid")?
            .parse::<u32>()
            .context("--parent-pid 不是有效数字")?;
        Ok(Self {
            parent_pid,
            source: PathBuf::from(get("--source")?),
            target: PathBuf::from(get("--target")?),
            result: PathBuf::from(get("--result")?),
        })
    }
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
    // SAFETY: OpenProcess 返回句柄，CloseHandle 在作用域结束时释放；参数均为有效值。
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

/// 把新 exe 覆盖到目标路径，文件被占用时重试。
fn replace_exe(source: &Path, target: &Path) -> Result<()> {
    ensure!(source.is_file(), "新桌面端不存在: {}", source.display());
    let mut attempt = 0u32;
    let last = loop {
        match std::fs::copy(source, target) {
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

/// 拉起新桌面端。
fn relaunch(target: &Path) -> Result<()> {
    std::process::Command::new(target)
        .spawn()
        .with_context(|| format!("无法启动新桌面端 {}", target.display()))?;
    Ok(())
}
