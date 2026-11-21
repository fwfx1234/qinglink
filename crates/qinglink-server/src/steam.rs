//! Steam 游戏扫描器。
//!
//! 解析 Steam 库清单文件，提取已安装游戏的信息。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

mod icon_cache;

/// Steam 库文件夹信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SteamLibrary {
    /// 库文件夹的路径
    pub path: String,
    /// 该库中已安装的游戏数量
    pub game_count: usize,
}

/// Steam 游戏信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SteamGame {
    /// Steam 应用 ID
    pub app_id: u32,
    /// 游戏名称
    pub name: String,
    /// 安装目录名（steamapps/common 下的相对目录名）
    pub install_dir: String,
    /// 游戏安装的完整路径
    pub install_path: String,
    /// 所在 Steam 库的根路径
    pub library_path: String,
    /// 本地图标文件的路径
    pub icon_path: Option<String>,
    /// 磁盘占用字节数
    pub size_bytes: Option<u64>,
    /// 最近游玩时间（Unix 时间戳，秒）
    pub last_played: Option<i64>,
}

/// Steam 安装信息
#[derive(Clone, Debug)]
pub struct SteamInstallInfo {
    /// Steam 根目录
    pub root_path: PathBuf,
    /// 所有 Steam 库目录
    pub libraries: Vec<PathBuf>,
}

/// Steam 扫描器
#[derive(Debug)]
pub struct SteamScanner {
    install_info: Option<SteamInstallInfo>,
}

impl SteamScanner {
    /// 创建一个尚未定位 Steam 安装路径的扫描器。
    pub fn new() -> Self {
        Self { install_info: None }
    }

    /// 查找 Steam 安装路径
    pub fn find_steam_install(&mut self) -> Option<&SteamInstallInfo> {
        // 检查常见默认路径
        let default_paths = [
            PathBuf::from("C:\\Program Files (x86)\\Steam"),
            PathBuf::from("C:\\Program Files\\Steam"),
            PathBuf::from("D:\\Steam"),
            PathBuf::from("D:\\Program Files\\Steam"),
            PathBuf::from("E:\\Steam"),
            PathBuf::from("E:\\Program Files\\Steam"),
        ];

        for path in &default_paths {
            if self.is_steam_root(path) {
                self.install_info = Some(SteamInstallInfo {
                    root_path: path.clone(),
                    libraries: vec![path.clone()],
                });
                return self.install_info.as_ref();
            }
        }

        // 从注册表查找
        if let Some(path) = Self::find_steam_from_registry()
            && self.is_steam_root(&path)
        {
            self.install_info = Some(SteamInstallInfo {
                root_path: path.clone(),
                libraries: vec![path],
            });
            return self.install_info.as_ref();
        }

        // 兜底：遍历所有磁盘的常见 Steam 目录
        if let Some(path) = Self::find_steam_on_drives() {
            self.install_info = Some(SteamInstallInfo {
                root_path: path.clone(),
                libraries: vec![path],
            });
            return self.install_info.as_ref();
        }

        None
    }

