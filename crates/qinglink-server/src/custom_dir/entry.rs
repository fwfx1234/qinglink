//! 与 `AppScanner` 的联动边界：把扫描命中的文件路径映射为扫描器条目。

use std::path::Path;

use crate::service::app_scanner::AppEntry;

use super::CustomDirManager;
use super::model::CustomDir;

impl CustomDirManager {
    /// 把命中的文件路径映射为 `AppEntry`：快捷方式归应用，其余归游戏。
    pub(super) fn path_to_app_entry(&self, path: &Path, config: &CustomDir) -> Option<AppEntry> {
        let name = path.file_stem().and_then(|s| s.to_str())?.to_string();
        let original = name.clone();

        let category = if path.extension().and_then(|e| e.to_str()) == Some("lnk") {
            "Application".to_string()
        } else {
            "Game".to_string()
        };

        let exe_path = path.to_str()?.to_string();

        Some(AppEntry {
            id: format!("custom:{}:{}", config.id, path.display()),
            name,
            original_name: original,
            exe_path,
            // Icons are loaded on demand by the client. Extracting them for
            // every executable makes a large directory scan unnecessarily slow.
            icon_base64: None,
            icon_path: None,
            icon_source: None,
            category,
            source: "Custom".to_string(),
            args: Vec::new(),
            is_favorite: false,
            is_pinned: false,
            launch_mode: None,
        })
    }
}
