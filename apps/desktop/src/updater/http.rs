//! 极简 HTTPS GET：借助 Windows 自带的 PowerShell `Invoke-WebRequest`。
//!
//! 选择它的原因：这是纯 Windows 程序，引入 reqwest 会为 wasm 目标拖进一堆用不到的
//! 依赖（web-sys 等），而这些依赖在离线/受限网络下根本拉不下来。PowerShell 随 Windows
//! 自带，自动遵循系统代理与 TLS 设置，`-NoProfile -NonInteractive` + `CREATE_NO_WINDOW`
//! 保证无控制台闪窗、无交互卡死。下载写入文件，文本响应显式把 PowerShell 的
//! `[Console]::OutputEncoding` 设为 UTF-8：中文 Windows 上默认是 GB2312，会让
//! Release 说明里的中文在 Rust 侧按 UTF-8 解码时变成乱码。
//!
//! 所有调用都运行在后台线程；URL 由 GitHub API 给出，仍对单引号做转义以防命令注入。

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, ensure};

/// 单次请求超时（秒）。
const TIMEOUT_SECS: u64 = 300;
/// GitHub API 要求带 User-Agent，否则返回 403。
const USER_AGENT: &str = "QingLink-Updater";

/// 下载 URL 内容到文件，返回字节数。
pub fn get_to_file(url: &str, destination: &Path) -> anyhow::Result<u64> {
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $ProgressPreference='SilentlyContinue'; \
         Invoke-WebRequest -Uri '{url}' -OutFile '{dest}' -UseBasicParsing \
           -TimeoutSec {timeout} -Headers @{{'User-Agent'='{agent}'}}; \
         (Get-Item -LiteralPath '{dest}').Length",
        url = escape(url),
        dest = escape(&destination.to_string_lossy()),
        timeout = TIMEOUT_SECS,
        agent = USER_AGENT,
    );
    let output = run_powershell(&script)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let bytes = text
        .trim()
        .lines()
        .last()
        .and_then(|line| line.trim().parse::<u64>().ok())
        .unwrap_or(0);
    ensure!(
        destination.is_file(),
        "下载未生成文件: {}",
        destination.display()
    );
    Ok(bytes)
}

/// 下载 URL 的文本内容（用于 JSON API 与校验文件）。
pub fn get_text(url: &str) -> anyhow::Result<String> {
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $ProgressPreference='SilentlyContinue'; \
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         (Invoke-WebRequest -Uri '{url}' -UseBasicParsing -TimeoutSec {timeout} \
           -Headers @{{'User-Agent'='{agent}'}}).Content",
        url = escape(url),
        timeout = TIMEOUT_SECS,
        agent = USER_AGENT,
    );
    let output = run_powershell(&script)?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_powershell(script: &str) -> anyhow::Result<std::process::Output> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .stderr(Stdio::piped())
            .output()
            .context("无法启动 PowerShell 下载请求")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (script, Stdio::piped());
        anyhow::bail!("更新功能仅支持 Windows")
    }
}

/// 转义 PowerShell 单引号字符串。
fn escape(value: &str) -> String {
    value.replace('\'', "''")
}
