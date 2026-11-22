//! 下载更新文件、核对 SHA256、放入待安装目录。
//!
//! 校验文件是唯一的完整性来源：内容形如 `<64位十六进制>  <文件名>`，取第一段与
//! 下载文件的 SHA256 比较。缺校验文件或哈希不符都直接失败，绝不使用来路不明的文件。
//!
//! 进度说明：`Invoke-WebRequest` 直接落盘，无法逐块回报，因此下载阶段给的是
//! 不确定进度（0%），完成后按文件数给出 0-100% 的整体进度。

use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use sha2::{Digest, Sha256};

use super::github::{DownloadAsset, UpdateInfo};
use super::http;

/// 下载进度回调：已完成文件数与总文件数。
pub type Progress = dyn Fn(usize, usize) + Send + Sync;

/// 下载所有更新文件并校验，返回待安装目录（内含两个 exe）。
pub fn download_all(
    info: &UpdateInfo,
    pending_dir: &Path,
    progress: &Progress,
) -> anyhow::Result<PathBuf> {
    if pending_dir.exists() {
        std::fs::remove_dir_all(pending_dir)
            .with_context(|| format!("清理旧更新目录 {}", pending_dir.display()))?;
    }
    std::fs::create_dir_all(pending_dir)
        .with_context(|| format!("创建更新目录 {}", pending_dir.display()))?;

    let total = info.assets.len();
    for (index, asset) in info.assets.iter().enumerate() {
        progress(index, total);
        verify_asset(asset, pending_dir)?;
    }
    progress(total, total);
    Ok(pending_dir.to_path_buf())
}

/// 下载单个文件并核对其校验文件。
fn verify_asset(asset: &DownloadAsset, dir: &Path) -> anyhow::Result<()> {
    let destination = dir.join(&asset.name);
    let checksum_path = dir.join(format!("{}.sha256", asset.name));

    http::get_to_file(&asset.checksum_url, &checksum_path)?;
    let expected = read_checksum(&checksum_path)?;

    http::get_to_file(&asset.url, &destination)?;
    let bytes = std::fs::read(&destination)
        .with_context(|| format!("读取下载文件 {}", destination.display()))?;
    if bytes.is_empty() {
        bail!("下载的文件为空：{}", asset.name);
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    ensure!(
        actual.eq_ignore_ascii_case(&expected),
        "{} 校验失败：期望 {expected}，实际 {actual}",
        asset.name
    );
    Ok(())
}

/// 读取校验文件的第一行哈希。
fn read_checksum(path: &Path) -> anyhow::Result<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let hash = text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    ensure!(
        hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()),
        "校验文件格式无效：{}（内容：{text}）",
        path.display()
    );
    Ok(hash)
}
