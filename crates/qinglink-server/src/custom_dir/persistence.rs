//! 配置文件持久化，以及写入前的路径与扩展名校验。

use std::fs;
use std::path::Path;

use anyhow::Result;

use super::CustomDirManager;
use super::model::CustomDirConfig;

impl CustomDirManager {
    /// 写入配置前校验目录路径：必须存在、是目录，并去掉 Windows 的 `\\?\` 前缀。
    pub(super) fn validate_directory_path(path: &str) -> Result<String> {
        let path = Path::new(path);
        if !path.exists() {
            anyhow::bail!("路径不存在: {}", path.display());
        }
        if !path.is_dir() {
            anyhow::bail!("路径不是目录: {}", path.display());
        }
        let canonical = path.canonicalize()?.to_string_lossy().to_string();
        Ok(canonical
            .strip_prefix(r"\\?\")
            .unwrap_or(&canonical)
            .to_string())
    }

    /// 写入配置前规范化扩展名：去前导点、转小写、去重，并拒绝非法项。
    pub(super) fn normalize_extensions(extensions: Vec<String>) -> Result<Vec<String>> {
        let mut normalized = Vec::new();
        for extension in extensions {
            let extension = extension
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase();
            if extension.is_empty()
                || extension.len() > 16
                || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                anyhow::bail!("无效的扩展名: {extension}");
            }
            if !normalized.contains(&extension) {
                normalized.push(extension);
            }
        }
        if normalized.is_empty() {
            anyhow::bail!("至少需要一个文件扩展名");
        }
        Ok(normalized)
    }

    /// 基于配置文件路径创建管理器，配置读取失败时回退为空配置。
    pub fn new(config_path: std::path::PathBuf) -> Self {
        let config = Self::load_config(&config_path).unwrap_or_default();
        Self {
            config,
            config_path,
        }
    }

    /// 加载配置
    fn load_config(path: &Path) -> Option<CustomDirConfig> {
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 保存配置
    pub(super) fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }
}
