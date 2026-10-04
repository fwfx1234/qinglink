//! 文件浏览器：列出目录内容、可用盘符与常用快捷入口。

use serde::Serialize;
use std::path::Path;

#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;

/// 目录中的一个条目（文件、子目录或盘符）。
#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    /// 显示名称（文件名或盘符名称）。
    pub name: String,
    /// 条目的完整路径。
    pub path: String,
    /// 是否为目录。
    pub is_dir: bool,
    /// 文件大小（字节）；目录为 0。
    pub size: u64,
    /// 最后修改时间（Unix 秒），无法获取时为 `None`。
    pub modified: Option<u64>,
    /// 是否为隐藏项（点开头或带 Windows 隐藏属性）。
    pub is_hidden: bool,
}

/// 一次目录浏览的结果，含当前路径、父路径与全部条目。
#[derive(Debug, Clone, Serialize)]
pub struct DirectoryListing {
    /// 被浏览的目录路径。
    pub current_path: String,
    /// 父目录路径；已是根目录时为 `None`。
    pub parent_path: Option<String>,
    /// 目录内的条目，目录在前、其余按名称排序。
    pub entries: Vec<FileEntry>,
    /// 条目总数（分页前的完整数量）。
    pub total_count: usize,
}

/// 列出指定目录下的全部条目；路径不存在或不是目录时返回错误。
///
/// # Errors
/// `path` 不存在、不是目录，或读取目录内容失败时返回 `Err`。
pub fn browse_directory(path: &str) -> Result<DirectoryListing, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err("路径不存在".to_string());
    }
    if !path.is_dir() {
        return Err("不是目录".to_string());
    }

    let mut entries: Vec<FileEntry> = match std::fs::read_dir(path) {
        Ok(read_dir) => read_dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let metadata = entry.metadata().ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                let is_hidden = is_hidden_file(&name, &metadata);
                Some(FileEntry {
                    name,
                    path: entry.path().to_string_lossy().to_string(),
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                    modified: metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()),
                    is_hidden,
                })
            })
            .collect(),
        Err(e) => return Err(format!("读取目录失败: {e}")),
    };

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let total_count = entries.len();

    Ok(DirectoryListing {
        current_path: path.to_string_lossy().to_string(),
        parent_path: path.parent().map(|p| p.to_string_lossy().to_string()),
        entries,
        total_count,
    })
}

#[cfg(target_os = "windows")]
fn is_hidden_file(name: &str, metadata: &std::fs::Metadata) -> bool {
    name.starts_with('.') || (metadata.file_attributes() & 0x2) != 0
}

#[cfg(not(target_os = "windows"))]
fn is_hidden_file(name: &str, _metadata: &std::fs::Metadata) -> bool {
    name.starts_with('.')
}

/// 枚举本机存在的盘符（A:–Z:），用于文件浏览器的根列表。
pub fn get_drives() -> Vec<FileEntry> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let path = format!("{}:\\", letter as char);
        if Path::new(&path).is_dir() {
            drives.push(FileEntry {
                name: format!("{} 盘", letter as char),
                path,
                is_dir: true,
                size: 0,
                modified: None,
                is_hidden: false,
            });
        }
    }
    drives
}

/// 常用入口列表：桌面、文档、下载、主目录，以及存在的游戏 / Steam 目录。
pub fn get_quick_access() -> Vec<(String, String)> {
    let mut result = Vec::new();

    if let Some(dir) = dirs::desktop_dir() {
        result.push(("桌面".to_string(), dir.to_string_lossy().to_string()));
    }
    if let Some(dir) = dirs::document_dir() {
        result.push(("文档".to_string(), dir.to_string_lossy().to_string()));
    }
    if let Some(dir) = dirs::download_dir() {
        result.push(("下载".to_string(), dir.to_string_lossy().to_string()));
    }
    if let Some(dir) = dirs::home_dir() {
        result.push(("主目录".to_string(), dir.to_string_lossy().to_string()));
    }

    for drive in &["C:", "D:", "E:", "F:"] {
        let game_dir = format!("{}\\Games", drive);
        if Path::new(&game_dir).is_dir() {
            result.push((format!("{} 游戏", drive), game_dir));
        }
        let steam_dir = format!("{}\\Program Files (x86)\\Steam\\steamapps\\common", drive);
        if Path::new(&steam_dir).is_dir() {
            result.push((format!("{} Steam", drive), steam_dir));
        }
    }

    result
}
