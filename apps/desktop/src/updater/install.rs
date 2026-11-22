//! 安装编排：启动下载好的新 exe 副本作为自更新执行器，然后让主程序退出。
//!
//! 运行中的 exe 无法覆盖自己，所以替换必须由另一个进程完成。新 exe 副本从更新目录
//! 以 `--self-update` 启动（执行器逻辑已编译进桌面端本体），等本进程退出后把自身覆盖
//! 到目标路径、重新登记服务信任并拉起新进程。因为执行器从下载目录运行，即使主 exe
//! 之后被覆盖，也不影响它继续执行。

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, ensure};

/// 更新包里桌面端 exe 的固定文件名。
const DESKTOP_EXE: &str = "QingLink.exe";

/// 校验待安装目录里有新 exe，启动自更新执行器。
///
/// `extracted_dir` 是下载后的目录；`current_exe` 是正在运行的桌面端路径，也是替换目标。
/// 新 exe 副本从该目录启动，其 `--source` 缺省为自身。
pub fn launch(extracted_dir: &Path, current_exe: &Path) -> anyhow::Result<()> {
    let new_desktop = extracted_dir.join(DESKTOP_EXE);
    ensure!(
        new_desktop.is_file(),
        "更新包缺少 {DESKTOP_EXE}，无法继续安装"
    );

    let current = current_exe.canonicalize().context("无法解析当前程序路径")?;
    let new_canon = new_desktop.canonicalize().context("无法解析新程序路径")?;
    // 不变式：更新执行器必须从下载目录的新副本启动，绝不能覆盖正在运行的自己。
    ensure!(
        new_canon != current,
        "更新源与目标相同，拒绝安装：{}",
        new_canon.display()
    );

    let parent_pid = std::process::id();
    let result = std::env::temp_dir().join(format!("qinglink-update-{parent_pid}.txt"));

    let mut command = std::process::Command::new(&new_desktop);
    command
        .arg("--self-update")
        .arg("--parent-pid")
        .arg(parent_pid.to_string())
        .arg("--target")
        .arg(&current)
        .arg("--result")
        .arg(&result)
        .current_dir(extracted_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // 更新执行器以 GUI 子系统构建，无控制台；仍设置无窗口标志以防调试构建闪窗。
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let _child = command
        .spawn()
        .with_context(|| format!("无法启动自更新执行器 {}", new_desktop.display()))?;

    // 执行器已脱离父进程独立运行，句柄无需保留。
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