    /// 在所有磁盘上查找常见位置的 Steam 安装目录。
    fn find_steam_on_drives() -> Option<PathBuf> {
        for letter in b'C'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            let candidates = [
                format!("{drive}Steam"),
                format!("{drive}Program Files (x86)\\Steam"),
                format!("{drive}Program Files\\Steam"),
                format!("{drive}Games\\Steam"),
                format!("{drive}SteamLibrary"),
            ];
            for candidate in candidates {
                let path = PathBuf::from(&candidate);
                if path.join("Steam.exe").exists() && path.join("steamapps").is_dir() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// 从注册表查找 Steam 安装路径
    fn find_steam_from_registry() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Registry::{
                HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
            };
            use windows::core::w;

            let subkeys = [r"SOFTWARE\Wow6432Node\Valve\Steam", r"SOFTWARE\Valve\Steam"];

            for subkey in &subkeys {
                unsafe {
                    let mut hkey = windows::Win32::System::Registry::HKEY::default();
                    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(Some(0)).collect();
                    if RegOpenKeyExW(
                        HKEY_LOCAL_MACHINE,
                        windows::core::PCWSTR(subkey_wide.as_ptr()),
                        Some(0),
                        KEY_READ,
                        &mut hkey,
                    )
                    .is_err()
                    {
                        continue;
                    }

                    let value_name = w!("InstallPath");
                    let mut data_type = windows::Win32::System::Registry::REG_VALUE_TYPE(0);
                    let mut data_len = 0u32;

                    if RegQueryValueExW(hkey, value_name, None, None, None, Some(&mut data_len))
                        .is_err()
                    {
                        let _ = RegCloseKey(hkey);
                        continue;
                    }

                    let buf_len = (data_len as usize) / 2 + 1;
                    let mut buf = vec![0u16; buf_len];
                    if RegQueryValueExW(
                        hkey,
                        value_name,
                        None,
                        Some(&mut data_type),
                        Some(buf.as_mut_ptr() as *mut u8),
                        Some(&mut data_len),
                    )
                    .is_err()
                    {
                        let _ = RegCloseKey(hkey);
                        continue;
                    }
                    let _ = RegCloseKey(hkey);

                    let len = (data_len as usize) / 2;
                    let path_str = String::from_utf16_lossy(&buf[..len]);
                    let path = PathBuf::from(path_str.trim_end_matches('\0'));
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    /// 判断是否为 Steam 根目录
    fn is_steam_root(&self, path: &Path) -> bool {
        path.join("Steam.exe").exists() && path.join("steamapps").is_dir()
    }

    /// 解析所有库文件夹
    ///
    /// # Errors
    ///
    /// 尚未定位到 Steam 安装路径（内部 `install_info` 为空）时返回 `Err`；
    /// 读取 `<Steam>/steamapps/libraryfolders.vdf` 失败时同样返回 `Err`。
    pub fn parse_library_folders(&mut self) -> Result<Vec<PathBuf>> {
        let info = self
            .install_info
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Steam 未安装"))?;

        let vdf_path = info.root_path.join("steamapps").join("libraryfolders.vdf");

        if !vdf_path.exists() {
            return Ok(vec![info.root_path.clone()]);
        }

        let content = fs::read_to_string(&vdf_path)
            .map_err(|e| anyhow::anyhow!("读取 libraryfolders.vdf 失败: {}", e))?;

        let mut libraries = vec![info.root_path.clone()];

        for line in content.lines() {
            if let Some((key, value)) = Self::parse_vdf_line(line)
                && key == "path"
            {
                let path = PathBuf::from(value);
                if !libraries.contains(&path) && path.join("steamapps").is_dir() {
                    libraries.push(path);
                }
            }
        }

        if let Some(info) = &mut self.install_info {
            info.libraries = libraries.clone();
        }

        Ok(libraries)
    }

    /// 扫描所有 Steam 游戏
    ///
    /// # Errors
    ///
    /// 解析库文件夹列表失败（见 [`Self::parse_library_folders`]）时返回 `Err`；
    /// 读取某个库的 `steamapps` 目录失败时同样返回 `Err`。单个
    /// `appmanifest_*.acf` 解析失败只会跳过该游戏，不会让整体扫描失败。
    pub fn scan_games(&mut self) -> Result<Vec<SteamGame>> {
        let libraries = self.parse_library_folders()?;
        let mut games = Vec::new();

        for library in &libraries {
            let steamapps = library.join("steamapps");
            if !steamapps.is_dir() {
                continue;
            }

            let entries = fs::read_dir(&steamapps)
                .map_err(|e| anyhow::anyhow!("读取 steamapps 目录失败: {}", e))?;

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if file_name.starts_with("appmanifest_")
                    && file_name.ends_with(".acf")
                    && let Ok(game) = self.parse_app_manifest(&path, library)
                {
                    games.push(game);
                }
            }
        }

        Ok(games)
    }

    /// 解析 appmanifest_*.acf 文件
    fn parse_app_manifest(&self, path: &Path, library_path: &Path) -> Result<SteamGame> {
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("读取 appmanifest 失败: {}", e))?;

        let mut app_id = 0u32;
        let mut name = String::new();
        let mut install_dir = String::new();
        let mut size_bytes = None;
        let mut last_played = None;

        let mut in_app_state = false;
        let mut brace_depth = 0;

        for line in content.lines() {
            let line = line.trim();

            if line == "\"AppState\"" {
                in_app_state = true;
                continue;
            }

            if in_app_state {
                if line == "{" {
                    brace_depth += 1;
                    continue;
                }
                if line == "}" {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_app_state = false;
                    }
                    continue;
                }

                if let Some((key, value)) = Self::parse_vdf_line(line) {
                    match key.as_str() {
                        "appid" => app_id = value.parse().unwrap_or(0),
                        "name" => name = value,
                        "installdir" => install_dir = value,
                        "SizeOnDisk" => {
                            size_bytes = value.parse().ok();
                        }
                        "LastPlayed" => {
                            last_played = value.parse().ok();
                        }
                        _ => {}
                    }
                }
            }
        }

        let install_path = library_path
            .join("steamapps")
            .join("common")
            .join(&install_dir);

        let icon_path = self.find_steam_icon(app_id, library_path);

        if app_id == 0 || name.is_empty() || install_dir.is_empty() {
            anyhow::bail!("Steam manifest is missing a valid app id, name, or install directory");
        }

        Ok(SteamGame {
            app_id,
            name,
            install_dir,
            install_path: install_path.to_string_lossy().to_string(),
            library_path: library_path.to_string_lossy().to_string(),
            icon_path,
            size_bytes,
            last_played,
        })
    }

    /// 解析 VDF 行
    fn parse_vdf_line(line: &str) -> Option<(String, String)> {
        let line = line.trim();
        let rest = line.strip_prefix('"')?;
        let key_end = rest.find('"')?;
        let key = rest[..key_end].to_string();
        let value = rest[key_end + 1..].trim().trim_matches('"');
        Some((key, value.replace("\\\\", "\\")))
    }

    /// 查找 Steam 游戏图标
    fn find_steam_icon(&self, app_id: u32, library_path: &Path) -> Option<String> {
        let steam_root = self
            .install_info
            .as_ref()
            .map(|info| info.root_path.as_path());
        icon_cache::find(steam_root, library_path, app_id)
    }
}

impl Default for SteamScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::SteamScanner;

    #[test]
    fn parses_manifest_values_without_quotes_or_leading_tabs() {
        assert_eq!(
            SteamScanner::parse_vdf_line("\t\"appid\"\t\t\"1446780\""),
            Some(("appid".to_string(), "1446780".to_string()))
        );
        assert_eq!(
            SteamScanner::parse_vdf_line("\t\"installdir\"\t\t\"MonsterHunterRise\""),
            Some(("installdir".to_string(), "MonsterHunterRise".to_string()))
        );
    }
}
