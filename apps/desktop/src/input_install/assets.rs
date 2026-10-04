use std::{fs, path::Path};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

pub(crate) struct EmbeddedAsset {
    pub(crate) relative_path: &'static str,
    pub(crate) bytes: &'static [u8],
    pub(crate) sha256: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/embedded_input_assets.rs"));

pub(super) fn packaged_assets() -> &'static [EmbeddedAsset] {
    INPUT_ASSETS
}

pub(crate) fn extract(directory: &Path) -> Result<()> {
    fs::create_dir_all(directory).context("无法创建资源暂存目录")?;
    for asset in INPUT_ASSETS {
        let target = directory.join(asset.relative_path);
        ensure!(target.starts_with(directory), "资源路径越界");
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, asset.bytes)
            .with_context(|| format!("无法释放 {}", asset.relative_path))?;
        verify_file_hash(&target, asset.sha256)?;
    }
    Ok(())
}

pub(crate) fn verify_embedded_assets() -> Result<()> {
    for asset in INPUT_ASSETS {
        let actual = format!("{:x}", Sha256::digest(asset.bytes));
        ensure!(
            actual == asset.sha256,
            "内嵌资源校验失败：{}",
            asset.relative_path
        );
    }
    Ok(())
}

fn verify_file_hash(path: &Path, expected: &str) -> Result<()> {
    let bytes = fs::read(path)?;
    ensure!(
        format!("{:x}", Sha256::digest(bytes)) == expected,
        "文件校验失败：{}",
        path.display()
    );
    Ok(())
}
