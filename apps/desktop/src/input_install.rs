use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod acl;
mod assets;
mod elevation;
mod health;
mod scm;
mod signature;

const SERVICE_NAME: &str = "QingLinkService";
const SERVICE_EXE: &str = "QingLinkService.exe";

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
    /// The service's own health endpoint answers on the loopback port.
    ///
    /// Kept separate from [`Self::available`] so a failure can be attributed:
    /// "installed and running but no health" (a binding or firewall problem, or
    /// an environment that forbids the connection) reads very differently from
    /// "not running at all".
    pub network_ready: bool,
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
    let state = Arc::new(Mutex::new(InputInstallUiState::default()));
    let worker_state = Arc::clone(&state);
    std::thread::spawn(move || refresh(&worker_state));
    state
}

pub fn package_available() -> bool {
    !assets::packaged_assets().is_empty()
}

pub fn inspect() -> InputInstallStatus {
    let service_installed = scm::service_exists(SERVICE_NAME);
    let service_running = scm::service_is_running(SERVICE_NAME);
    let image_path_ok = scm::service_image_path_matches(SERVICE_NAME, &service_path());
    let service_binary_matches = service_binary_matches();
    let network_ready = service_running && health::service_network_ready();
    let available = service_running && image_path_ok && service_binary_matches && network_ready;
    let detail = if available {
        "QingLink 安全桌面输入服务可用".to_string()
    } else if !package_available() {
        "发布包未内嵌安全输入服务，请使用 QINGLINK_REQUIRE_INPUT_ASSETS=1 重新构建".to_string()
    } else if !service_installed {
        "安全输入组件尚未安装".to_string()
    } else if !image_path_ok {
        "输入服务路径与当前安装包不一致，可执行修复".to_string()
    } else if !service_binary_matches {
        "输入服务版本过旧或文件已损坏，需要修复".to_string()
    } else if !service_running {
        "输入服务已安装但未运行，可执行修复".to_string()
    } else if !network_ready {
        "输入服务已运行但健康协议未就绪，可执行修复".to_string()
    } else {
        "安全输入组件未完整就绪，可执行修复".to_string()
    };
    InputInstallStatus {
        package_available: package_available(),
        service_installed,
        service_running,
        service_binary_matches,
        network_ready,
        available,
        detail,
    }
}

