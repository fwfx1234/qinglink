//! 配置文件持久化，以及写入前的路径与扩展名校验。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

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

    /// 基于配置文件路径创建管理器。
    ///
    /// 配置读不出来时**不静默回退**：损坏的文件会改名保留现场并记一条 error，
    /// 然后以空配置启动。原来的 `.ok()?` + `unwrap_or_default()` 会让用户下一次
    /// 新增一个目录时，把整份旧配置覆盖成"只剩这一个新目录"。
    pub fn new(config_path: std::path::PathBuf) -> Self {
        let config = match Self::load_config(&config_path) {
            Ok(config) => config.unwrap_or_default(),
            Err(error) => {
                let backup = Self::quarantine_corrupt(&config_path);
                tracing::error!(
                    config = %config_path.display(),
                    backup = backup.as_ref().map(|path| path.display().to_string()),
                    %error,
                    "自定义目录配置读取失败，已保留原文件并以空配置启动"
                );
                CustomDirConfig::default()
            }
        };
        Self {
            config,
            config_path,
        }
    }

    /// 加载配置；`Ok(None)` 表示配置文件不存在（首次启动的正常情形）。
    ///
    /// IO 错误与 JSON 解析错误都向上返回：它们的共同点是"文件里有内容但读不懂"，
    /// 与"没有文件"完全不是一回事。
    fn load_config(path: &Path) -> Result<Option<CustomDirConfig>> {
        if !path.exists() {
            return Ok(None);
        }
        let content =
            fs::read_to_string(path).with_context(|| format!("读取 {} 失败", path.display()))?;
        let config = serde_json::from_str(&content)
            .with_context(|| format!("解析 {} 失败", path.display()))?;
        Ok(Some(config))
    }

    /// 把读不出来的配置文件改名保留，返回新路径；改名失败返回 `None`。
    ///
    /// 保留现场比"修好它"更实际：解析失败的配置通常已经被外部工具改坏，
    /// 直接覆盖会连同用户手工添加的目录一起丢掉。
    fn quarantine_corrupt(path: &Path) -> Option<std::path::PathBuf> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();
        let backup = path.with_extension(format!("corrupt-{stamp}"));
        fs::rename(path, &backup).ok().map(|()| backup)
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
