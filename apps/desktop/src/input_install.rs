#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SERVICE_NAME: &str = "QingLinkService";
const SERVICE_EXE: &str = "QingLinkService.exe";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct EmbeddedAsset {
    relative_path: &'static str,
    bytes: &'static [u8],
    sha256: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/embedded_input_assets.rs"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputOperation {
    Install,
    Uninstall,
}

impl InputOperation {
    fn argument(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Uninstall => "uninstall",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InputInstallStatus {
    pub package_available: bool,
    pub service_installed: bool,
    pub service_running: bool,
    pub service_binary_matches: bool,
    pub available: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct InputInstallUiState {
    pub status: InputInstallStatus,
    pub busy: bool,
    pub result_message: Option<String>,
}

pub type SharedInputInstallState = Arc<Mutex<InputInstallUiState>>;

pub fn shared_state() -> SharedInputInstallState {
    Arc::new(Mutex::new(InputInstallUiState {
        status: inspect(),
        ..Default::default()
    }))
}

pub fn package_available() -> bool {
    !INPUT_ASSETS.is_empty()
}

pub fn inspect() -> InputInstallStatus {
    let service_installed = service_exists(SERVICE_NAME);
    let service_running = service_is_running(SERVICE_NAME);
    let image_path_ok = service_image_path_matches();
    let service_binary_matches = service_binary_matches();
    let available = service_running && image_path_ok && service_binary_matches;
    let detail = if available {
        "QingLink 安全桌面输入服务可用".to_string()
    } else if !package_available() {
        "发布包未内嵌安全输入服务，请使用 QINGLINK_REQUIRE_INPUT_ASSETS=1 重新构建".to_string()
    } else if !service_installed {
        "安全输入组件尚未安装".to_string()
    } else if service_installed && !image_path_ok {
        "输入服务路径与当前安装包不一致，可执行修复".to_string()
    } else if service_installed && !service_binary_matches {
        "输入服务版本过旧或文件已损坏，需要修复".to_string()
    } else if service_installed && !service_running {
        "输入服务已安装但未运行，可执行修复".to_string()
    } else {
        "安全输入组件未完整就绪，可执行修复".to_string()
    };
    InputInstallStatus {
        package_available: package_available(),
        service_installed,
        service_running,
        service_binary_matches,
        available,
        detail,
    }
}

pub fn refresh(shared: &SharedInputInstallState) {
    if let Ok(mut state) = shared.lock() {
        state.status = inspect();
        state.result_message = Some("安全输入状态已刷新".into());
    }
}

pub fn start_elevated(shared: &SharedInputInstallState, operation: InputOperation) {
    if let Ok(mut state) = shared.lock() {
        if state.busy {
            return;
        }
        if operation == InputOperation::Install && !package_available() {
            state.result_message = Some(
                "发布包未内嵌安全输入服务，请使用 QINGLINK_REQUIRE_INPUT_ASSETS=1 构建后安装"
                    .into(),
            );
            return;
        }
        state.busy = true;
        state.result_message = Some(match operation {
            InputOperation::Install => "正在请求管理员权限并安装…".into(),
            InputOperation::Uninstall => "正在请求管理员权限并卸载…".into(),
        });
    }

    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        let result = run_elevated(operation);
        let status = inspect();
        if let Ok(mut state) = shared.lock() {
            state.busy = false;
            state.status = status;
            state.result_message = Some(match result {
                Ok(message) => message,
                Err(error) => format!("安全输入操作失败：{error:#}"),
            });
        }
    });
}

pub fn handle_helper_command(args: &[String]) -> Option<Result<()>> {
    let index = args.iter().position(|arg| arg == "--input-component")?;
    let operation = match args.get(index + 1).map(String::as_str) {
        Some("install") => InputOperation::Install,
        Some("uninstall") => InputOperation::Uninstall,
        _ => return Some(Err(anyhow!("invalid input component operation"))),
    };
    let result_path = args
        .windows(2)
        .find_map(|pair| (pair[0] == "--result").then(|| PathBuf::from(&pair[1])));
    if args.contains(&"--skip-signature-check".to_string()) {
        unsafe {
            std::env::set_var("QINGLINK_SKIP_SIGNATURE_CHECK", "1");
        }
    }
    let result = match operation {
        InputOperation::Install => install(),
        InputOperation::Uninstall => uninstall(),
    };
    if let Some(path) = result_path {
        let message = match &result {
            Ok(()) => "ok".to_string(),
            Err(error) => format!("error:{error:#}"),
        };
        let _ = fs::write(path, message);
    }
    Some(result)
}

fn run_elevated(operation: InputOperation) -> Result<String> {
    let executable = std::env::current_exe().context("无法定位桌面程序")?;
    let result_path = std::env::temp_dir().join(format!(
        "qinglink-input-result-{}-{}.txt",
        std::process::id(),
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let arguments = format!(
        "--input-component {} --result \"{}\" --skip-signature-check",
        operation.argument(),
        result_path.display()
    );
    match qinglink_windows::elevation::run_elevated(&executable.to_string_lossy(), &arguments) {
        Ok(()) => {
            let helper_result = fs::read_to_string(&result_path).unwrap_or_default();
            let _ = fs::remove_file(&result_path);
            if let Some(error) = helper_result.strip_prefix("error:") {
                bail!(error.to_string());
            }
            Ok(match operation {
                InputOperation::Install => "安全输入组件已安装并启动".into(),
                InputOperation::Uninstall => "安全输入组件已卸载".into(),
            })
        }
        Err(qinglink_windows::elevation::ElevationError::Cancelled) => {
            let _ = fs::remove_file(&result_path);
            bail!("用户取消了 UAC 授权")
        }
        Err(error) => {
            let _ = fs::remove_file(&result_path);
            bail!("{error}")
        }
    }
}

fn install() -> Result<()> {
    ensure!(package_available(), "程序未内嵌安全输入服务");
    ensure!(is_elevated(), "安装助手未获得管理员权限");
    verify_embedded_assets()?;
    if std::env::var_os("QINGLINK_SKIP_SIGNATURE_CHECK").is_none() {
        verify_embedded_signatures()?;
    }

    let install_dir = install_dir()?;
    let staging = install_dir.with_extension(format!("staging-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).context("无法清理安装暂存目录")?;
    }
    extract_assets(&staging)?;

    if service_exists(SERVICE_NAME) {
        if is_elevated() {
            let _ = run_sc_allow_failure(&["stop", SERVICE_NAME]);
            wait_for_service_stopped(SERVICE_NAME);
            for _ in 0..10 {
                if !service_is_running(SERVICE_NAME) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            let _ = no_window(&mut std::process::Command::new("taskkill"))
                .args(["/F", "/IM", SERVICE_EXE])
                .status();
            std::thread::sleep(std::time::Duration::from_secs(2));
        } else {
            let _ = qinglink_windows::elevation::run_elevated_sc(&["stop", SERVICE_NAME]);
            wait_for_service_stopped(SERVICE_NAME);
            for _ in 0..10 {
                if !service_is_running(SERVICE_NAME) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }

    fs::create_dir_all(&install_dir).context("无法创建安全输入安装目录")?;
    for asset in INPUT_ASSETS {
        let source = staging.join(asset.relative_path);
        let target = install_dir.join(asset.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source, &target).with_context(|| format!("无法安装 {}", asset.relative_path))?;
    }
    let _ = fs::remove_dir_all(&staging);

    // 确保已安装的服务和驱动载荷仅由管理员和 SYSTEM 写入。
    run_checked(
        "icacls.exe",
        &[
            path_arg(&install_dir)?,
            "/inheritance:r",
            "/grant:r",
            "SYSTEM:(OI)(CI)(F)",
            "*S-1-5-32-544:(OI)(CI)(F)",
            "*S-1-5-4:(OI)(CI)(RX)",
        ],
    )?;

    let service_path = install_dir.join(SERVICE_EXE);
    let quoted_service_path = format!("\"{}\"", service_path.display());
    if service_exists(SERVICE_NAME) {
        run_sc(&[
            "config",
            SERVICE_NAME,
            "binPath=",
            &quoted_service_path,
            "start=",
            "auto",
            "obj=",
            "LocalSystem",
        ])?;
    } else {
        run_sc(&[
            "create",
            SERVICE_NAME,
            "binPath=",
            &quoted_service_path,
            "start=",
            "auto",
            "obj=",
            "LocalSystem",
            "DisplayName=",
            "QingLink System Service",
        ])?;
    }
    run_sc(&["description", SERVICE_NAME, "QingLink System Service"])?;
    // Keep remote control available after transient boot/network failures or
    // an unexpected process exit.  SCM performs the restart independently of
    // the interactive desktop, so this also covers the no-keyboard case.
    run_sc(&[
        "failure",
        SERVICE_NAME,
        "reset=",
        "86400",
        "actions=",
        "restart/5000/restart/5000/restart/10000",
    ])?;
    // Permit the logged-in user to start/stop this service for the desktop
    // lifecycle manager, while keeping configuration and deletion elevated.
    run_checked(
        "sc.exe",
        &[
            "sdset",
            SERVICE_NAME,
            "D:(A;;CCLCSWRPWPLOCRRC;;;IU)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWRPWPDTLOCRRC;;;SY)",
        ],
    )
    .context("无法配置安全输入服务 ACL")?;
    qinglink_windows::firewall::ensure_network_rules()
        .context("无法创建局域网发现和远程控制防火墙规则")?;
    qinglink_windows::autostart::enable().context("无法启用 QingLink 桌面端登录自启动")?;
    start_service_and_wait()?;

    let status = inspect();
    ensure!(status.service_running, "输入服务安装后未能启动");
    Ok(())
}

fn uninstall() -> Result<()> {
    ensure!(is_elevated(), "卸载助手未获得管理员权限");
    let _ = run_sc_allow_failure(&["stop", SERVICE_NAME]);
    let _ = run_sc_allow_failure(&["delete", SERVICE_NAME]);
    qinglink_windows::autostart::disable().context("无法移除 QingLink 桌面端登录自启动")?;
    qinglink_windows::firewall::remove_network_rules().context("无法移除 QingLink 防火墙规则")?;
    let directory = install_dir()?;
    if directory.exists() {
        fs::remove_dir_all(directory).context("无法删除安全输入安装目录")?;
    }
    Ok(())
}

fn extract_assets(directory: &Path) -> Result<()> {
    fs::create_dir_all(directory).context("无法创建资源暂存目录")?;
    for asset in INPUT_ASSETS {
        let target = directory.join(asset.relative_path);
        ensure!(target.starts_with(directory), "资源路径越界");
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, asset.bytes)
            .with_context(|| format!("无法释放 {}", asset.relative_path))?;
        verify_file_hash(&target, asset.sha256)?;
    }
    Ok(())
}

fn verify_embedded_assets() -> Result<()> {
    for asset in INPUT_ASSETS {
        let actual = format!("{:x}", Sha256::digest(asset.bytes));
        ensure!(
            actual == asset.sha256,
            "内嵌资源校验失败：{}",
            asset.relative_path
        );
    }
    Ok(())
}

fn verify_file_hash(path: &Path, expected: &str) -> Result<()> {
    let bytes = fs::read(path)?;
    ensure!(
        format!("{:x}", Sha256::digest(bytes)) == expected,
        "文件校验失败：{}",
        path.display()
    );
    Ok(())
}

fn verify_embedded_signatures() -> Result<()> {
    let staging =
        std::env::temp_dir().join(format!("qinglink-signature-check-{}", std::process::id()));
    extract_assets(&staging)?;
    for relative in [SERVICE_EXE] {
        let path = powershell_literal(&staging.join(relative).to_string_lossy());
        let script = format!(
            "if ((Get-AuthenticodeSignature -LiteralPath '{path}').Status -ne 'Valid') {{ exit 1 }}"
        );
        let status = no_window(&mut Command::new("powershell.exe"))
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()?;
        ensure!(status.success(), "签名无效：{relative}");
    }
    let _ = fs::remove_dir_all(staging);
    Ok(())
}

fn install_dir() -> Result<PathBuf> {
    let program_files =
        std::env::var_os("ProgramFiles").ok_or_else(|| anyhow!("无法读取 ProgramFiles 目录"))?;
    Ok(PathBuf::from(program_files).join("QingLink/Input"))
}

#[cfg(target_os = "windows")]
fn no_window(cmd: &mut Command) -> &mut Command {
    cmd.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(target_os = "windows"))]
fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}

fn service_exists(name: &str) -> bool {
    no_window(&mut Command::new("sc.exe"))
        .args(["query", name])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn service_is_running(name: &str) -> bool {
    no_window(&mut Command::new("sc.exe"))
        .args(["query", name])
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains("RUNNING")
        })
}

/// SCM's RUNNING state only means that the service process registered with
/// Windows. Probe the actual HTTP endpoint before treating the service as
/// ready, otherwise a failed bind is exposed as a false-positive startup.
pub(crate) fn service_network_ready() -> bool {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let address: SocketAddr = match "127.0.0.1:3721".parse() {
        Ok(value) => value,
        Err(_) => return false,
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
    if stream
        .write_all(b"GET /api/v2/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = [0u8; 256];
    let Ok(size) = stream.read(&mut response) else {
        return false;
    };
    let response = String::from_utf8_lossy(&response[..size]);
    response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200")
}

fn service_image_path_matches() -> bool {
    let expected = install_dir().ok().map(|p| p.join(SERVICE_EXE));
    let Some(expected) = expected else {
        return false;
    };
    let Ok(output) = no_window(&mut Command::new("sc.exe"))
        .args(["qc", SERVICE_NAME])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    text.contains(&expected.to_string_lossy().to_ascii_lowercase())
}

fn service_binary_matches() -> bool {
    let expected = INPUT_ASSETS
        .iter()
        .find(|asset| asset.relative_path.eq_ignore_ascii_case(SERVICE_EXE))
        .map(|asset| asset.sha256);
    let Some(expected) = expected else {
        return false;
    };
    let Ok(path) = install_dir().map(|dir| dir.join(SERVICE_EXE)) else {
        return false;
    };
    fs::read(&path)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected)
        .unwrap_or(false)
}

fn wait_for_service_stopped(name: &str) {
    // Wait for the terminal STOPPED state, not merely "not RUNNING":
    // `sc start` fails while the service is still in STOP_PENDING.
    for _ in 0..40 {
        if !service_exists(name) {
            return;
        }
        let stopped = no_window(&mut Command::new("sc.exe"))
            .args(["query", name])
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains("STOPPED")
            });
        if stopped {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

fn wait_for_service_running(name: &str) -> bool {
    for _ in 0..40 {
        if service_is_running(name) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    false
}

/// Start the service and wait until SCM reports it RUNNING.  `sc start`
/// returns 1056 when the service is already running (for example after SCM's
/// failure-restart fired), so the exit code is treated as advisory and the
/// real state is polled instead.
fn start_service_and_wait() -> Result<()> {
    if service_is_running(SERVICE_NAME) {
        return Ok(());
    }
    if let Err(error) = run_sc_allow_failure(&["start", SERVICE_NAME]) {
        tracing::warn!(%error, "调用 sc start 失败，继续轮询服务状态");
    }
    ensure!(
        wait_for_service_running(SERVICE_NAME),
        "输入服务未能启动，请检查 Windows 服务状态或事件日志"
    );
    Ok(())
}

fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        unsafe { windows::Win32::UI::Shell::IsUserAnAdmin().as_bool() }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn run_sc(args: &[&str]) -> Result<()> {
    run_checked("sc.exe", args)
}

fn run_sc_allow_failure(args: &[&str]) -> Result<ExitStatus> {
    no_window(&mut Command::new("sc.exe"))
        .args(args)
        .status()
        .context("无法运行 sc.exe")
}

pub fn start_service_elevated() -> Result<String> {
    if !service_exists(SERVICE_NAME) {
        anyhow::bail!("服务未安装");
    }
    if service_is_running(SERVICE_NAME) {
        return Ok("服务已在运行".into());
    }
    start_elevated_and_wait()?;
    Ok("服务已启动".into())
}

/// Starts the installed service without UAC. Installation grants the
/// interactive user SERVICE_START, which is required for silent login startup.
pub fn start_service_direct() -> Result<String> {
    if !service_exists(SERVICE_NAME) {
        anyhow::bail!("服务未安装");
    }
    start_service_and_wait()?;
    Ok("服务已启动".into())
}

pub fn stop_service_elevated() -> Result<String> {
    if !service_exists(SERVICE_NAME) {
        anyhow::bail!("服务未安装");
    }
    qinglink_windows::elevation::run_elevated_sc(&["stop", SERVICE_NAME])
        .map_err(|e| anyhow::anyhow!(e))
        .map(|()| "服务已停止".into())
}

pub fn restart_service_elevated() -> Result<String> {
    if service_is_running(SERVICE_NAME) {
        qinglink_windows::elevation::run_elevated_sc(&["stop", SERVICE_NAME])
            .map_err(|e| anyhow::anyhow!(e))?;
        wait_for_service_stopped(SERVICE_NAME);
    }
    start_elevated_and_wait()?;
    Ok("服务已重启".into())
}

/// Elevated `sc start`, then wait for RUNNING.  A 1056 "already running"
/// result is tolerated because SCM's failure-restart may have started the
/// service in the meantime; only the final polled state decides success.
fn start_elevated_and_wait() -> Result<()> {
    if service_is_running(SERVICE_NAME) {
        return Ok(());
    }
    let start_result = qinglink_windows::elevation::run_elevated_sc(&["start", SERVICE_NAME]);
    if wait_for_service_running(SERVICE_NAME) {
        return Ok(());
    }
    match start_result {
        Err(error) => Err(anyhow::anyhow!("{error}")),
        Ok(()) => Err(anyhow::anyhow!("服务未能启动，请检查 Windows 事件日志")),
    }
}

fn run_checked(program: &str, args: &[&str]) -> Result<()> {
    let output = no_window(&mut Command::new(program))
        .args(args)
        .output()
        .with_context(|| format!("无法运行 {program}"))?;
    ensure!(
        output.status.success(),
        "{} 失败：{}{}",
        program,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn path_arg(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("安装路径包含无法处理的字符"))
}

fn powershell_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_paths_are_escaped() {
        assert_eq!(
            powershell_literal("C:\\O'Brien\\app.exe"),
            "C:\\O''Brien\\app.exe"
        );
    }

    #[test]
    fn development_build_reports_missing_package_consistently() {
        if INPUT_ASSETS.is_empty() {
            assert!(!package_available());
            assert!(!inspect().package_available);
        }
    }
}