pub fn refresh(shared: &SharedInputInstallState) {
    let status = inspect();
    if let Ok(mut state) = shared.lock() {
        state.status = status;
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
        let result = elevation::run_elevated(operation);
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
    if let Some(index) = args.iter().position(|arg| arg == "--input-service-control") {
        let command = args.get(index + 1).map(String::as_str);
        let service_name = args.get(index + 2).map(String::as_str);
        let result_path = args
            .windows(2)
            .find_map(|pair| (pair[0] == "--result").then(|| PathBuf::from(&pair[1])));
        let result = match (command, service_name) {
            (Some("start"), Some(SERVICE_NAME)) => scm::start_and_wait(SERVICE_NAME),
            (Some("stop"), Some(SERVICE_NAME)) => scm::stop_and_wait(SERVICE_NAME).map(|_| ()),
            _ => Err(anyhow!("invalid input service control operation")),
        };
        if let Some(path) = result_path {
            let message = match &result {
                Ok(()) => "ok".to_string(),
                Err(error) => format!("error:{error:#}"),
            };
            let _ = fs::write(path, message);
        }
        return Some(result);
    }
    let index = args.iter().position(|arg| arg == "--input-component")?;
    let operation = match args.get(index + 1).map(String::as_str) {
        Some("install") => InputOperation::Install,
        Some("uninstall") => InputOperation::Uninstall,
        _ => return Some(Err(anyhow!("invalid input component operation"))),
    };
    let result_path = args
        .windows(2)
        .find_map(|pair| (pair[0] == "--result").then(|| PathBuf::from(&pair[1])));
    if args.iter().any(|arg| arg == "--skip-signature-check") {
        // This flag is accepted only by the elevated helper process.
        unsafe { std::env::set_var("QINGLINK_SKIP_SIGNATURE_CHECK", "1") };
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

fn install() -> Result<()> {
    ensure!(package_available(), "程序未内嵌安全输入服务");
    ensure!(is_elevated(), "安装助手未获得管理员权限");
    signature::verify_embedded_assets()?;
    let development_identity =
        cfg!(debug_assertions) && std::env::var_os("QINGLINK_SKIP_SIGNATURE_CHECK").is_some();
    if !development_identity {
        signature::verify_embedded_signatures()?;
        signature::verify_signature(&std::env::current_exe()?)?;
    }

    let install_directory = install_dir()?;
    let staging = install_directory.with_extension(format!("staging-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).context("无法清理安装暂存目录")?;
    }
    assets::extract(&staging)?;

    if scm::service_exists(SERVICE_NAME) {
        scm::stop_and_wait(SERVICE_NAME)?;
    }
    fs::create_dir_all(&install_directory).context("无法创建安全输入安装目录")?;
    for asset in assets::packaged_assets() {
        let source = staging.join(asset.relative_path);
        let target = install_directory.join(asset.relative_path);
        ensure!(target.starts_with(&install_directory), "资源路径越界");
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source, &target).with_context(|| format!("无法安装 {}", asset.relative_path))?;
    }
    let _ = fs::remove_dir_all(&staging);
    acl::secure_install_directory(&install_directory)?;

    write_trusted_desktop(&install_directory)?;
    scm::install_or_configure(SERVICE_NAME, &service_path())?;
    scm::configure_service_permissions(SERVICE_NAME)?;
    qinglink_windows::firewall::ensure_network_rules()
        .context("无法创建局域网发现和远程控制防火墙规则")?;
    qinglink_windows::autostart::enable().context("无法启用 QingLink 桌面端登录自启动")?;
    scm::start_and_wait(SERVICE_NAME)?;
    ensure!(
        scm::service_is_running(SERVICE_NAME),
        "输入服务安装后未能启动"
    );
    Ok(())
}

fn uninstall() -> Result<()> {
    ensure!(is_elevated(), "卸载助手未获得管理员权限");
    scm::delete_service(SERVICE_NAME)?;
    qinglink_windows::autostart::disable().context("无法移除 QingLink 桌面端登录自启动")?;
    qinglink_windows::firewall::remove_network_rules().context("无法移除 QingLink 防火墙规则")?;
    let directory = install_dir()?;
    if directory.exists() {
        fs::remove_dir_all(directory).context("无法删除安全输入安装目录")?;
    }
    Ok(())
}

fn write_trusted_desktop(install_directory: &Path) -> Result<()> {
    let desktop_path = std::env::current_exe()?.canonicalize()?;
    let desktop_hash = format!("{:x}", Sha256::digest(fs::read(&desktop_path)?));
    let record = serde_json::json!({
        "path": desktop_path.to_string_lossy(),
        "sha256": desktop_hash,
    });
    fs::write(
        install_directory.join("trusted-desktop.json"),
        serde_json::to_vec(&record)?,
    )
    .context("无法登记可信桌面端，请修复服务安装")
}

fn service_binary_matches() -> bool {
    let Some(expected) = assets::packaged_assets()
        .iter()
        .find(|asset| asset.relative_path.eq_ignore_ascii_case(SERVICE_EXE))
        .map(|asset| asset.sha256)
    else {
        return false;
    };
    fs::read(service_path())
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected)
        .unwrap_or(false)
}

fn install_dir() -> Result<PathBuf> {
    let program_files =
        std::env::var_os("ProgramFiles").ok_or_else(|| anyhow!("无法读取 ProgramFiles 目录"))?;
    Ok(PathBuf::from(program_files).join("QingLink/Input"))
}

fn service_path() -> PathBuf {
    install_dir()
        .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\QingLink\Input"))
        .join(SERVICE_EXE)
}

pub fn start_service_elevated() -> Result<String> {
    ensure!(scm::service_exists(SERVICE_NAME), "服务未安装");
    elevation::start_elevated_and_wait(SERVICE_NAME)?;
    Ok("服务已启动".into())
}

pub fn start_service_direct() -> Result<String> {
    ensure!(scm::service_exists(SERVICE_NAME), "服务未安装");
    scm::start_and_wait(SERVICE_NAME)?;
    Ok("服务已启动".into())
}

pub fn stop_service_elevated() -> Result<String> {
    ensure!(scm::service_exists(SERVICE_NAME), "服务未安装");
    elevation::run_elevated_service_control("stop", SERVICE_NAME)?;
    Ok("服务已停止".into())
}

pub fn restart_service_elevated() -> Result<String> {
    ensure!(scm::service_exists(SERVICE_NAME), "服务未安装");
    if scm::service_is_running(SERVICE_NAME) {
        elevation::run_elevated_service_control("stop", SERVICE_NAME)?;
        scm::wait_for_stopped(SERVICE_NAME);
    }
    elevation::start_elevated_and_wait(SERVICE_NAME)?;
    Ok("服务已重启".into())
}

fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: IsUserAnAdmin does not dereference caller-provided pointers.
        unsafe { windows::Win32::UI::Shell::IsUserAnAdmin().as_bool() }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_result_argument_without_value_is_ignored() {
        assert!(
            handle_helper_command(&[
                "--input-component".into(),
                "invalid".into(),
                "--result".into()
            ])
            .is_some_and(|result| result.is_err())
        );
    }

    #[test]
    fn development_build_reports_missing_package_consistently() {
        if assets::packaged_assets().is_empty() {
            assert!(!package_available());
            assert!(!inspect().package_available);
        }
    }
}
