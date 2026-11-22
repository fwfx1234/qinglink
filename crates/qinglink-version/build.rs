//! 生成统一的产品版本号，并在源码改动时自动递增构建号。
//!
//! 版本号由两部分拼成：`Cargo.toml` 中手工维护的语义版本（`0.1.0`）与本脚本自增
//! 的构建号，合起来是 `0.1.0+13`。构建号只在被跟踪的源码**指纹变化**时加一，因此
//! 同一份源码重复构建得到同一个版本号，改一行代码再构建才会变大。
//!
//! 状态写在同目录的 `build-number.txt`（第一行构建号，第二行上次的源码指纹），随
//! 仓库一起提交。它刻意不被 `cargo:rerun-if-changed` 覆盖：否则每次自增都会让
//! cargo 认为输入变了，进而每次构建都重跑本脚本并重编译所有依赖本 crate 的程序。
//!
//! CI 发布构建是唯一例外：流水线从 Release tag 解析出构建号，经环境变量
//! `QINGLINK_CI_BUILD_NUMBER` 传入后直接用它，跳过指纹计算与状态写回。原因是 CI
//! checkout 出来的文件修改时间全部重置，照常算指纹必然与仓库里存的对不上而自增，
//! 导致产物版本与 tag 不一致。

use std::{
    env, fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use sha2::{Digest, Sha256};

/// 参与源码指纹的目录（相对仓库根）。
///
/// `crates` 不在此列：本 crate 的状态文件就在那棵树下，监视整棵目录会让自增反过来
/// 触发下一次重编译。各 crate 的 `src` 与 `Cargo.toml` 由 [`watched_paths`] 逐个补上。
const TRACKED_DIRS: &[&str] = &[
    "apps/desktop/assets",
    "apps/desktop/src",
    "apps/desktop/web-ui/src",
    "crates/qinglink-version/src",
    "protocol",
    "services/qinglink-service/src",
];

/// 参与源码指纹的单文件（相对仓库根）。
const TRACKED_FILES: &[&str] = &[
    "Cargo.toml",
    "apps/desktop/qinglink.exe.manifest",
    "apps/desktop/qinglink.rc",
];

/// 不参与指纹的目录名：依赖缓存与构建产物。
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "target",
    ".git",
    ".dart_tool",
    "build",
];

/// 不参与指纹的路径前缀。
///
/// `assets/input/` 里是安全输入服务的二进制——它由 `apps/desktop/build.rs` 内嵌，本身是
/// 另一次构建的产物。算进指纹会让"刷新服务资源"反过来给桌面端升一个号，而新内嵌的服务
/// 又永远慢一个号，两边对不齐。
const SKIPPED_PATHS: &[&str] = &["apps/desktop/assets/input"];

/// 本 crate 的相对路径；它的状态文件不属于产品源码。
const SELF_DIR: &str = "crates/qinglink-version";

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo_root = manifest_dir.join("../..");

    // 环境变量变化必须触发重跑：否则同一 CI 里传不同构建号不会更新产物版本。
    println!("cargo:rerun-if-env-changed=QINGLINK_CI_BUILD_NUMBER");

    for path in watched_paths(&repo_root) {
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let base = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());

    // CI 发布构建：tag 是版本号的唯一真相源，直接用注入的构建号，不写回状态文件。
    if let Some(build) = ci_build_number() {
        println!("cargo:rustc-env=QINGLINK_VERSION={base}+{build}");
        println!("cargo:rustc-env=QINGLINK_BUILD_NUMBER={build}");
        return;
    }

    let state_path = manifest_dir.join("build-number.txt");
    let fingerprint = fingerprint(&repo_root);
    let (stored, previous) = read_state(&state_path);
    let build = next_build_number(stored, previous.as_deref(), &fingerprint);
    if stored != build || previous.as_deref() != Some(fingerprint.as_str()) {
        write_state(&state_path, build, &fingerprint);
    }

    println!("cargo:rustc-env=QINGLINK_VERSION={base}+{build}");
    println!("cargo:rustc-env=QINGLINK_BUILD_NUMBER={build}");
}

