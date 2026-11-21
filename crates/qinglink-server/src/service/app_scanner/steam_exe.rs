//! Steam 游戏主可执行文件的定位。
//!
//! Steam 的清单里没有"该启动哪个 exe"，只能靠文件系统猜，而猜错的代价是整个
//! 条目的图标（拿到崩溃上报器的图，或者根本找不到）。所以这里不"取第一个"：
//! 先收集候选，排掉明显不是游戏主体的程序，再按名字与目录名的相似度打分。

use std::path::{Path, PathBuf};

/// 明显不是游戏主体的可执行文件：崩溃上报器、前置运行库安装器、卸载器。
///
/// 这些文件散落在游戏目录各处，而"取找到的第一个 exe"最容易先撞上它们——
/// Crimson Desert 就是这么拿到 `bin64/crashpad_handler.exe` 的图标。
/// 比较前两边都已归一化，所以这里也写归一化后的名字（小写、无分隔符、前缀匹配）。
const NON_GAME_EXES: [&str; 13] = [
    "crashpadhandler",
    "crashreportclient",
    "unitycrashhandler",
    "ue4prereqsetup",
    "ue5prereqsetup",
    "prereqsetup",
    "vcredist",
    "dxsetup",
    "dxwebsetup",
    "dotnetfx",
    "oalinst",
    "unins0",
    "uninstall",
];

/// 相对游戏安装目录的最大搜索深度。
///
/// Unreal 工程的程序常在 `<项目>/Binaries/Win64/x.exe` 下（相对深度 3），
/// 旧版只挖两层，八方旅人这种布局连主程序都找不到，`exe_path` 于是退化成安装目录。
const EXE_SEARCH_DEPTH: u32 = 3;

/// 一次查找最多进入的目录数，避免在几百 GB 的游戏目录里做无谓遍历。
const EXE_SEARCH_BUDGET: usize = 4096;

/// 在游戏安装目录里找出最可能是主程序的那个 exe。
pub(super) fn find_main_exe(
    install_path: &str,
    game_name: &str,
    install_dir: &str,
) -> Option<String> {
    let root = Path::new(install_path);
    if !root.is_dir() {
        return None;
    }
    ExeSearch::new(install_dir, game_name).run(root)
}

/// 一次主程序查找的上下文：归一化后的目标名字与剩余的目录预算。
struct ExeSearch {
    /// 安装目录名（`installdir`），比清单里的名字更接近真实文件名。
    install_key: String,
    /// 清单里的游戏名。
    name_key: String,
    budget: usize,
    best: Option<(i64, String)>,
}

impl ExeSearch {
    fn new(install_dir: &str, game_name: &str) -> Self {
        Self {
            install_key: normalize_name(install_dir),
            name_key: normalize_name(game_name),
            budget: EXE_SEARCH_BUDGET,
            best: None,
        }
    }

    fn run(mut self, root: &Path) -> Option<String> {
        self.visit(root, 0);
        self.best.map(|(_, path)| path)
    }

    /// 遍历一层目录：本层的 exe 参与打分，子目录继续往下（深度受 [`EXE_SEARCH_DEPTH`] 限制）。
    fn visit(&mut self, dir: &Path, depth: u32) {
        if self.budget == 0 || depth > EXE_SEARCH_DEPTH {
            return;
        }
        self.budget -= 1;

        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        for path in &paths {
            if path.is_file() {
                self.consider(path, depth);
            }
        }
        if depth == EXE_SEARCH_DEPTH {
            return;
        }
        for path in &paths {
            if path.is_dir() {
                self.visit(path, depth + 1);
            }
        }
    }

    /// 给一个候选打分，只保留最高的那个。
    fn consider(&mut self, path: &Path, depth: u32) {
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            return;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            return;
        };
        let key = normalize_name(stem);
        if key.is_empty() || is_non_game_exe(&key) {
            return;
        }

        let size = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
        let score = score_exe(&key, &self.install_key, &self.name_key, depth, size);
        if self.best.as_ref().is_none_or(|(best, _)| score > *best) {
            self.best = Some((score, path.to_string_lossy().to_string()));
        }
    }
}

/// 一个 exe 的得分。
///
/// 名字与安装目录 / 游戏名一致最高，互相包含次之，都不沾边时只能靠"更浅、更大"
/// 占一点便宜：几百 MB 的主程序永远比几十 KB 的辅助程序更像游戏本体。
fn score_exe(key: &str, install_key: &str, name_key: &str, depth: u32, size: u64) -> i64 {
    let exact = key == install_key || key == name_key;
    let contained = [install_key, name_key]
        .iter()
        .any(|wanted| !wanted.is_empty() && (key.contains(*wanted) || wanted.contains(key)));
    let name_score = if exact {
        1000
    } else if contained {
        600
    } else {
        0
    };

    let depth_penalty = i64::from(depth) * 50;
    let size_bonus = (size / (1024 * 1024)).min(200) as i64;
    name_score - depth_penalty + size_bonus
}

/// 名字归一化：只留字母数字并统一小写，`CrimsonDesert.exe`、`Crimson Desert.exe`
/// 与安装目录 `Crimson Desert` 因此能对上。
fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_non_game_exe(key: &str) -> bool {
    NON_GAME_EXES.iter().any(|name| key.starts_with(*name))
}
