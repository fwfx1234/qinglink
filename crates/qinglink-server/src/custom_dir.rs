//! Custom directory management.
//!
//! Allows users to specify additional directories for application scanning.

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

const MAX_SCAN_DEPTH: u32 = 8;
const MAX_SCAN_ENTRIES: usize = 10_000;

/// 自定义扫描目录配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomDir {
    pub id: String,
    pub path: String,
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub max_depth: u32,
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    #[serde(default = "default_bool_true")]
    pub recursive: bool,
    pub created_at: i64,
    pub last_scanned: Option<i64>,
}

fn default_extensions() -> Vec<String> {
    vec!["exe".to_string(), "lnk".to_string()]
}

fn default_bool_true() -> bool {
    true
}

/// 自定义目录配置集合
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CustomDirConfig {
    #[serde(default)]
    pub dirs: Vec<CustomDir>,
}

/// 添加自定义目录请求
#[derive(Clone, Debug, Deserialize)]
pub struct AddCustomDirRequest {
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    #[serde(default)]
    pub max_depth: u32,
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    #[serde(default = "default_bool_true")]
    pub recursive: bool,
}

/// 更新自定义目录请求
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateCustomDirRequest {
    pub path: Option<String>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub max_depth: Option<u32>,
    pub extensions: Option<Vec<String>>,
    pub recursive: Option<bool>,
}

/// 目录验证结果
#[derive(Clone, Debug, Serialize)]
pub struct DirValidationResult {
    pub valid: bool,
    pub exists: bool,
    pub is_dir: bool,
    pub readable: bool,
    pub exe_count: usize,
    pub error: Option<String>,
}

/// 自定义目录管理器
pub struct CustomDirManager {
    config: CustomDirConfig,
    config_path: std::path::PathBuf,
}

impl CustomDirManager {
    fn validate_directory_path(path: &str) -> Result<String> {
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

    fn normalize_extensions(extensions: Vec<String>) -> Result<Vec<String>> {
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
    fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// 获取所有自定义目录
    pub fn list(&self) -> &[CustomDir] {
        &self.config.dirs
    }

    /// 获取启用的目录
    pub fn list_enabled(&self) -> Vec<&CustomDir> {
        self.config.dirs.iter().filter(|d| d.enabled).collect()
    }

    /// 添加自定义目录
    pub fn add(&mut self, req: AddCustomDirRequest) -> Result<CustomDir> {
        let path = Self::validate_directory_path(&req.path)?;
        let extensions =
            Self::normalize_extensions(req.extensions.unwrap_or_else(default_extensions))?;

        if self.config.dirs.iter().any(|d| d.path == path) {
            anyhow::bail!("目录已存在: {path}");
        }

        let dir = CustomDir {
            id: format!("custom_{}", &uuid::Uuid::new_v4().to_string()[..8]),
            path: path.clone(),
            name: req.name.unwrap_or_else(|| {
                Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("未命名")
                    .to_string()
            }),
            enabled: req.enabled,
            max_depth: req.max_depth.min(MAX_SCAN_DEPTH),
            extensions,
            recursive: req.recursive,
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            last_scanned: None,
        };

        self.config.dirs.push(dir.clone());
        self.save()?;

        Ok(dir)
    }

    /// 更新自定义目录
    pub fn update(&mut self, id: &str, req: UpdateCustomDirRequest) -> Result<CustomDir> {
        let path = req
            .path
            .as_deref()
            .map(Self::validate_directory_path)
            .transpose()?;
        let extensions = req.extensions.map(Self::normalize_extensions).transpose()?;
        let dir = self
            .config
            .dirs
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| anyhow::anyhow!("目录不存在: {}", id))?;

        if let Some(path) = path {
            dir.path = path;
        }
        if let Some(name) = req.name {
            dir.name = name;
        }
        if let Some(enabled) = req.enabled {
            dir.enabled = enabled;
        }
        if let Some(max_depth) = req.max_depth {
            dir.max_depth = max_depth.min(MAX_SCAN_DEPTH);
        }
        if let Some(extensions) = extensions {
            dir.extensions = extensions;
        }
        if let Some(recursive) = req.recursive {
            dir.recursive = recursive;
        }

        let dir = dir.clone();
        self.save()?;

        Ok(dir)
    }

    /// 删除自定义目录
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let len_before = self.config.dirs.len();
        self.config.dirs.retain(|d| d.id != id);
        if self.config.dirs.len() == len_before {
            anyhow::bail!("目录不存在: {}", id);
        }
        self.save()
    }

