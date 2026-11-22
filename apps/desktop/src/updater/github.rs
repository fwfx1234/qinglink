//! 从 GitHub Releases 检查桌面端更新。
//!
//! 通道约定：桌面端只认 `desktop-v` 前缀的 tag（例如 `desktop-v0.1.0+29`），这样
//! 移动端 release 不会混进来。tag 去掉前缀后就是完整版本号，交给
//! `qinglink_version::is_newer` 与本地版本比较。
//!
//! 每个 release 带四个 asset：桌面端 `QingLink.exe`、更新助手 `QingLinkUpdater.exe`
//! 及各自的 `.sha256` 校验文件。按精确文件名匹配，避免多平台 release 里取错。

use serde::Deserialize;

use super::http;

/// GitHub Releases API 地址。
const API_URL: &str = "https://api.github.com/repos/fwfx1234/qinglink/releases?per_page=30";

/// 桌面端与更新助手在 release 中的资产名。
pub const DESKTOP_ASSET: &str = "QingLink.exe";
pub const UPDATER_ASSET: &str = "QingLinkUpdater.exe";

/// 一个 asset 的元信息。
#[derive(Clone, Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// GitHub release 响应中我们关心的字段。
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

/// 单个可下载文件及其校验地址。
#[derive(Clone, Debug)]
pub struct DownloadAsset {
    pub name: String,
    pub url: String,
    pub checksum_url: String,
}

/// 检查到的更新结果。
#[derive(Clone, Debug)]
pub struct UpdateInfo {
    /// release tag 去掉前缀后的版本号，例如 `0.1.0+29`。
    pub version: String,
    /// 用户在浏览器打开的 release 页面。
    pub release_url: String,
    /// 更新说明（release body），可能为空。
    pub notes: String,
    /// 需要下载的文件（桌面端 exe + 更新助手 exe）。
    pub assets: Vec<DownloadAsset>,
}

/// 查询 GitHub，判断是否有比本地更新的桌面端 release。
///
/// `current` 是本地完整版本号（来自 `qinglink_version::VERSION`）。
/// 返回 `Ok(None)` 表示已是最新；网络或格式错误返回 `Err`。
pub fn check(current: &str) -> anyhow::Result<Option<UpdateInfo>> {
    let text = http::get_text(API_URL)?;
    let releases: Vec<Release> = serde_json::from_str(&text)
        .map_err(|error| anyhow::anyhow!("解析 GitHub 响应失败：{error}"))?;

    let mut newest: Option<(String, UpdateInfo)> = None;
    for release in releases {
        if release.draft || release.prerelease {
            continue;
        }
        let Some(version) = release.tag_name.strip_prefix("desktop-v") else {
            continue;
        };
        if version.is_empty() || !qinglink_version::is_newer(version, current) {
            continue;
        }
        let Some(info) = build_info(version, &release) else {
            continue;
        };
        let replace = newest
            .as_ref()
            .is_none_or(|(best, _)| qinglink_version::is_newer(version, best));
        if replace {
            newest = Some((version.to_string(), info));
        }
    }

    Ok(newest.map(|(_, info)| info))
}

/// 从 release 中取出两个 exe 及其校验文件；缺任一个就跳过该 release。
fn build_info(version: &str, release: &Release) -> Option<UpdateInfo> {
    let mut assets = Vec::new();
    for name in [DESKTOP_ASSET, UPDATER_ASSET] {
        let file = release.assets.iter().find(|asset| asset.name == name)?;
        let checksum_name = format!("{name}.sha256");
        let checksum = release
            .assets
            .iter()
            .find(|asset| asset.name == checksum_name)?;
        assets.push(DownloadAsset {
            name: name.to_string(),
            url: file.browser_download_url.clone(),
            checksum_url: checksum.browser_download_url.clone(),
        });
    }
    Some(UpdateInfo {
        version: version.to_string(),
        release_url: release.html_url.clone(),
        notes: release.body.clone().unwrap_or_default(),
        assets,
    })
}
