use anyhow::{Context, Result};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const RULE_NAME: &str = "QingLink LAN Discovery (UDP 3723)";
const TCP_RULE_NAME: &str = "QingLink Remote Control (TCP 3721)";
const DISCOVERY_PORT: &str = "3723";
const CONTROL_PORT: &str = "3721";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn ensure_udp_discovery_rule() -> Result<()> {
    ensure_rule(RULE_NAME, "UDP", DISCOVERY_PORT)
}

pub fn ensure_network_rules() -> Result<()> {
    ensure_udp_discovery_rule()?;
    ensure_rule(TCP_RULE_NAME, "TCP", CONTROL_PORT)
}

fn ensure_rule(name: &str, protocol: &str, port: &str) -> Result<()> {
    let _ = no_window(&mut std::process::Command::new("netsh"))
        .args([
            "advfirewall",
            "firewall",
            "delete",
            "rule",
            &format!("name={name}"),
        ])
        .output();
    let status = no_window(&mut std::process::Command::new("netsh"))
        .args([
            "advfirewall",
            "firewall",
            "add",
            "rule",
            &format!("name={name}"),
            "dir=in",
            "action=allow",
            &format!("protocol={protocol}"),
            &format!("localport={port}"),
            "profile=private",
        ])
        .status()
        .context("启动 Windows 防火墙配置失败")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("netsh 返回状态 {status}")
    }
}

pub fn remove_udp_discovery_rule() -> Result<()> {
    remove_rule(RULE_NAME)
}

pub fn remove_network_rules() -> Result<()> {
    remove_rule(RULE_NAME)?;
    remove_rule(TCP_RULE_NAME)
}

fn remove_rule(name: &str) -> Result<()> {
    let status = no_window(&mut std::process::Command::new("netsh"))
        .args([
            "advfirewall",
            "firewall",
            "delete",
            "rule",
            &format!("name={name}"),
        ])
        .status()
        .context("启动 Windows 防火墙配置失败")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("netsh 返回状态 {status}")
    }
}

#[cfg(target_os = "windows")]
fn no_window(cmd: &mut std::process::Command) -> &mut std::process::Command {
    cmd.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(target_os = "windows"))]
fn no_window(cmd: &mut std::process::Command) -> &mut std::process::Command {
    cmd
}
