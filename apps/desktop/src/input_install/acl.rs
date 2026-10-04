use std::{path::Path, process::Command};

use anyhow::{Context, Result, ensure};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub(crate) fn secure_install_directory(path: &Path) -> Result<()> {
    secure_directory(
        path,
        &[
            "*S-1-5-18:(OI)(CI)(F)",
            "*S-1-5-32-544:(OI)(CI)(F)",
            "*S-1-5-4:(OI)(CI)(RX)",
        ],
    )
}

fn secure_directory(path: &Path, grants: &[&str]) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let path = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("目录路径包含无法处理的字符"))?;
        run_icacls(&[path, "/reset", "/T", "/C"])?;
        run_icacls(&[path, "/inheritance:r", "/T", "/C"])?;
        let mut arguments = vec![path, "/grant:r"];
        arguments.extend_from_slice(grants);
        arguments.extend_from_slice(&["/T", "/C"]);
        run_icacls(&arguments)?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, grants);
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn run_icacls(arguments: &[&str]) -> Result<()> {
    let output = Command::new("icacls.exe")
        .args(arguments)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("无法运行 icacls.exe")?;
    ensure!(
        output.status.success(),
        "icacls.exe 失败：{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
