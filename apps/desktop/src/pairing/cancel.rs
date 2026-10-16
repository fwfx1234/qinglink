//! 把弹窗上的「取消配对」送到服务，并在后台线程里等答复。
//!
//! 主线程持有 Windows 消息泵，而一次 IPC 往返最长 5 秒；在主线程上等会把托盘和弹窗
//! 一起冻住。因此这里起一条专用线程：线程只负责调用与回传结论，界面更新仍旧只在主
//! 线程上发生（结论经事件通道回到主循环）。

use qinglink_protocol::service_ipc::DesktopCommand;
use tokio::sync::mpsc::UnboundedSender;

use super::PromptEvent;
use crate::service::{CALL_TIMEOUT, ServiceClient};

/// 起一个后台线程请求服务取消配对。
///
/// # Errors
/// 线程创建失败时返回错误，调用方应当把按钮还给用户。
pub(crate) fn spawn(
    client: ServiceClient,
    sender: UnboundedSender<PromptEvent>,
    request_id: String,
) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("qinglink-pairing-cancel".to_string())
        .spawn(move || {
            let outcome = client
                .call_blocking(
                    DesktopCommand::CancelPairing {
                        pairing_id: request_id.clone(),
                    },
                    CALL_TIMEOUT,
                )
                .map(|_| ())
                .map_err(|error| error.to_string());
            let _ = sender.send(PromptEvent::CancelFinished {
                request_id,
                outcome,
            });
        })
        .map(|_| ())
}
