//! 已启用目录的遍历扫描：深度与条目预算、重解析点跳过与扩展名过滤。

use std::fs;
use std::path::Path;

use crate::service::app_scanner::AppEntry;

use super::model::CustomDir;
use super::{CustomDirManager, MAX_SCAN_DEPTH};

/// 单次扫描收集的条目数量上限。
const MAX_SCAN_ENTRIES: usize = 10_000;

impl CustomDirManager {
    /// 扫描自定义目录生成 AppEntry
    pub fn scan(&self) -> Vec<AppEntry> {
        let mut apps = Vec::new();

        for dir in self.list_enabled() {
            let path = Path::new(&dir.path);
            if !path.exists() || !path.is_dir() {
                continue;
            }

            if dir.recursive {
                self.scan_recursive(path, dir, &mut apps, 0);
            } else {
                self.scan_flat(path, dir, &mut apps);
            }
        }

        apps
    }

    #[cfg(target_os = "windows")]
    fn is_reparse_point(path: &Path) -> bool {
        use std::os::windows::fs::MetadataExt;
        fs::symlink_metadata(path)
            .map(|metadata| metadata.file_attributes() & 0x400 != 0)
            .unwrap_or(true)
    }

    #[cfg(not(target_os = "windows"))]
    fn is_reparse_point(path: &Path) -> bool {
        fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(true)
    }

    fn scan_recursive(&self, dir: &Path, config: &CustomDir, apps: &mut Vec<AppEntry>, depth: u32) {
        let max_depth = if config.max_depth == 0 {
            MAX_SCAN_DEPTH
        } else {
            config.max_depth.min(MAX_SCAN_DEPTH)
        };
        if depth > max_depth {
            return;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            if apps.len() >= MAX_SCAN_ENTRIES {
                return;
            }
            let path = entry.path();
            if Self::is_reparse_point(&path) {
                continue;
            }
            if path.is_dir() {
                self.scan_recursive(&path, config, apps, depth + 1);
            } else if self.should_include_file(&path, config)
                && let Some(app) = self.path_to_app_entry(&path, config)
            {
                apps.push(app);
            }
        }
    }

    fn scan_flat(&self, dir: &Path, config: &CustomDir, apps: &mut Vec<AppEntry>) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            if apps.len() >= MAX_SCAN_ENTRIES {
                return;
            }
            let path = entry.path();
            if path.is_file()
                && self.should_include_file(&path, config)
                && let Some(app) = self.path_to_app_entry(&path, config)
            {
                apps.push(app);
            }
        }
    }

    fn should_include_file(&self, path: &Path, config: &CustomDir) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                config
                    .extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext))
            })
            .unwrap_or(false)
    }
}
