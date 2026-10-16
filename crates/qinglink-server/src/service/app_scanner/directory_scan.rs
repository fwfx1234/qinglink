//! 用户选择的目录扫描：时间与目录预算、可执行文件收集和推荐度排序。

use std::collections::HashMap;

use crate::service::app_scanner::{AppEntry, AppScanner, ScanCandidate};

/// Time/directory budget for one user-initiated directory scan.
struct ScanBudget {
    deadline: std::time::Instant,
    visited_dirs: usize,
    max_dirs: usize,
}

impl ScanBudget {
    fn new(time_budget: std::time::Duration, max_dirs: usize) -> Self {
        Self {
            deadline: std::time::Instant::now() + time_budget,
            visited_dirs: 0,
            max_dirs,
        }
    }

    fn exhausted(&self) -> bool {
        self.visited_dirs >= self.max_dirs || std::time::Instant::now() >= self.deadline
    }
}

impl AppScanner {
    /// Executables inside a program directory that nobody launches directly.
    ///
    /// A triple-A game folder routinely contains a crash reporter, a
    /// redistributable installer, an updater and a settings tool next to the
    /// real executable. Listing all of them is what makes a folder scan look
    /// like "hundreds of games".
    fn is_auxiliary_game_executable(entry: &AppEntry) -> bool {
        const AUXILIARY_KEYWORDS: &[&str] = &[
            "unins",
            "uninstall",
            "crashhandler",
            "crash_handler",
            "crashreport",
            "crashpad_handler",
            "crs-handler",
            "crs-uploader",
            "unitycrashhandler",
            "unrealcefsubprocess",
            "padconfig",
            "settings",
            "config",
            "updater",
            "update",
            "patch",
            "installer",
            "setup",
            "launcher_helper",
            "vcredist",
            "dxsetup",
            "directx",
            "dotnetfx",
            "oalinst",
            "redist",
            "notification_helper",
            "activation",
        ];
        let name = std::path::Path::new(&entry.exe_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(&entry.name)
            .to_ascii_lowercase();
        AUXILIARY_KEYWORDS
            .iter()
            .any(|keyword| name.contains(keyword))
    }

    /// Scan one user-selected directory for launchable executables.
    ///
    /// `kind` selects whether the results are catalogued as games or as
    /// software; folder scans used to hard-code `Game`, which is why scanning a
    /// utilities folder filled the game list. Candidates are still not persisted
    /// until the client confirms a subset.
    ///
    /// # Errors
    /// `path` 不是目录，或无法规范化该路径（路径不存在、权限不足等）时返回 `Err`。
    pub fn scan_executable_directory(
        &self,
        path: &str,
        kind: &str,
    ) -> Result<Vec<ScanCandidate>, String> {
        let root = std::path::Path::new(path);
        if !root.is_dir() {
            return Err("选择的路径不是目录".to_string());
        }
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        // `canonicalize` yields `\\?\` verbatim paths which the shell cannot
        // launch, so normalize before producing candidates.
        let root = Self::strip_verbatim(root);
        let category = if kind.eq_ignore_ascii_case("game") {
            "Game"
        } else {
            "Application"
        };
        let mut candidates = Vec::new();
        // A parent folder that contains many game folders can hold a huge tree
        // (and junctions). Bound the wall-clock time and the number of visited
        // directories so the request cannot outlive the mobile timeout.
        let mut budget = ScanBudget::new(std::time::Duration::from_secs(8), 20_000);
        Self::collect_executable_candidates(
            &root,
            &root,
            0,
            category,
            &mut candidates,
            &mut budget,
        );
        if budget.exhausted() {
            tracing::warn!(
                path = %path,
                count = candidates.len(),
                visited_dirs = budget.visited_dirs,
                "目录扫描达到时间/目录上限，返回部分结果"
            );
        }
        Self::rank_candidates(candidates)
    }

    fn collect_executable_candidates(
        root: &std::path::Path,
        directory: &std::path::Path,
        depth: u32,
        category: &str,
        entries: &mut Vec<ScanCandidate>,
        budget: &mut ScanBudget,
    ) {
        const MAX_DEPTH: u32 = 10;
        const MAX_ENTRIES: usize = 1000;
        if depth > MAX_DEPTH || entries.len() >= MAX_ENTRIES || budget.exhausted() {
            return;
        }
        budget.visited_dirs += 1;
        let Ok(children) = std::fs::read_dir(directory) else {
            return;
        };
        for child in children.flatten() {
            if entries.len() >= MAX_ENTRIES || budget.exhausted() {
                return;
            }
            let path = child.path();
            if path.is_dir() {
                Self::collect_executable_candidates(
                    root,
                    &path,
                    depth + 1,
                    category,
                    entries,
                    budget,
                );
                continue;
            }
            if !path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            let exe_path = path.to_string_lossy().to_string();
            let entry = AppEntry {
                id: format!("candidate:{}", Self::normalize_path(&exe_path)),
                name: name.to_string(),
                original_name: name.to_string(),
                exe_path,
                icon_base64: None,
                icon_path: None,
                icon_source: None,
                category: category.to_string(),
                source: "Custom".to_string(),
                args: Vec::new(),
                is_favorite: false,
                is_pinned: false,
                launch_mode: None,
            };
            if Self::is_auxiliary_game_executable(&entry) || !path.starts_with(root) {
                continue;
            }
            let size_bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            entries.push(ScanCandidate {
                entry,
                size_bytes,
                // `depth` is already relative to the scan root: the recursion
                // starts at 0 and increments per directory.
                depth,
                recommended: false,
            });
        }
    }

    /// Pick the most plausible executable per directory and sort the result.
    ///
    /// Scanning a "Games" parent folder yields one entry per game folder rather
    /// than every helper binary each game ships, and the recommended set is what
    /// the client pre-selects.
    fn rank_candidates(candidates: Vec<ScanCandidate>) -> Result<Vec<ScanCandidate>, String> {
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, candidate) in candidates.iter().enumerate() {
            let directory = std::path::Path::new(&candidate.entry.exe_path)
                .parent()
                .map(|parent| parent.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            groups.entry(directory).or_default().push(index);
        }

        let mut recommended = vec![false; candidates.len()];
        for (directory, indexes) in &groups {
            let directory_name = std::path::Path::new(directory)
                .file_name()
                .and_then(|name| name.to_str())
                .map(Self::fold_name)
                .unwrap_or_default();
            let best = indexes.iter().copied().max_by_key(|index| {
                let candidate = &candidates[*index];
                let stem = std::path::Path::new(&candidate.entry.exe_path)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .map(Self::fold_name)
                    .unwrap_or_default();
                let name_score = if stem == directory_name {
                    3
                } else if !stem.is_empty() && stem.contains(&directory_name) {
                    2
                } else {
                    0
                };
                // Log-scale-ish ranking: size breaks ties without letting a
                // single 40 GB asset dump outrank a name match.
                (name_score, candidate.size_bytes.min(1 << 32))
            });
            if let Some(index) = best {
                recommended[index] = true;
            }
        }

        let mut ranked: Vec<ScanCandidate> = candidates
            .into_iter()
            .enumerate()
            .map(|(index, mut candidate)| {
                candidate.recommended = recommended[index];
                candidate
            })
            .collect();
        // Recommendations first, then the root directory's own executables,
        // then everything else alphabetically — a stable, reviewable order.
        ranked.sort_by(|left, right| {
            right
                .recommended
                .cmp(&left.recommended)
                .then_with(|| left.depth.cmp(&right.depth))
                .then_with(|| {
                    left.entry
                        .name
                        .to_lowercase()
                        .cmp(&right.entry.name.to_lowercase())
                })
        });
        Ok(ranked)
    }
}
