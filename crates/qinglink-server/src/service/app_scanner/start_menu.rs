//! 开始菜单扫描：快捷方式发现与噪音条目的过滤。

use std::collections::HashMap;
use std::path::PathBuf;

use crate::service::app_scanner::{AppEntry, AppScanner};

impl AppScanner {
    pub(super) fn scan_start_menu(&self, renames: &HashMap<String, String>) -> Vec<AppEntry> {
        let mut apps = Vec::new();
        // 环境变量缺失时必须留痕：以前用 `unwrap_or_default()` 得到空路径，
        // 拼出来的相对路径自然不存在，于是该来源稳定产出 0 条——
        // 与"本机真的没装程序"完全无法区分。
        let mut paths = Vec::new();
        for variable in ["APPDATA", "PROGRAMDATA"] {
            match std::env::var_os(variable) {
                Some(value) if !value.is_empty() => paths.push(
                    PathBuf::from(value)
                        .join("Microsoft")
                        .join("Windows")
                        .join("Start Menu")
                        .join("Programs"),
                ),
                _ => tracing::warn!(
                    variable,
                    "开始菜单扫描缺少环境变量，该来源本次产出 0 条（不代表本机没有安装程序）"
                ),
            }
        }
        for base in &paths {
            if base.exists() {
                self.scan_dir(base, renames, &mut apps);
            } else {
                tracing::debug!(
                    path = %base.display(),
                    "开始菜单目录不存在，跳过该来源"
                );
            }
        }
        apps
    }

    fn scan_dir(
        &self,
        dir: &std::path::Path,
        renames: &HashMap<String, String>,
        apps: &mut Vec<AppEntry>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.scan_dir(&path, renames, apps);
                } else if path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
                    && let Some(name) = path.file_stem().and_then(|s| s.to_str())
                {
                    if Self::is_start_menu_noise(name) {
                        continue;
                    }
                    // A shortcut whose target is gone only ever produces a
                    // failed launch; the Start Menu keeps many of them around.
                    let path_str = path.to_str().unwrap_or("").to_string();
                    if !crate::platform::shortcut_target_exists(&path_str) {
                        continue;
                    }
                    let original = name.to_string();
                    let display = renames
                        .get(&original)
                        .cloned()
                        .unwrap_or_else(|| original.clone());
                    apps.push(AppEntry {
                        id: format!("startmenu:{}", path.display()),
                        name: display,
                        original_name: original,
                        exe_path: path_str,
                        icon_base64: None,
                        icon_path: None,
                        icon_source: None,
                        category: "Application".to_string(),
                        source: "StartMenu".to_string(),
                        args: Vec::new(),
                        is_favorite: false,
                        is_pinned: false,
                        launch_mode: None,
                    });
                }
            }
        }
    }

    /// Start Menu shortcuts that are documentation, installers or vendor links
    /// rather than programs a user would launch remotely.
    ///
    /// Keeping these out of the catalog is the cheapest way to shrink the phone's
    /// list: the Start Menu is dominated by them.
    fn is_start_menu_noise(name: &str) -> bool {
        const NOISE_KEYWORDS: &[&str] = &[
            "uninstall",
            "卸载",
            "remove",
            "setup",
            "install",
            "update",
            "updater",
            "help",
            "帮助",
            "website",
            "官网",
            "feedback",
            "反馈",
            "readme",
            "read me",
            "read_me",
            "release notes",
            "releasenotes",
            "changelog",
            "license",
            "许可",
            "user guide",
            "userguide",
            "manual",
            "手册",
            "说明",
            "documentation",
            "repair",
            "修复",
            "diagnose",
            "诊断",
            "uninstaller",
            "support",
            "技术支持",
            "crash report",
            "crashreport",
        ];
        let name = name.to_lowercase();
        NOISE_KEYWORDS.iter().any(|keyword| name.contains(keyword))
    }
}
