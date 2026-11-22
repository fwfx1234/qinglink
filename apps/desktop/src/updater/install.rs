//! 安装编排：启动独立更新器，然后让主程序退出，把替换 exe 的工作交给更新器。
//!
//! 运行中的 exe 无法覆盖自己，所以替换为独立 `QingLinkUpdater.exe` 完成：它从更新包
//! 解压目录启动，等本进程退出后把新 exe 覆盖到目标路径、重新登记服务信任并拉起新进程。

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, ensure};

/// 更新包里更新器与桌面端 exe 的固定文件名。
const UPDATER_EXE: &str = "QingLinkUpdater.exe";
const DESKTOP_EXE: &str = "QingLink.exe";

/// 校验待安装目录齐全，启动更新器。
///
/// `extracted_dir` 是下载后的目录；`current_exe` 是正在运行的桌面端路径，也是替换目标。
/// 更新器从该目录启动，因此即使主 exe 之后被覆盖，也不影响它继续运行。
pub fn launch(extracted_dir: &Path, current_exe: &Path) -> anyhow::Result<()> {
    let updater = extracted_dir.join(UPDATER_EXE);
    let new_desktop = extracted_dir.join(DESKTOP_EXE);
    ensure!(updater.is_file(), "更新包缺少 {UPDATER_EXE}，无法继续安装");
    ensure!(
        new_desktop.is_file(),
        "更新包缺少 {DESKTOP_EXE}，无法继续安装"
    );

    let current = current_exe.canonicalize().context("无法解析当前程序路径")?;
    let parent_pid = std::process::id();
    let result = std::env::temp_dir().join(format!("qinglink-update-{parent_pid}.txt"));

    let mut command = std::process::Command::new(&updater);
    command
        .arg("--parent-pid")
        .arg(parent_pid.to_string())
        .arg("--source")
        .arg(&new_desktop)
        .arg("--target")
        .arg(&current)
        .arg("--result")
        .arg(&result)
        .current_dir(extracted_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // 更新器是控制台程序，直接启动会闪一个黑窗口；下载请求也做了同样处理。
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let _child = command
        .spawn()
        .with_context(|| format!("无法启动更新器 {}", updater.display()))?;

    // 更新器已脱离父进程独立运行，句柄无需保留。
    drop(_child);
    Ok(())
}

/// 更新包下载目录：数据目录下的 `updates/<版本号>`。
pub fn pending_dir(data_dir: &Path, version: &str) -> PathBuf {
    // 版本号来自 release tag，可能含 `+` 等字符；这里只用作目录名，Windows 下合法。
    data_dir.join("updates").join(sanitize(version))
}

/// 解压后的目录，与 `download::download_all` 的返回值一致。
pub fn extracted_dir(data_dir: &Path, version: &str) -> PathBuf {
    pending_dir(data_dir, version)
}

/// 把版本号收敛成安全的目录名，只保留字母数字、点与加减号。
fn sanitize(version: &str) -> String {
    let cleaned: String = version
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "unknown".to_string()
    } else {
        cleaned
    }
}
