use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};

use super::{InputOperation, scm};

pub(crate) fn run_elevated(operation: InputOperation) -> Result<String> {
    let result_file = result_path();
    let argument = format!(
        "--input-component {} --result \"{}\" --skip-signature-check",
        operation.argument(),
        result_file.display()
    );
    let result = qinglink_windows::elevation::run_elevated(
        &std::env::current_exe()
            .context("无法定位桌面程序")?
            .to_string_lossy(),
        &argument,
    );
    read_helper_result(
        result,
        result_file,
        match operation {
            InputOperation::Install => "安全输入组件已安装并启动",
            InputOperation::Uninstall => "安全输入组件已卸载",
        },
    )
}

pub(crate) fn run_elevated_service_control(command: &str, service_name: &str) -> Result<()> {
    let result_file = result_path();
    let argument = format!(
        "--input-service-control {command} {service_name} --result \"{}\"",
        result_file.display()
    );
    let result = qinglink_windows::elevation::run_elevated(
        &std::env::current_exe()
            .context("无法定位桌面程序")?
            .to_string_lossy(),
        &argument,
    );
    read_helper_result(result, result_file, "服务控制成功").map(|_| ())
}

pub(crate) fn start_elevated_and_wait(service_name: &str) -> Result<()> {
    run_elevated_service_control("start", service_name)?;
    scm::wait_for_running(service_name);
    if scm::service_is_running(service_name) {
        Ok(())
    } else {
        bail!("服务未能启动，请检查 Windows 事件日志")
    }
}

fn read_helper_result(
    result: qinglink_windows::elevation::Result<()>,
    result_file: PathBuf,
    success_message: &str,
) -> Result<String> {
    match result {
        Ok(()) => {
            let helper_result = fs::read_to_string(&result_file).unwrap_or_default();
            let _ = fs::remove_file(&result_file);
            if let Some(error) = helper_result.strip_prefix("error:") {
                bail!(error.to_string());
            }
            Ok(success_message.to_string())
        }
        Err(qinglink_windows::elevation::ElevationError::Cancelled) => {
            let _ = fs::remove_file(result_file);
            bail!("用户取消了 UAC 授权")
        }
        Err(error) => {
            let _ = fs::remove_file(result_file);
            bail!("{error}")
        }
    }
}

fn result_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "qinglink-input-result-{}-{}.txt",
        std::process::id(),
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ))
}
