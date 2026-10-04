//! 扫描使用的路径与名称规范化。

use crate::service::app_scanner::AppScanner;

impl AppScanner {
    pub(super) fn normalize_path(path: &str) -> String {
        path.trim()
            .trim_matches('"')
            .replace('/', "\\")
            .to_lowercase()
    }

    /// Drop the Windows verbatim (`\\?\`) prefix from a canonical path.
    pub(super) fn strip_verbatim(path: std::path::PathBuf) -> std::path::PathBuf {
        let text = path.to_string_lossy().to_string();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            std::path::PathBuf::from(format!(r"\\{rest}"))
        } else if let Some(rest) = text.strip_prefix(r"\\?\") {
            std::path::PathBuf::from(rest)
        } else {
            path
        }
    }

    /// Normalize a name for fuzzy comparison (`Half-Life 2` → `halflife2`).
    pub(super) fn fold_name(value: &str) -> String {
        value
            .chars()
            .filter(|character| character.is_alphanumeric())
            .flat_map(|character| character.to_lowercase())
            .collect()
    }
}
