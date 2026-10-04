//! `--self-check`：回答那些从外部难以观测的问题。
//!
//! "托盘图标不出现"有若干种截然不同的原因，而 shell 把其中大多数都藏了起来：
//! 图标可能被收进溢出浮出菜单，光栅化出的位图可能是空白或单色，注册也可能直接被
//! 拒绝。猜测代价太高，因此这条命令把图标渲染成 PNG、报告其像素统计、尝试一次
//! 真实注册，然后把全部结果以 JSON 打印。
//!
//! 它不碰服务，也不启动 Web 服务器，所以在应用已经运行时也可以安全执行。

use std::path::Path;

use anyhow::{Context, Result};
use qinglink_storage::AppPaths;
use serde_json::{Value, json};

use crate::tray::{TrayController, icon};
use crate::web::assets;

/// Run the diagnostic and print a JSON report.
pub fn run(paths: &AppPaths) -> Result<()> {
    let log_file = paths.log_file("qinglink.log");
    let log_directory = log_file.parent().map(Path::to_path_buf).unwrap_or_default();

    let mut report = serde_json::Map::new();
    report.insert("version".into(), json!(env!("CARGO_PKG_VERSION")));
    report.insert("pid".into(), json!(std::process::id()));
    report.insert(
        "data_dir".into(),
        json!(paths.data_dir().display().to_string()),
    );
    report.insert("log_dir".into(), json!(log_directory.display().to_string()));
    report.insert(
        "log_dir_writable".into(),
        json!(is_writable(&log_directory)),
    );
    report.insert(
        "web_interface".into(),
        json!({
            "assets_embedded": assets::ASSET_COUNT,
            "index_bytes": assets::INDEX_HTML.len(),
            "placeholder": assets::is_placeholder(),
        }),
    );
    report.insert("tray_icon".into(), icon_report(&log_directory)?);
    report.insert("tray_registration".into(), registration_report());
    report.insert("service".into(), service_report());

    let rendered = serde_json::to_string_pretty(&Value::Object(report))
        .context("cannot serialize the self-check report")?;
    println!("{rendered}");
    Ok(())
}

/// Probe the service pipe and print the result as JSON.
///
/// This answers the question `--self-check` deliberately does not: whether a
/// management call can get through at all. The distinction it preserves is the
/// expensive one to guess at — a service that is installed and running while the
/// pipe is refused is an environment restriction, not a broken installation, and
/// the two need opposite responses.
pub fn run_service_probe() -> Result<()> {
    let install = crate::input_install::inspect();
    let probe = crate::service_lifecycle::probe_management_pipe();

    let report = match &probe {
        Ok(response) => json!({
            "ok": response.ok,
            "code": response.code,
            "message": response.message,
            "outcome": format!("管理通道可用：{}", response.message),
        }),
        Err(error) => {
            let chain = format!("{error:#}");
            let blocked = crate::service_lifecycle::is_environment_restriction(&chain);
            let hint = if blocked && install.service_installed && install.service_running {
                "服务已安装且正在运行，但当前执行环境拒绝访问命名管道；\
                 请在普通用户会话或不受限的终端中重试"
            } else if blocked {
                "无法访问服务管道；服务可能未安装或未运行"
            } else {
                "管道已连接，但请求失败；服务与桌面端的协议版本可能不匹配"
            };
            json!({
                "ok": false,
                "code": "PIPE_UNREACHABLE",
                "error": chain,
                "outcome": hint,
            })
        }
    };

    let mut document = serde_json::Map::new();
    document.insert("version".into(), json!(env!("CARGO_PKG_VERSION")));
    document.insert("pid".into(), json!(std::process::id()));
    document.insert("pipe".into(), json!(crate::service_lifecycle::PIPE_NAME));
    document.insert("probe".into(), report);
    document.insert(
        "install_state".into(),
        json!({
            "package_available": install.package_available,
            "service_installed": install.service_installed,
            "service_running": install.service_running,
            "service_binary_matches": install.service_binary_matches,
            "network_ready": install.network_ready,
            "available": install.available,
            "detail": install.detail,
        }),
    );

    let rendered = serde_json::to_string_pretty(&Value::Object(document))
        .context("cannot serialize the service probe")?;
    println!("{rendered}");
    Ok(())
}