/// 读取 CI 从 Release tag 注入的构建号。
///
/// 本地开发不设这个变量，返回 `None`，继续走源码指纹自增。只有正整数有效。
fn ci_build_number() -> Option<u32> {
    env::var("QINGLINK_CI_BUILD_NUMBER")
        .ok()
        .and_then(|raw| raw.trim().parse().ok())
        .filter(|number| *number > 0)
}

/// 需要监视的目录与文件。
fn watched_paths(repo_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(repo_root.join("crates")) {
        let mut crates: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter(|path| relative(path, repo_root) != SELF_DIR)
            .collect();
        crates.sort();
        for directory in crates {
            paths.push(directory.join("src"));
            paths.push(directory.join("Cargo.toml"));
        }
    }
    for directory in TRACKED_DIRS {
        paths.push(repo_root.join(directory));
    }
    for file in TRACKED_FILES {
        paths.push(repo_root.join(file));
    }
    paths
}

/// 由路径、大小与修改时间算出的源码指纹。
///
/// 只看元数据而不读内容：仓库里有几万个源文件，逐字节哈希会明显拖慢构建，而
/// "内容改了"必然伴随修改时间变化。
fn fingerprint(repo_root: &Path) -> String {
    let mut entries: Vec<(String, u64, u128)> = Vec::new();
    for path in watched_paths(repo_root) {
        if path.is_dir() {
            collect_files(&path, repo_root, &mut entries);
        } else if let Some(entry) = file_entry(&path, repo_root) {
            entries.push(entry);
        }
    }
    if entries.is_empty() {
        return String::new();
    }
    entries.sort();
    let mut hasher = Sha256::new();
    for (path, size, modified) in &entries {
        hasher.update(path.as_bytes());
        hasher.update(size.to_le_bytes());
        hasher.update(modified.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// 递归收集目录下的普通文件条目。
fn collect_files(directory: &Path, repo_root: &Path, output: &mut Vec<(String, u64, u128)>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if SKIPPED_DIRS.contains(&entry.file_name().to_string_lossy().as_ref()) {
            continue;
        }
        let relative_path = relative(&path, repo_root);
        if SKIPPED_PATHS
            .iter()
            .any(|skipped| relative_path.starts_with(skipped))
        {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, repo_root, output);
        } else if let Some(file) = file_entry(&path, repo_root) {
            output.push(file);
        }
    }
}

/// 单个文件的指纹条目：相对路径、字节数与修改时间（纳秒）。
fn file_entry(path: &Path, repo_root: &Path) -> Option<(String, u64, u128)> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((relative(path, repo_root), metadata.len(), modified))
}

/// 统一的相对路径写法，保证 Windows 与 Unix 下指纹一致。
fn relative(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// 读取状态文件：构建号与上次的源码指纹（指纹可能缺失）。
fn read_state(path: &Path) -> (u32, Option<String>) {
    let Ok(text) = fs::read_to_string(path) else {
        return (0, None);
    };
    let mut lines = text.lines();
    let stored = lines
        .next()
        .and_then(|line| line.trim().parse().ok())
        .unwrap_or(0);
    let fingerprint = lines
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string);
    (stored, fingerprint)
}

/// 写回状态文件；仅在内容变化时落盘，避免无谓地更新修改时间。
fn write_state(path: &Path, build: u32, fingerprint: &str) {
    let text = format!("{build}\n{fingerprint}\n");
    if fs::read_to_string(path).is_ok_and(|current| current == text) {
        return;
    }
    fs::write(path, text).expect("write build-number.txt");
}

/// 新构建号：源码指纹变了就加一，没变就沿用。
fn next_build_number(stored: u32, previous: Option<&str>, fingerprint: &str) -> u32 {
    if fingerprint.is_empty() {
        // 遍历失败时不递增：宁可沿用旧号，也不要让版本号凭构建次数上涨。
        return stored.max(1);
    }
    match previous {
        Some(previous) if previous == fingerprint => stored.max(1),
        _ => stored.saturating_add(1),
    }
}
