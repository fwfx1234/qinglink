//! 管理界面的事件 socket。
//!
//! 一个 socket 承载所有通道。这样浏览器只需一条连接，也便于服务端施加按通道的
//! 策略（只有被请求时，会话才订阅 `logs`，因为日志量没有上界）。

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast::error::RecvError;

use super::state::{ServerEvent, WebState};

/// Channels a fresh connection receives.
const DEFAULT_CHANNELS: &[&str] = &["system", "service", "devices", "pairing", "apps", "notice"];

/// Every channel the client may ask for.
const ALL_CHANNELS: &[&str] = &[
    "system", "service", "devices", "pairing", "apps", "notice", "logs",
];

/// A message from the browser.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ClientAction {
    /// Add channels to the subscription set.
    Subscribe {
        #[serde(default)]
        channels: Vec<String>,
    },
    /// Remove channels from the subscription set.
    Unsubscribe {
        #[serde(default)]
        channels: Vec<String>,
    },
    /// Liveness probe.
    Ping,
}

/// One step of the socket loop.
///
/// `Resync` exists because state frames are change-only: a socket that fell
/// behind cannot be repaired by reporting the gap, it has to be re-seeded.
enum Outgoing {
    /// A single frame.
    One(Value),
    /// Re-send the current state, then report how many frames were missed.
    Resync { dropped: u64 },
}

type SocketSink = SplitSink<WebSocket, Message>;

/// Upgrade an HTTP request into the event socket.
pub async fn handler(upgrade: WebSocketUpgrade, State(state): State<Arc<WebState>>) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, state))
        .into_response()
}