/// Service reachability, without touching the pipe.
///
/// `--self-check` runs while the application may be running, so it reports what
/// the service control manager sees and leaves the pipe probe to
/// `--check-service`.
fn service_report() -> Value {
    let install = crate::input_install::inspect();
    json!({
        "package_available": install.package_available,
        "service_installed": install.service_installed,
        "service_running": install.service_running,
        "service_binary_matches": install.service_binary_matches,
        "network_ready": install.network_ready,
        "available": install.available,
        "detail": install.detail,
    })
}

/// Statistically describe the bitmap the shell would be given.
///
/// `monochrome` is the decisive field: an all-one-colour icon is what a
/// template conversion produces, and it is invisible against a matching
/// taskbar.
fn icon_report(log_directory: &Path) -> Result<Value> {
    let size = icon::target_size();
    let rgba = icon::rasterize(size)?;

    let mut opaque = 0_usize;
    let mut transparent = 0_usize;
    let mut monochrome = true;
    let mut has_brand_blue = false;
    let mut has_white = false;
    let mut colours = std::collections::BTreeSet::new();

    for pixel in rgba.chunks_exact(4) {
        let [red, green, blue, alpha] = [pixel[0], pixel[1], pixel[2], pixel[3]];
        if alpha == 0 {
            transparent += 1;
            continue;
        }
        opaque += 1;
        if red != green || green != blue {
            monochrome = false;
        }
        // Widen before comparing: `red + 30` overflows u8 at 255.
        let (red, green, blue) = (i32::from(red), i32::from(green), i32::from(blue));
        // Brand blue (#137BC2) and the white glyph of the mark.
        if blue > 120 && blue > red + 30 {
            has_brand_blue = true;
        }
        if red > 200 && green > 200 && blue > 200 {
            has_white = true;
        }
        colours.insert((red / 16, green / 16, blue / 16));
    }

    let png = log_directory.join(format!("tray-icon-{size}px.png"));
    let png_written = write_png(&png, size, &rgba).is_ok();

    // Two independent things can make the icon invisible: bad pixels, and a
    // bitmap that never became a valid icon handle. Only the second one is
    // silent in the shell, so it is reported explicitly.
    let handle = crate::tray::icon::bitmap_from_rgba(&rgba, size);
    let (icon_handle_ok, icon_handle_error) = match &handle {
        Ok(_) => (true, None),
        Err(error) => (false, Some(format!("{error:#}"))),
    };
    if let Ok(icon) = handle {
        // SAFETY: the handle came from `bitmap_from_rgba` and is destroyed once.
        unsafe { crate::tray::icon::destroy(icon) };
    }

    Ok(json!({
        "size_px": size,
        "opaque_pixels": opaque,
        "transparent_pixels": transparent,
        "distinct_colours": colours.len(),
        "monochrome": monochrome,
        "has_brand_blue": has_brand_blue,
        "has_white_glyph": has_white,
        "icon_handle_ok": icon_handle_ok,
        "icon_handle_error": icon_handle_error,
        "png": png_written.then(|| png.display().to_string()),
    }))
}

/// Attempt a real registration, then let the icon go again.
///
/// The window is brief: the controller is dropped before this function returns.
fn registration_report() -> Value {
    match TrayController::create(false) {
        Ok((tray, _events)) => json!(tray.registration()),
        Err(error) => json!({
            "visible": false,
            "detail": format!("托盘图标创建失败：{error:#}"),
        }),
    }
}

/// Write the rendered icon so it can be inspected by eye.
fn write_png(path: &Path, size: u32, rgba: &[u8]) -> Result<()> {
    let image = image::RgbaImage::from_raw(size, size, rgba.to_vec())
        .context("the rendered icon buffer has the wrong size")?;
    image
        .save(path)
        .context("cannot write the tray icon preview")?;
    Ok(())
}

/// Whether a directory accepts new files.
fn is_writable(directory: &Path) -> bool {
    let probe = directory.join(".qinglink-write-probe");
    match std::fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}
