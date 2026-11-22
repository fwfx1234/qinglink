//! 下载更新文件、核对 SHA256、放入待安装目录。
//!
//! 完整性来源统一为 release 里的 `SHA256SUMS` 清单：先下载清单并解析，再逐文件下载
//! 并按清单里的期望哈希比对。清单缺失、格式无效、缺条目或哈希不符都直接失败，
//! 绝不使用未通过校验的文件。
//!
//! 进度说明：`Invoke-WebRequest` 直接落盘，无法逐块回报，因此下载阶段给的是
//! 不确定进度（0%），完成后按文件数给出 0-100% 的整体进度。

use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use sha2::{Digest, Sha256};

use super::checksum;
use super::github::{DownloadAsset, UpdateInfo};
use super::http;

/// 下载进度回调：已完成文件数与总文件数。
pub type Progress = dyn Fn(usize, usize) + Send + Sync;

/// 下载所有更新文件并校验，返回待安装目录（内含新桌面端 exe）。
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

    // 先把清单下载到内存并解析；清单是所有文件校验的唯一来源。
    let manifest_text = http::get_text(&info.manifest_url)?;
    let manifest =
        checksum::parse_manifest(&manifest_text).context("解析校验清单失败，已中止下载")?;

    let total = info.assets.len();
    for (index, asset) in info.assets.iter().enumerate() {
        progress(index, total);
        verify_asset(asset, pending_dir, &manifest)?;
    }
    progress(total, total);
    Ok(pending_dir.to_path_buf())
}

/// 下载单个文件并按清单核对其哈希。
fn verify_asset(
    asset: &DownloadAsset,
    dir: &Path,
    manifest: &std::collections::HashMap<String, String>,
) -> anyhow::Result<()> {
    let expected = checksum::lookup(manifest, &asset.name)?;
    let destination = dir.join(&asset.name);
    http::get_to_file(&asset.url, &destination)?;
    let bytes = std::fs::read(&destination)
        .with_context(|| format!("读取下载文件 {}", destination.display()))?;
    if bytes.is_empty() {
        bail!("下载的文件为空：{}", asset.name);
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    ensure!(
        actual.eq_ignore_ascii_case(expected),
        "{} 校验失败：期望 {expected}，实际 {actual}",
        asset.name
    );
    Ok(())
}
