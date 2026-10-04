//! 自定义目录管理 API：列表查询、增删改与目录校验。

use std::fs;
use std::path::Path;

use anyhow::Result;

use super::model::{
    AddCustomDirRequest, CustomDir, DirValidationResult, UpdateCustomDirRequest, default_extensions,
};
use super::{CustomDirManager, MAX_SCAN_DEPTH};

impl CustomDirManager {
    /// 获取所有自定义目录
    pub fn list(&self) -> &[CustomDir] {
        &self.config.dirs
    }

    /// 获取启用的目录
    pub fn list_enabled(&self) -> Vec<&CustomDir> {
        self.config.dirs.iter().filter(|d| d.enabled).collect()
    }

    /// 添加自定义目录
    ///
    /// # Errors
    ///
    /// 目录路径不存在、不是目录或无法规范化，扩展名列表为空或含非法项，
    /// 同一路径已存在，或写入配置文件失败时返回 `Err`。
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
    ///
    /// # Errors
    ///
    /// 提供了新路径但该路径不存在、不是目录或无法规范化，提供了非法或空的
    /// 扩展名列表，指定 ID 的目录不存在，或写入配置文件失败时返回 `Err`。
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
    ///
    /// # Errors
    ///
    /// 指定 ID 的目录不存在，或写入配置文件失败时返回 `Err`。
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
}
