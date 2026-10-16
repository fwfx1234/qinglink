//! 服务端下行的 WebSocket 响应词汇：`WsServerResponse` 与 `WsServerMessage`。
//!
//! 自 `requests_ws.rs` 按“请求 / 响应词汇”职责拆出；`crate::protocol::requests` 仍按
//! 历史路径重新导出这两个类型，因此调用方与线格式都无需改动。

use serde::Serialize;

use crate::protocol::requests::{WindowInfo, WsEvent};

// === WebSocket server responses ===

/// 服务端通过 WebSocket 返回的响应，用 `type` 区分类型、`data` 携带负载。
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WsServerResponse {
    // === Auth ===
    /// 认证成功，返回会话令牌及有效期信息。
    Authenticated(crate::protocol::responses::AuthenticatedResponse),
    /// 设备令牌已被成功撤销。
    Revoked,

    // === System ===
    /// 系统总览状态。
    SystemStatus(crate::protocol::responses::SystemStatus),
    /// 锁屏状态与自动解锁能力。
    UnlockStatus(crate::protocol::responses::UnlockStatus),
    /// 安全桌面输入通道状态。
    LockInputStatus(crate::protocol::responses::LockInputStatus),
    /// 提交锁屏 PIN 的执行结果。
    LockInputResult(crate::protocol::responses::LockInputResult),
    /// 安全桌面的详细状态。
    SecureDesktopStatus(crate::protocol::responses::SecureDesktopStatus),
    /// 显示器列表。
    Displays(crate::protocol::responses::DisplaysResponse),
    /// 主网卡的 MAC 地址。
    MacAddress {
        /// 格式化后的 MAC 地址字符串。
        mac_address: String,
    },
    /// 局域网子网前缀长度。
    SubnetPrefix {
        /// 子网前缀长度，取值范围 0 到 32。
        prefix: u8,
    },
    /// 支持网络唤醒的网卡列表。
    WakeNetworks {
        /// 网卡适配器信息列表。
        networks: Vec<qinglink_protocol::WakeNetworkAdapter>,
    },

    // === Processes ===
    /// 进程分页列表。
    ProcessList(crate::protocol::responses::ProcessListResponse),
    /// 单个进程的图标。
    ProcessIcon(crate::protocol::responses::ProcessIconResponse),
    /// 当前前台窗口信息。
    Foreground(crate::protocol::responses::ForegroundResponse),

    // === Apps ===
    /// 应用搜索结果。
    SearchApps(crate::protocol::responses::SearchAppsResponse),

    // === Scanner ===
    /// 应用扫描结果（旧版接口）。
    ScannerApps(Vec<crate::service::app_scanner::ScannerAppEntryResponse>),
    /// 启动器条目的分页结果。
    LauncherItems(crate::protocol::responses::LauncherItemPage),
    /// 最近启动过的启动器条目列表。
    RecentLauncherItems(Vec<crate::protocol::responses::LauncherItemSummary>),
    /// 指定条目的图标数据。
    LauncherIcon(crate::protocol::responses::LauncherIconResponse),
    /// 条目可选的图标文件列表。
    IconCandidates(crate::protocol::responses::IconCandidatesResponse),
    /// 图标解析或更新的结果。
    IconUpdate(crate::protocol::responses::IconUpdateResponse),
    /// 单个图标文件的预览（不落库、不改条目）。
    IconPreview(crate::protocol::responses::IconPreviewResponse),
    /// 被屏蔽的启动器条目列表。
    BlockedLauncherItems(crate::protocol::responses::BlockedLauncherItemsResponse),
    /// 目录扫描得到的候选条目。
    GameScanCandidates(crate::protocol::responses::GameScanCandidatesResponse),
    /// Steam 游戏与库信息。
    SteamGames(crate::protocol::responses::SteamGamesResponse),
    /// Steam 刷新完成，并附带本次扫描到的游戏数量。
    SteamRefresh {
        /// 本次扫描到的游戏数量。
        scanned: usize,
    },

    // === Custom Dirs ===
    /// 自定义扫描目录列表。
    CustomDirs(crate::protocol::responses::CustomDirListResponse),
    /// 单个自定义扫描目录，用于新增或更新后的返回。
    CustomDir(crate::custom_dir::CustomDir),
    /// 目录校验结果。
    DirValidation(crate::custom_dir::DirValidationResult),

    // === Tasks ===
    /// 任务（进程）的分页列表。
    Tasks(crate::protocol::responses::ProcessListResponse),

    // === Volume ===
    /// 音量与静音状态。
    VolumeStatus(crate::protocol::responses::VolumeStatus),
    /// 音频输出设备列表。
    AudioOutputs(crate::protocol::responses::AudioOutputsResponse),

    // === File System ===
    /// 目录浏览结果。
    DirectoryListing(crate::service::file_browser::DirectoryListing),
    /// 快速访问项，每项为名称与路径组成的二元组。
    QuickAccess(Vec<(String, String)>),

    // === Window Control ===
    /// 当前打开的窗口列表。
    WindowList(Vec<WindowInfo>),
    // === Scanner App Management ===
    /// 应用条目已新增或更新，返回其 ID。
    ScannerAppUpdated {
        /// 被修改的条目 ID。
        id: String,
    },
    /// 应用条目已删除，返回其 ID。
    ScannerAppDeleted {
        /// 被删除的条目 ID。
        id: String,
    },

    // === Common ===
    /// 心跳应答。
    Pong,
    /// 通用成功应答。
    Success,
    /// 通用错误应答。
    Error {
        /// 错误码，供客户端分支处理。
        code: String,
        /// 面向用户的错误描述。
        message: String,
    },
}

impl WsServerResponse {
    /// 构造通用成功应答。
    pub fn success() -> Self {
        Self::Success
    }

    /// 按错误码与错误描述构造错误应答。
    pub fn error(code: &str, message: &str) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }
}

// === WebSocket unified message (server → client) ===

/// 服务端发往客户端的 WebSocket 消息，统一封装事件推送与请求响应。
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum WsServerMessage {
    /// 服务端主动推送的事件。
    Event(WsEvent),
    /// 对客户端请求的响应。
    Response(WsServerResponse),
}
