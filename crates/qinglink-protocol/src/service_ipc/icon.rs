//! 图标载荷：单个条目的图标数据、刷新结果与候选文件列表。

use serde::{Deserialize, Serialize};

/// Icon bytes are transported base64-encoded because the transport is JSON.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppIcon {
    /// 条目标识。
    pub id: String,
    /// base64 编码的图标数据，未解析到图标时为 `None`。
    pub icon_base64: Option<String>,
}

/// Result of re-resolving one entry's icon.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppIconUpdate {
    /// 条目标识。
    pub id: String,
    /// base64 编码的图标数据，未解析到图标时为 `None`。
    #[serde(default)]
    pub icon_base64: Option<String>,
    /// Provenance label (`manual` / `steam` / `shortcut` / `executable` /
    /// `image` / `shell` / `none`).
    pub source: String,
    /// 解析所用图标文件的路径，未使用具体文件时为 `None`。
    #[serde(default)]
    pub icon_path: Option<String>,
}

/// One icon file the management UI can offer for an entry.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppIconCandidate {
    /// 图标文件的绝对路径。
    pub path: String,
    /// 候选来源类别（如 `manual` / `steam` / `image`）。
    pub kind: String,
    /// 图标文件大小（字节）。
    pub size_bytes: u64,
}

/// 某个条目可选的图标文件列表。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppIconCandidates {
    /// 条目标识。
    pub id: String,
    /// 候选图标文件。
    pub candidates: Vec<AppIconCandidate>,
}
