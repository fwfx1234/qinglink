//! 程序更新的管理端点：检查、下载、触发安装、读取状态。
//!
//! 检查与下载在后台线程进行（GitHub 请求与文件下载都可能耗时），立即返回 `accepted`，
//! 结果与进度通过 `system/update_state` WebSocket 事件推送。界面始终以事件里的完整
//! 快照为准，REST 的 `GET /update` 只用于首次进入页面时拉取当前状态。
//!
//! 检查到新版本后自动进入后台下载，下载完成仍停在 `ready` 等待用户手动安装；
//! 安装刻意不自动执行——它会退出主程序并要求 UAC，必须是一次明确的用户操作。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use super::super::error::{ApiError, ok};
use super::super::state::WebState;
use crate::updater::{SharedUpdateState, download, github, github::UpdateInfo, install};

/// 进程启动后自动检查的触发入口。
///
/// 在 Web 服务装配完成后由 bootstrap 调用一次；失败只影响更新状态，不阻断启动。
/// 检查到更新会自动下载，安装仍由用户手动触发。
pub fn auto_check(state: Arc<WebState>) {
    run_check(state);
}

/// `GET /update`：当前更新状态快照。
pub async fn status(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&json!(state.update_state.view()))
}

/// `POST /update/check`：后台检查 GitHub 是否有新版本。
pub async fn check(State(state): State<Arc<WebState>>) -> Json<Value> {
    run_check(state)
}

/// 检查任务本体；手动触发与启动自动触发共用。
fn run_check(state: Arc<WebState>) -> Json<Value> {
    let shared = state.update_state.clone();
    // 防止并发检查/下载：已在忙态时直接返回，界面状态不变。
    if shared.view().busy {
        return accepted();
    }
    shared.begin_check();
    state.publish_update_state();

    std::thread::spawn(move || match github::check(qinglink_version::VERSION) {
        Ok(Some(info)) => {
            let changed = shared.finish_check(info.clone());
            // 每个完成分支都要推送：否则前端会一直停在 checking。
            state.publish_update_state();
            if changed {
                spawn_download(&state, &shared, &info);
            }
        }
        Ok(None) => {
            shared.finish_no_update();
            state.publish_update_state();
        }
        Err(error) => {
            shared.fail(format!("检查更新失败：{error:#}"));
            state.publish_update_state();
        }
    });
    accepted()
}

/// 后台下载并校验；检查发现新版本后自动调用，也供手动重试使用。
fn spawn_download(web: &Arc<WebState>, shared: &SharedUpdateState, info: &UpdateInfo) {
    shared.begin_download();
    web.publish_update_state();

    let shared_for_task = shared.clone();
    let web_for_task = Arc::clone(web);
    let info = info.clone();
    std::thread::spawn(move || {
        let shared_for_progress = shared_for_task.clone();
        let web_for_progress = Arc::clone(&web_for_task);
        let progress = move |done: usize, total: usize| {
            shared_for_progress.set_progress(done as u64, total as u64);
            web_for_progress.publish_update_state();
        };
        let result = (|| -> anyhow::Result<()> {
            let data_dir = qinglink_storage::AppPaths::resolve()?;
            let target = install::pending_dir(data_dir.data_dir(), &info.version);
            download::download_all(&info, &target, &progress)?;
            Ok(())
        })();
        match result {
            Ok(()) => shared_for_task.finish_download(),
            Err(error) => shared_for_task.fail(format!("下载更新失败：{error:#}")),
        }
        web_for_task.publish_update_state();
    });
}

/// `POST /update/download`：手动重试下载已发现的版本。
pub async fn download_update(State(state): State<Arc<WebState>>) -> Json<Value> {
    let shared = state.update_state.clone();
    if shared.view().busy {
        return accepted();
    }
    let Some(info) = shared.discovered() else {
        return accepted();
    };
    spawn_download(&state, &shared, &info);
    accepted()
}

/// `POST /update/install`：启动更新器并请求主程序退出。
///
/// 只有在 `ready` 阶段才真正执行：避免下载未完成或检查失败时误触发。
pub async fn install_update(State(state): State<Arc<WebState>>) -> Result<Json<Value>, ApiError> {
    let shared = state.update_state.clone();
    if shared.view().phase != crate::updater::UpdatePhase::Ready {
        return Err(ApiError::conflict("更新尚未下载完成，无法安装"));
    }
    let info = shared
        .discovered()
        .ok_or_else(|| ApiError::conflict("没有可用的更新"))?;

    let data_dir = qinglink_storage::AppPaths::resolve().map_err(ApiError::io)?;
    let pending = install::extracted_dir(data_dir.data_dir(), &info.version);
    let current = std::env::current_exe().map_err(ApiError::io)?;
    install::launch(&pending, &current).map_err(ApiError::io)?;

    shared.finish_install();
    state.publish_update_state();

    // 给事件一点时间送达浏览器，再让主循环退出（退出序列自身也有 flush 等待）。
    let host = state.host.clone();
    let (reply, _receiver) = tokio::sync::oneshot::channel();
    let _ = host.send(super::super::state::HostCommand::Exit { reply });
    Ok(ok(&json!({ "accepted": true })))
}

fn accepted() -> Json<Value> {
    ok(&json!({ "accepted": true }))
}