async fn serve(socket: WebSocket, state: Arc<WebState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut events = state.subscribe();
    let mut channels: BTreeSet<String> = DEFAULT_CHANNELS.iter().map(|c| c.to_string()).collect();

    // Seed the client with the current state so a freshly opened page is correct
    // without waiting for the next change. `hello` carries the subscribed
    // channel list; the rest mirrors what the event stream would have sent.
    let hello = envelope(
        "system",
        "hello",
        &json!({
            "server_time_unix": now_seconds(),
            "channels": channels.iter().cloned().collect::<Vec<_>>(),
        }),
    );
    if !send_frames(&mut sender, std::iter::once(hello)).await {
        return;
    }
    if !send_frames(&mut sender, state_frames(&state, &channels).into_iter()).await {
        return;
    }

    loop {
        // Resolve exactly one outbound step per iteration, then send it with
        // no other borrow alive. Sending inside a `select!` arm would hold the
        // socket borrow across the other branch.
        let outgoing = tokio::select! {
            incoming = receiver.next() => match incoming {
                Some(Ok(Message::Text(text))) => handle_client_text(&text, &mut channels).map(Outgoing::One),
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                // Binary and control frames are not part of this protocol.
                Some(Ok(_)) => None,
            },
            event = events.recv() => match event {
                Ok(event) => render(event, &channels).map(Outgoing::One),
                // The subscriber fell behind; `Lagged` names the gap. The missed
                // state frames are re-sent below, because a change-only stream
                // cannot repair itself.
                Err(RecvError::Lagged(count)) => Some(Outgoing::Resync { dropped: count }),
                Err(RecvError::Closed) => break,
            },
        };

        match outgoing {
            None => {}
            Some(Outgoing::One(frame)) => {
                if sender.send(Message::Text(frame.to_string())).await.is_err() {
                    break;
                }
            }
            Some(Outgoing::Resync { dropped }) => {
                if !send_frames(&mut sender, state_frames(&state, &channels).into_iter()).await {
                    break;
                }
                if channels.contains("logs") {
                    let notice = envelope("logs", "dropped", &json!({ "count": dropped }));
                    if sender
                        .send(Message::Text(notice.to_string()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    }

    // Complete the closing handshake so clients see a clean shutdown instead of
    // a dropped connection. Best effort: the peer may already be gone.
    let _ = sender.send(Message::Close(None)).await;
}

/// Send a sequence of already-rendered frames. `false` means the peer is gone.
async fn send_frames(sender: &mut SocketSink, frames: impl Iterator<Item = Value>) -> bool {
    for frame in frames {
        if sender.send(Message::Text(frame.to_string())).await.is_err() {
            return false;
        }
    }
    true
}

/// The current state of everything the browser cannot poll for.
///
/// Sent right after `hello` and again whenever this socket falls behind. Frames
/// whose channel is not subscribed are dropped, so a logs-only or
/// state-less subscription is not spammed.
fn state_frames(state: &Arc<WebState>, channels: &BTreeSet<String>) -> Vec<Value> {
    let mut frames = vec![
        ("service", "status", json!(state.service.status())),
        ("service", "install_state", json!(state.install_view())),
        (
            "pairing",
            "pending",
            json!({ "pairing": state.service.pending_pairing() }),
        ),
        // Tells every list that re-reads on this event to fetch again.
        ("apps", "catalog_changed", json!({ "reason": "resync" })),
    ];
    if let Some(total) = state.service.device_count() {
        frames.push(("devices", "changed", json!({ "total": total })));
    }
    frames
        .into_iter()
        .filter(|(channel, _, _)| channels.contains(*channel))
        .map(|(channel, event, data)| envelope(channel, event, &data))
        .collect()
}

/// Apply a client action, returning an immediate reply when one is warranted.
fn handle_client_text(text: &str, channels: &mut BTreeSet<String>) -> Option<Value> {
    let action = match serde_json::from_str::<ClientAction>(text) {
        Ok(action) => action,
        Err(error) => {
            tracing::debug!(%error, "ignoring an unparsable admin socket message");
            return None;
        }
    };
    match action {
        ClientAction::Subscribe { channels: wanted } => {
            for channel in wanted {
                if ALL_CHANNELS.contains(&channel.as_str()) {
                    channels.insert(channel);
                }
            }
            None
        }
        ClientAction::Unsubscribe { channels: wanted } => {
            for channel in wanted {
                channels.remove(&channel);
            }
            None
        }
        ClientAction::Ping => Some(envelope(
            "system",
            "pong",
            &json!({ "server_time_unix": now_seconds() }),
        )),
    }
}

/// Project an internal event onto the documented wire envelope.
fn render(event: ServerEvent, channels: &BTreeSet<String>) -> Option<Value> {
    let (channel, name, data) = match event {
        ServerEvent::Service(status) => ("service", "status", json!(status)),
        ServerEvent::Pairing(pairing) => ("pairing", "pending", json!({ "pairing": *pairing })),
        ServerEvent::InstallState(install) => ("service", "install_state", json!(install)),
        ServerEvent::Logs(entries) => ("logs", "entries", json!({ "items": entries.as_ref() })),
        ServerEvent::LogsDropped(count) => ("logs", "dropped", json!({ "count": count })),
        ServerEvent::CatalogChanged(reason) => {
            ("apps", "catalog_changed", json!({ "reason": reason }))
        }
        ServerEvent::ScanStarted(kind) => ("apps", "scan_started", json!({ "kind": kind })),
        ServerEvent::Devices { total } => ("devices", "changed", json!({ "total": total })),
        ServerEvent::Settings(settings) => ("system", "settings", json!(*settings)),
        ServerEvent::LogsCleared { cleared } => ("logs", "cleared", json!({ "cleared": cleared })),
        ServerEvent::Toast { level, message } => (
            "notice",
            "toast",
            json!({ "level": level, "message": message }),
        ),
        ServerEvent::UpdateState(state) => ("system", "update_state", json!(state)),
        ServerEvent::Shutdown(reason) => ("system", "shutdown", json!({ "reason": reason })),
    };
    if !channels.contains(channel) {
        return None;
    }
    Some(envelope(channel, name, &data))
}

fn envelope(channel: &str, event: &str, data: &Value) -> Value {
    json!({ "channel": channel, "event": event, "data": data })
}

fn now_seconds() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