    /// 验证目录
    pub fn validate(path: &str) -> DirValidationResult {
        let path = Path::new(path);

        if !path.exists() {
            return DirValidationResult {
                valid: false,
                exists: false,
                is_dir: false,
                readable: false,
                exe_count: 0,
                error: Some("路径不存在".to_string()),
            };
        }

        if !path.is_dir() {
            return DirValidationResult {
                valid: false,
                exists: true,
                is_dir: false,
                readable: false,
                exe_count: 0,
                error: Some("路径不是目录".to_string()),
            };
        }

        match fs::read_dir(path) {
            Ok(entries) => {
                let exe_count = entries
                    .flatten()
                    .filter(|e| {
                        e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("exe"))
                            .unwrap_or(false)
                    })
                    .count();

                DirValidationResult {
                    valid: true,
                    exists: true,
                    is_dir: true,
                    readable: true,
                    exe_count,
                    error: None,
                }
            }
            Err(e) => DirValidationResult {
                valid: false,
                exists: true,
                is_dir: true,
                readable: false,
                exe_count: 0,
                error: Some(format!("无法读取目录: {}", e)),
            },
        }
    }

    /// 扫描自定义目录生成 AppEntry
    pub fn scan(&self) -> Vec<super::service::app_scanner::AppEntry> {
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

    fn scan_recursive(
        &self,
        dir: &Path,
        config: &CustomDir,
        apps: &mut Vec<super::service::app_scanner::AppEntry>,
        depth: u32,
    ) {
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

    fn scan_flat(
        &self,
        dir: &Path,
        config: &CustomDir,
        apps: &mut Vec<super::service::app_scanner::AppEntry>,
    ) {
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

    fn path_to_app_entry(
        &self,
        path: &Path,
        config: &CustomDir,
    ) -> Option<super::service::app_scanner::AppEntry> {
        let name = path.file_stem().and_then(|s| s.to_str())?.to_string();
        let original = name.clone();

        let category = if path.extension().and_then(|e| e.to_str()) == Some("lnk") {
            "Application".to_string()
        } else {
            "Game".to_string()
        };

        let exe_path = path.to_str()?.to_string();

        Some(super::service::app_scanner::AppEntry {
            id: format!("custom:{}:{}", config.id, path.display()),
            name,
            original_name: original,
            exe_path,
            // Icons are loaded on demand by the client. Extracting them for
            // every executable makes a large directory scan unnecessarily slow.
            icon_base64: None,
            category,
            source: "Custom".to_string(),
            args: Vec::new(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_config_path() -> std::path::PathBuf {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_custom_dirs.json");
        // 防止 tempdir 在测试结束时被删除
        std::mem::forget(dir);
        path
    }

    #[test]
    fn add_and_list_custom_dir() {
        let path = temp_config_path();
        let mut manager = CustomDirManager::new(path.clone());

        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().to_string_lossy().to_string();

        let req = AddCustomDirRequest {
            path: test_path.clone(),
            name: Some("测试目录".to_string()),
            enabled: true,
            max_depth: 3,
            extensions: None,
            recursive: true,
        };

        let dir = manager.add(req).unwrap();
        assert_eq!(dir.path, test_path);
        assert_eq!(dir.name, "测试目录");
        assert!(dir.enabled);
        assert_eq!(dir.max_depth, 3);

        let dirs = manager.list();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].path, test_path);

        std::mem::forget(test_dir);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn add_duplicate_dir_fails() {
        let path = temp_config_path();
        let mut manager = CustomDirManager::new(path.clone());

        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().to_string_lossy().to_string();

        let req = AddCustomDirRequest {
            path: test_path.clone(),
            name: None,
            enabled: true,
            max_depth: 0,
            extensions: None,
            recursive: true,
        };

        manager.add(req.clone()).unwrap();
        let result = manager.add(req);
        assert!(result.is_err(), "重复添加应失败");

        std::mem::forget(test_dir);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn add_nonexistent_path_fails() {
        let path = temp_config_path();
        let mut manager = CustomDirManager::new(path.clone());

        let req = AddCustomDirRequest {
            path: "Z:\\不存在的路径\\测试".to_string(),
            name: None,
            enabled: true,
            max_depth: 0,
            extensions: None,
            recursive: true,
        };

        let result = manager.add(req);
        assert!(result.is_err(), "添加不存在的路径应失败");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn update_custom_dir() {
        let path = temp_config_path();
        let mut manager = CustomDirManager::new(path.clone());

        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().to_string_lossy().to_string();

        let req = AddCustomDirRequest {
            path: test_path.clone(),
            name: Some("原始名称".to_string()),
            enabled: true,
            max_depth: 0,
            extensions: None,
            recursive: true,
        };

        let dir = manager.add(req).unwrap();
        let id = dir.id.clone();

        let update_req = UpdateCustomDirRequest {
            path: None,
            name: Some("新名称".to_string()),
            enabled: Some(false),
            max_depth: Some(5),
            extensions: None,
            recursive: None,
        };

        let updated = manager.update(&id, update_req).unwrap();
        assert_eq!(updated.name, "新名称");
        assert!(!updated.enabled);
        assert_eq!(updated.max_depth, 5);

        std::mem::forget(test_dir);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn remove_custom_dir() {
        let path = temp_config_path();
        let mut manager = CustomDirManager::new(path.clone());

        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().to_string_lossy().to_string();

        let req = AddCustomDirRequest {
            path: test_path.clone(),
            name: None,
            enabled: true,
            max_depth: 0,
            extensions: None,
            recursive: true,
        };

        let dir = manager.add(req).unwrap();
        assert_eq!(manager.list().len(), 1);

        manager.remove(&dir.id).unwrap();
        assert_eq!(manager.list().len(), 0);

        std::mem::forget(test_dir);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn remove_nonexistent_dir_fails() {
        let path = temp_config_path();
        let mut manager = CustomDirManager::new(path.clone());

        let result = manager.remove("不存在的ID");
        assert!(result.is_err(), "删除不存在的目录应失败");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn validate_existing_directory() {
        let test_dir = tempfile::tempdir().unwrap();
        let result = CustomDirManager::validate(test_dir.path().to_str().unwrap());

        assert!(result.valid);
        assert!(result.exists);
        assert!(result.is_dir);
        assert!(result.readable);
    }

    #[test]
    fn validate_nonexistent_path() {
        let result = CustomDirManager::validate("Z:\\不存在的路径\\测试");

        assert!(!result.valid);
        assert!(!result.exists);
        assert!(!result.is_dir);
        assert!(!result.readable);
        assert!(result.error.is_some());
    }

    #[test]
    fn validate_file_instead_of_dir() {
        let path = temp_config_path();
        let _ = fs::write(&path, "test");

        let result = CustomDirManager::validate(path.to_str().unwrap());
        assert!(!result.valid);
        assert!(result.exists);
        assert!(!result.is_dir);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn persistence_across_instances() {
        let path = temp_config_path();

        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().to_string_lossy().to_string();

        {
            let mut manager = CustomDirManager::new(path.clone());
            let req = AddCustomDirRequest {
                path: test_path.clone(),
                name: Some("持久化测试".to_string()),
                enabled: true,
                max_depth: 0,
                extensions: None,
                recursive: true,
            };
            manager.add(req).unwrap();
        }

        {
            let manager = CustomDirManager::new(path.clone());
            let dirs = manager.list();
            assert_eq!(dirs.len(), 1);
            assert_eq!(dirs[0].name, "持久化测试");
        }

        std::mem::forget(test_dir);
        let _ = fs::remove_file(&path);
    }
}
