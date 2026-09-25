#![allow(clippy::unwrap_used)]

/// WebSocket 协议端到端测试
///
/// 覆盖认证、API v2 配对和二进制输入通道（12 字节 MouseMoveFrame）。
/// 二进制帧只写入 `input_move_tx` watch 通道，不会真正移动鼠标
/// （输入工作线程仅在 `RemoteServer::run` 中启动，测试直接用路由）。
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use qinglink_server::server::{AppState, RemoteServer};
use qinglink_server::service::RemoteControlService;
use qinglink_storage::AppPaths;
use qinglink_storage::database::{DatabaseService, DatabaseSpec};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("qinglink-ws-e2e-{}", nanos))
}

fn setup_app_state() -> (AppState, std::path::PathBuf) {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let paths = AppPaths::for_test(dir.clone());
    let database = Arc::new(DatabaseService::new(paths.clone()));
    let _ = database.register_database(DatabaseSpec::app("qinglink/data", "qinglink.db"));
    let service = RemoteControlService::new(paths);
    let state = AppState::new(service, database);
    (state, dir)
}

/// 绑定回环端口并托管 axum 路由，返回服务器基础地址。
async fn spawn_server(state: AppState) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = RemoteServer::create_router(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("ws://{addr}")
}

async fn ws_connect(base: &str, path: &str) -> WsStream {
    let (ws, _) = tokio_tungstenite::connect_async(format!("{base}{path}"))
        .await
        .expect("WebSocket 连接失败");
    ws
}

/// 读取下一条文本消息并解析为 JSON（忽略 ping/pong/二进制帧）。
async fn recv_json(ws: &mut WsStream) -> serde_json::Value {
    loop {
        let message = ws
            .next()
            .await
            .expect("WebSocket 意外关闭")
            .expect("WebSocket 接收错误");
        if let Message::Text(text) = message {
            return serde_json::from_str(&text).expect("响应不是合法 JSON");
        }
    }
}

/// 发送带 request_id 的请求并等待匹配的响应。
async fn request(
    ws: &mut WsStream,
    request_type: &str,
    extra: serde_json::Value,
) -> serde_json::Value {
    let id = format!("req-{}", uuid::Uuid::new_v4());
    let mut payload = serde_json::json!({ "type": request_type, "request_id": id });
    if let Some(object) = extra.as_object() {
        for (key, value) in object {
            payload[key] = value.clone();
        }
    }
    ws.send(Message::Text(payload.to_string())).await.unwrap();
    loop {
        let response = recv_json(ws).await;
        if response["request_id"].as_str() == Some(id.as_str()) {
            return response;
        }
    }
}

