//! 原生的"在这台 PC 上选文件"对话框。
//!
//! 界面跑在浏览器里，看不到主机的文件系统：`<input type="file">` 只会交回一个
//! 不透明的 blob，而不是目录要存的绝对路径。桌面进程持有用户会话，因此按需弹出
//! 真正的 shell 选择器并返回该路径。

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use serde_json::json;

use qinglink_windows::platform_windows::{FileDialogKind, pick_file};

use super::super::error::{ApiError, ok};
use super::super::state::WebState;

/// Request body of `POST /dialog/file`.
#[derive(Debug, Deserialize)]
pub struct FileDialogRequest {
    /// What the caller is looking for: `image` or `executable`.
    kind: String,
    /// Directory the picker opens in, when the caller already knows one.
    #[serde(default)]
    start_in: Option<String>,
}

/// `POST /dialog/file`
///
/// Returns `{"path": "C:\\…"}` with `path: null` when the user cancelled.
pub async fn pick(
    State(state): State<Arc<WebState>>,
    Json(request): Json<FileDialogRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let kind = FileDialogKind::parse(&request.kind)
        .ok_or_else(|| ApiError::bad_request("kind 只能是 image 或 executable"))?;
    // One picker at a time: a second request would stack a modal window on top
    // of the one already waiting for an answer, and neither would be reachable.
    if state.file_picker_open.swap(true, Ordering::AcqRel) {
        return Err(ApiError::conflict("已经有一个文件选择窗口在等待"));
    }
    let start_in = request.start_in.filter(|path| !path.trim().is_empty());
    let result = run_picker(kind, start_in).await;
    state.file_picker_open.store(false, Ordering::Release);
    let path = result?;
    Ok(ok(&json!({ "path": path })))
}

/// Run the blocking picker on a thread of its own.
///
/// Not `spawn_blocking`: the shell picker is a single-threaded apartment
/// component, and a pooled thread may already have been initialized as MTA by
/// unrelated work (icon extraction does exactly that). A dedicated thread always
/// starts clean.
async fn run_picker(
    kind: FileDialogKind,
    start_in: Option<String>,
) -> Result<Option<String>, ApiError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("qinglink-file-picker".to_string())
        .spawn(move || {
            let start_in = start_in.as_deref().map(Path::new);
            let _ = sender.send(pick_file(kind, start_in));
        })
        .map_err(|error| ApiError::internal(format!("无法启动文件选择线程：{error}")))?;

    receiver
        .await
        .map_err(|_| ApiError::internal("文件选择线程异常退出"))?
        .map_err(|error| ApiError::internal(error.to_string()))
}
