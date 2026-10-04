//! 已安装应用搜索：扫描开始菜单并解析 .lnk 快捷方式的目标路径。

use std::fs;
use std::os::windows::ffi::OsStrExt;

use qinglink_protocol::AppInfo;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink};
use windows::core::{Interface, PCWSTR};

use super::com_apartment::ComApartment;

/// 按名称关键字搜索已安装的应用，返回可用于启动的应用信息列表。
pub fn search_installed_apps(query: &str) -> Vec<AppInfo> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let _com = ComApartment::init().ok();

    let dirs = [
        format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            std::env::var("ProgramData").unwrap_or_default()
        ),
        format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            std::env::var("APPDATA").unwrap_or_default()
        ),
    ];

    for dir in &dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_lnk = name.to_lowercase().ends_with(".lnk");
                        let display_name = if is_lnk {
                            name[..name.len() - 4].to_string()
                        } else {
                            name.clone()
                        };

                        if query.is_empty() || display_name.to_lowercase().contains(&query_lower) {
                            let resolved_path = if is_lnk {
                                resolve_lnk_target(&path).0
                            } else {
                                path.to_string_lossy().to_string()
                            };

                            results.push(AppInfo {
                                name: display_name,
                                path: resolved_path,
                            });
                        }
                    } else if metadata.is_dir()
                        && let Ok(sub) = fs::read_dir(&path)
                    {
                        for sub_entry in sub.flatten() {
                            if sub_entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                                let name = sub_entry.file_name().to_string_lossy().to_string();
                                let is_lnk = name.to_lowercase().ends_with(".lnk");
                                let display_name = if is_lnk {
                                    name[..name.len() - 4].to_string()
                                } else {
                                    name.clone()
                                };

                                if query.is_empty()
                                    || display_name.to_lowercase().contains(&query_lower)
                                {
                                    let resolved_path = if is_lnk {
                                        resolve_lnk_target(&sub_entry.path()).0
                                    } else {
                                        sub_entry.path().to_string_lossy().to_string()
                                    };

                                    results.push(AppInfo {
                                        name: display_name,
                                        path: resolved_path,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 过滤掉快捷方式指向的已失效目标路径，避免启动器展示无法运行的条目。
    results.retain(|app| std::path::Path::new(&app.path).exists());
    results.truncate(100);
    results
}

/// 解析 .lnk 快捷方式获取目标路径和原始参数字符串。
pub(super) fn resolve_lnk_target(path: &std::path::Path) -> (String, Option<String>) {
    let fallback = || (path.to_string_lossy().to_string(), None);

    unsafe {
        let link: IShellLinkW = match CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) {
            Ok(link) => link,
            Err(_) => return fallback(),
        };
        let persist: IPersistFile = match link.cast() {
            Ok(persist) => persist,
            Err(_) => return fallback(),
        };
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).is_err() {
            return fallback();
        }

        let mut target_buf = vec![0u16; 32768];
        let _ = link.GetPath(&mut target_buf, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32);
        let target =
            read_wide_string(&target_buf).unwrap_or_else(|| path.to_string_lossy().to_string());

        let mut args_buf = vec![0u16; 32768];
        let arguments = if link.GetArguments(&mut args_buf).is_ok() {
            read_wide_string(&args_buf)
        } else {
            None
        };

        (target, arguments)
    }
}

/// Read a null-terminated UTF-16 buffer, returning `None` when empty.
pub(super) fn read_wide_string(buffer: &[u16]) -> Option<String> {
    let len = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    (len > 0).then(|| String::from_utf16_lossy(&buffer[..len]))
}