#[tokio::test]
async fn ws_events_token_auth_then_system_status() {
    let (state, _dir) = setup_app_state();
    let token = state
        .token_store
        .create_permanent_token("ws-e2e-device")
        .unwrap();
    let base = spawn_server(state).await;

    let mut ws = ws_connect(&base, "/api/v2/events").await;

    // 未认证请求应被拒绝
    let unauthorized = request(&mut ws, "get_system_status", serde_json::json!({})).await;
    assert_eq!(unauthorized["type"], "error");
    assert_eq!(unauthorized["data"]["code"], "UNAUTHORIZED");

    // Token 认证
    let auth = request(
        &mut ws,
        "authenticate",
        serde_json::json!({ "method": "token", "token": token }),
    )
    .await;
    assert_eq!(auth["type"], "authenticated");

    // 认证后的业务请求
    let status = request(&mut ws, "get_system_status", serde_json::json!({})).await;
    assert_eq!(status["type"], "system_status");
    assert!(status["data"]["memory_total"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn ws_invalid_token_rejected() {
    let (state, _dir) = setup_app_state();
    let base = spawn_server(state).await;

    let mut ws = ws_connect(&base, "/api/v2/events").await;
    let auth = request(
        &mut ws,
        "authenticate",
        serde_json::json!({ "method": "token", "token": "not-a-real-token" }),
    )
    .await;
    assert_eq!(auth["type"], "error");
    assert_eq!(auth["data"]["code"], "INVALID_TOKEN");
}

#[tokio::test]
async fn ws_binary_input_frame_reaches_move_channel() {
    let (state, _dir) = setup_app_state();
    let token = state
        .token_store
        .create_permanent_token("input-e2e-device")
        .unwrap();
    let mut move_rx = state.input_move_tx.subscribe();
    let base = spawn_server(state.clone()).await;

    let mut ws = ws_connect(&base, "/api/v2/input").await;
    let auth = request(
        &mut ws,
        "authenticate",
        serde_json::json!({ "method": "token", "token": token }),
    )
    .await;
    assert_eq!(auth["type"], "authenticated");

    // 12 字节 MouseMoveFrame：version=1, event=1, seq=0x0001, dx=5, dy=-3, ts=0x1234
    let frame = [
        1u8, 1u8, 0x01, 0x00, 0x05, 0x00, 0xfd, 0xff, 0x34, 0x12, 0x00, 0x00,
    ];
    ws.send(Message::Binary(frame.to_vec())).await.unwrap();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let (dx, dy) = *move_rx.borrow_and_update();
        if (dx, dy) == (5, -3) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等待二进制输入帧写入 input_move_tx 超时"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // 通道仍然存活，且输入通道只允许认证与心跳
    let blocked = request(&mut ws, "get_system_status", serde_json::json!({})).await;
    assert_eq!(blocked["type"], "error");
    assert_eq!(blocked["data"]["code"], "INPUT_REQUEST_NOT_ALLOWED");
}

#[tokio::test]
async fn ws_binary_input_rejected_before_auth() {
    let (state, _dir) = setup_app_state();
    let base = spawn_server(state).await;

    let mut ws = ws_connect(&base, "/api/v2/input").await;
    let frame = [
        1u8, 1u8, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    ws.send(Message::Binary(frame.to_vec())).await.unwrap();

    // 服务端应以 1008 关闭并携带 AUTH_REQUIRED
    let message = ws
        .next()
        .await
        .expect("连接应被服务端关闭")
        .expect("接收错误");
    match message {
        Message::Close(Some(frame)) => {
            assert_eq!(u16::from(frame.code), 1008u16);
            assert_eq!(frame.reason, "AUTH_REQUIRED");
        }
        other => panic!("预期 Close(1008) 帧，实际: {other:?}"),
    }
}

#[tokio::test]
async fn ws_invalid_input_frame_rejected_after_auth() {
    let (state, _dir) = setup_app_state();
    let token = state
        .token_store
        .create_permanent_token("input-bad-frame")
        .unwrap();
    let base = spawn_server(state).await;

    let mut ws = ws_connect(&base, "/api/v2/input").await;
    let auth = request(
        &mut ws,
        "authenticate",
        serde_json::json!({ "method": "token", "token": token }),
    )
    .await;
    assert_eq!(auth["type"], "authenticated");

    // 长度错误 / 版本错误的帧都应导致连接被关闭
    ws.send(Message::Binary(vec![1u8, 1u8, 0x00, 0x00, 0x01]))
        .await
        .unwrap();
    let message = ws
        .next()
        .await
        .expect("连接应被服务端关闭")
        .expect("接收错误");
    match message {
        Message::Close(Some(frame)) => {
            assert_eq!(u16::from(frame.code), 1003u16);
            assert_eq!(frame.reason, "INVALID_INPUT_FRAME");
        }
        other => panic!("预期 Close(1003) 帧，实际: {other:?}"),
    }
}

#[tokio::test]
async fn api_v2_pairing_issues_a_token() {
    let (state, _dir) = setup_app_state();
    let mut notifications = qinglink_server::server::pairing::subscribe(&state);
    let app = RemoteServer::create_router(state);
    let create = app
        .clone()
        .oneshot(
            http::Request::post("/api/v2/pairing/requests")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    r#"{"device_id":"phone-1","device_name":"Test phone"}"#,
                ))
                .expect("create pairing request"),
        )
        .await
        .expect("pairing request response");
    assert_eq!(create.status(), http::StatusCode::OK);
    let notification = notifications.recv().await.expect("pairing notification");

    let confirm_body = serde_json::json!({
        "request_id": notification.request_id,
        "pin": notification.pin,
        "device_id": "phone-1",
        "device_name": "Test phone"
    });
    let confirm = app
        .oneshot(
            http::Request::post("/api/v2/pairing/confirm")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(confirm_body.to_string()))
                .expect("confirm pairing request"),
        )
        .await
        .expect("pairing confirmation response");
    assert_eq!(confirm.status(), http::StatusCode::OK);
    let bytes = confirm
        .into_body()
        .collect()
        .await
        .expect("confirmation body")
        .to_bytes();
    let response: serde_json::Value = serde_json::from_slice(&bytes).expect("confirmation JSON");
    assert!(
        response["token"]
            .as_str()
            .is_some_and(|token| token.len() >= 16)
    );
    assert!(response["pc_id"].as_str().is_some_and(|id| !id.is_empty()));
}

#[tokio::test]
async fn api_v2_pairing_can_be_cancelled_before_confirmation() {
    let (state, _dir) = setup_app_state();
    let app = RemoteServer::create_router(state);
    let create = app
        .clone()
        .oneshot(
            http::Request::post("/api/v2/pairing/requests")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    r#"{"device_id":"phone-cancel","device_name":"Cancel phone"}"#,
                ))
                .expect("create pairing request"),
        )
        .await
        .expect("pairing request response");
    let body = create
        .into_body()
        .collect()
        .await
        .expect("request body")
        .to_bytes();
    let request_id = serde_json::from_slice::<serde_json::Value>(&body)
        .expect("request JSON")["data"]["request_id"]
        .as_str()
        .expect("request id")
        .to_owned();

    let cancel = app
        .clone()
        .oneshot(
            http::Request::delete(format!("/api/v2/pairing/requests/{request_id}"))
                .body(axum::body::Body::empty())
                .expect("cancel pairing request"),
        )
        .await
        .expect("cancel response");
    assert_eq!(cancel.status(), http::StatusCode::OK);

    let confirm = app
        .oneshot(
            http::Request::post("/api/v2/pairing/confirm")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(format!(
                    r#"{{"request_id":"{request_id}","pin":"000000","device_id":"phone-cancel","device_name":"Cancel phone"}}"#
                )))
                .expect("confirm cancelled request"),
        )
        .await
        .expect("confirmation response");
    assert_eq!(confirm.status(), http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ws_process_list_cache_serves_identical_second_page() {
    let (state, _dir) = setup_app_state();
    let token = state
        .token_store
        .create_permanent_token("process-e2e")
        .unwrap();
    let base = spawn_server(state).await;

    let mut ws = ws_connect(&base, "/api/v2/events").await;
    let auth = request(
        &mut ws,
        "authenticate",
        serde_json::json!({ "method": "token", "token": token }),
    )
    .await;
    assert_eq!(auth["type"], "authenticated");

    let first = request(
        &mut ws,
        "get_processes",
        serde_json::json!({ "page": 0, "page_size": 30, "filter": "all", "sort": "active" }),
    )
    .await;
    assert_eq!(first["type"], "process_list");
    let first_total = first["data"]["total"].as_u64().unwrap();

    // 1 秒 TTL 内第二次请求应命中缓存，total 完全一致
    let second = request(
        &mut ws,
        "get_processes",
        serde_json::json!({ "page": 0, "page_size": 30, "filter": "all", "sort": "active" }),
    )
    .await;
    assert_eq!(second["type"], "process_list");
    assert_eq!(
        second["data"]["total"].as_u64().unwrap(),
        first_total,
        "缓存窗口内进程总数应完全一致"
    );
}
