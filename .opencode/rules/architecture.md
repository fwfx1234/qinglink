# 架构规范 — QingLink

基于 [Rust crate 生态系统模式](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html) 和 [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html) 原则。

---

## 1. Crate 依赖图

```
┌─────────────────────────────────────────────────────────────────┐
│                        应用层                                     │
│                                                                  │
│  ┌──────────────────┐    ┌────────────────────────────────────┐ │
│  │  apps/desktop    │    │  services/qinglink-service          │ │
│  │  (GPUI + Axum)   │    │  (Windows Service)                  │ │
│  └────────┬─────────┘    └──────────┬─────────────────────────┘ │
│           │                         │                            │
├───────────┼─────────────────────────┼────────────────────────────┤
│           │       核心层             │                            │
│  ┌────────▼─────────────────────────▼──────────────────────────┐ │
│  │                    crates/qinglink-server                    │ │
│  │              (Axum HTTP/WS, 认证, 服务)                      │ │
│  └────────┬─────────────────────────┬─────────────────────────┘ │
│           │                         │                            │
│  ┌────────▼──────────┐    ┌─────────▼────────────────────────┐  │
│  │ qinglink-storage  │    │ qinglink-windows                  │ │
│  │ (SQLite + repos)  │    │ (audio, input, display, etc.)    │ │
│  └────────┬──────────┘    └─────────┬────────────────────────┘  │
│           │                         │                            │
├───────────┼─────────────────────────┼────────────────────────────┤
│           │    领域层                │                            │
│  ┌────────▼─────────────────────────▼──────────────────────────┐ │
│  │  qinglink-protocol  │  qinglink-domain                     │ │
│  │  (DTOs, schemas)    │  (状态机, 能力协商)                   │ │
│  └─────────────────────┴──────────────────────────────────────┘ │
│                           │                                      │
│                  ┌────────▼────────┐                             │
│                  │ qinglink-       │                             │
│                  │ discovery       │                             │
│                  │ (mDNS)          │                             │
│                  └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

### 依赖规则（严格）

| Crate | 可依赖 | 不可依赖 |
|-------|--------|---------|
| `qinglink-protocol` | （叶子 crate） | 任何内部 crate |
| `qinglink-domain` | `qinglink-protocol` | server, storage, windows |
| `qinglink-storage` | `qinglink-protocol` | server, domain, windows |
| `qinglink-discovery` | `qinglink-protocol` | server, storage, domain |
| `qinglink-windows` | `qinglink-protocol`, `qinglink-domain` | server, storage |
| `qinglink-server` | 其下所有 crate | apps/*, services/* |
| `apps/desktop` | `qinglink-server`, `qinglink-storage` | `qinglink-service`, 直接访问 `qinglink-windows` |
| `services/qinglink-service` | `qinglink-protocol`, `qinglink-windows` | `qinglink-server`, `qinglink-storage` |

**强制：** 禁止循环依赖。如果 crate A 依赖 crate B，B 绝不能依赖 A（即使是传递依赖）。

---

## 2. 模块职责

### 2.1 `qinglink-protocol` — Wire 格式

**目的：** 所有跨语言数据类型的单一事实来源。

**包含：**
- 请求/响应消息结构体
- 配对协议 DTO
- 健康检查消息类型
- 平台能力枚举
- `protocol/qinglink-v2.schema.json` 中的 JSON Schema

**规则：**
- 所有类型：`#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`
- JSON 字段名必须与 `qinglink-v2.schema.json` 完全匹配
- 禁止业务逻辑、I/O、状态、平台特定代码
- 禁止条件编译（`#[cfg(...)]`）
- Schema 变更需要版本升级

```rust
/// 移动端到桌面端的鼠标移动命令。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MouseMoveRequest {
    pub request_id: String,
    pub dx: i32,
    pub dy: i32,
}
```

### 2.2 `qinglink-domain` — 领域逻辑

**目的：** 无外部依赖的纯领域逻辑。

**包含：**
- 配对状态机（`Pending → Confirmed → Active → Revoked`）
- 能力协商类型
- 领域错误类型
- 验证规则（token 格式、PIN 格式）

**规则：**
- 禁止数据库访问、网络 I/O、平台 API
- 仅纯函数和确定性状态转换
- 无需任何外部设置即可测试

```rust
/// 配对状态转换。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingState {
    Pending { pin: String, expires_at: Instant },
    Confirmed { token: String },
    Active { device_id: DeviceId, last_seen: Instant },
    Revoked,
}

impl PairingState {
    /// 尝试用提供的 PIN 确认配对。
    pub fn confirm(self, pin: &str) -> Result<Self, PairingError> {
        match self {
            Self::Pending { pin: expected, expires_at } => {
                if Instant::now() > expires_at {
                    Err(PairingError::Expired)
                } else if pin != expected {
                    Err(PairingError::InvalidPin)
                } else {
                    Ok(Self::Confirmed { token: generate_token() })
                }
            }
            _ => Err(PairingError::InvalidState),
        }
    }
}
```

### 2.3 `qinglink-storage` — 持久化

**目的：** SQLite 数据访问，支持迁移和连接池。

**包含：**
- 数据库连接管理（r2d2 连接池）
- Schema 迁移（仅追加）
- 仓库模式实现
- 设置/应用/设备数据模型

**规则：**
- 所有查询必须使用参数化语句（禁止字符串拼接）
- 迁移仅追加（禁止修改已有迁移文件）
- 连接池可通过 `max_connections` 配置
- 返回领域类型，不返回原始 SQL 行

```rust
#[async_trait]
pub trait AppRepository: Send + Sync {
    async fn get_visible(&self) -> Result<Vec<AppInfo>, StorageError>;
    async fn upsert(&self, app: &AppInfo) -> Result<(), StorageError>;
    async fn delete(&self, id: &str) -> Result<bool, StorageError>;
}
```

### 2.4 `qinglink-server` — HTTP/WS 服务器

**目的：** 基于 Axum 的 HTTP 和 WebSocket 服务器。

**包含：**
- 路由处理器（`/api/v2/*`）
- WebSocket 生命周期管理
- 认证中间件
- 请求分发到服务层
- 连接注册表

**规则：**
- Handler 要薄 — 5 行内委托给服务层
- 禁止直接数据库查询（使用 repository）
- 每个 WS handler 必须有超时
- 错误响应遵循 protocol 中的 `ErrorResponse` 格式
- 所有中间件使用 `tower::Service` 模式

```rust
pub fn router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/api/v2/pairing", post(handle_pairing))
        .route("/api/v2/events", get(ws_handler))
        .layer(Extension(state))
        .layer(middleware::from_fn(auth_middleware))
}
```

### 2.5 `qinglink-windows` — 平台层

**目的：** Windows 特定的 OS 集成（WinAPI 的安全包装器）。

**包含：**
- 音频控制（音量、静音）
- 显示管理（亮度、分辨率）
- 输入注入（键盘、鼠标 via SendInput）
- 进程管理（列表、终止、启动）
- 防火墙规则（添加/删除应用例外）
- 电源管理（关机、重启、睡眠）
- 安全桌面输入（会话切换）
- 窗口管理（查找、聚焦、关闭）

**规则：**
- 所有 FFI 调用封装在安全包装器中
- Windows 错误转换为 `thiserror` 类型
- `HANDLE` 用 RAII + `Drop` 包装
- 安全包装器在 `unsafe` 调用前必须验证输入

### 2.6 `qinglink-discovery` — 服务发现

**目的：** mDNS 服务广播和浏览。

**规则：**
- 服务类型：`_qinglink._tcp.local.`
- TXT 记录包含：version、platform、capabilities

---

## 3. WebSocket 协议契约

### 3.1 消息信封

每个 WebSocket 消息必须遵循此信封格式：

```json
{
  "type": "message_type",
  "request_id": "uuid-v4",
  "payload": { ... }
}
```

| 字段 | 必填 | 类型 | 说明 |
|------|------|------|------|
| `type` | 是 | String | 消息类型标识 |
| `request_id` | 是（请求） | String | 客户端生成的 UUID，响应中回显 |
| `payload` | 否 | Object | 消息特定数据 |

### 3.2 请求/响应关联

```
客户端                              服务端
  │                                    │
  │─── { type: "mouse_move",           │
  │      request_id: "req-42",         │
  │      payload: { dx: 10, dy: -5 } } │
  │                                    │
  │◄── { type: "ack",                  │
  │      request_id: "req-42" }        │
  │                                    │
```

**规则：**
- 客户端生成 `request_id`（UUID v4）
- 服务端必须在所有响应中回显 `request_id`
- 客户端跟踪待处理请求并设置超时（默认 10 秒）
- 超时无响应则重试或报告失败

### 3.3 错误响应格式

所有错误响应必须使用此结构：

```rust
#[derive(Serialize)]
pub struct ErrorResponse {
    #[serde(rename = "type")]
    pub type_: &'static str,    // 始终为 "error"
    pub request_id: String,
    pub ok: bool,               // 始终为 false
    pub code: String,           // 机器可读的错误码
    pub message: String,        // 人类可读的描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
```

### 3.4 错误码注册表

| 错误码 | HTTP 状态 | 含义 | 客户端行为 |
|--------|----------|------|-----------|
| `INVALID_TOKEN` | 401 | Token 缺失或格式错误 | 重新认证 |
| `TOKEN_EXPIRED` | 401 | Token 超过 TTL | 刷新 token |
| `PAIRING_REQUIRED` | 403 | 无活动配对 | 启动配对流程 |
| `RATE_LIMITED` | 429 | 请求过多 | 退避重试 |
| `INVALID_PARAMS` | 400 | 请求验证失败 | 修正请求 |
| `NOT_FOUND` | 404 | 资源未找到 | 检查标识符 |
| `INTERNAL_ERROR` | 500 | 意外服务端错误 | 报告/记录 |

### 3.5 消息类型注册表

| 类型 | 方向 | 说明 |
|------|------|------|
| `pairing_request` | C→S | 请求配对（带 PIN） |
| `pairing_confirm` | S→C | 配对已接受 |
| `pairing_reject` | S→C | 配对被拒绝 |
| `health_ping` | C→S | 客户端健康检查 |
| `health_pong` | S→C | 服务端健康响应 |
| `mouse_move` | C→S | 鼠标移动增量 |
| `mouse_click` | C→S | 鼠标按键按下/释放 |
| `key_press` | C→S | 键盘按键按下 |
| `app_launch` | C→S | 启动应用 |
| `system_info` | C→S | 请求系统信息 |
| `system_info_response` | S→C | 系统信息数据 |
| `disconnect` | 双向 | 优雅断开 |
| `error` | S→C | 错误响应 |

---

## 4. 通信流程

### 4.1 服务发现流程

```
┌──────────────┐                    ┌──────────────────────────┐
│ Flutter App  │                    │ Rust Desktop             │
│              │                    │                          │
│  multicast   │─── mDNS 查询 ─────▶│  mdns-sd 广播            │
│  dns-sd      │◀── 服务信息 ───────│  (_qinglink._tcp.local.) │
│              │                    │                          │
│  解析 TXT    │  { version, platform, host, port }            │
│  记录        │                    │                          │
└──────────────┘                    └──────────────────────────┘
```

### 4.2 配对流程

```
┌──────────────┐                    ┌──────────────────────────┐
│ Flutter App  │                    │ Rust Desktop             │
│              │                    │                          │
│              │── POST /pairing ──▶│                          │
│              │   { device_name }  │                          │
│              │                    │── 显示 PIN 弹窗 ──▶ GPUI │
│              │◀── { pin, exp ───│                          │
│              │                    │                          │
│  用户输入    │                    │                          │
│  PIN         │── POST /confirm ──▶│                          │
│              │   { pin }          │                          │
│              │                    │── 验证 PIN               │
│              │◀── { token } ─────│                          │
│              │                    │                          │
└──────────────┘                    └──────────────────────────┘
```

### 4.3 控制流程（WebSocket）

```
┌──────────────┐                    ┌──────────────────────────┐
│ Flutter App  │                    │ Rust Desktop             │
│  WsClient    │                    │  WS Handler              │
│              │                    │                          │
│              │── WS /events ─────▶│                          │
│              │   auth: token      │── validate_token()       │
│              │◀── connected ─────│                          │
│              │                    │                          │
│              │── mouse_move ─────▶│                          │
│              │                    │──▶ 服务层                │
│              │                    │──▶ qinglink-service      │
│              │                    │    (named pipe)           │
│              │◀── ack ──────────│                          │
│              │                    │                          │
└──────────────┘                    └──────────────────────────┘
```

---

## 5. 禁止模式

### 5.1 禁止重复实现

| 逻辑 | 唯一授权位置 |
|------|-------------|
| 重连逻辑 | 仅 `ws_client.dart` |
| 健康检查 | 仅 `ws_client.dart` |
| 退避计算 | `ws_client.dart`（共享方法） |
| Token 刷新 | 仅 `auth_provider.dart` |
| 配对状态 | 仅 `qinglink-domain/pairing.rs` |

### 5.2 禁止跨层访问

```rust
// ❌ 错误 — handler 直接访问存储
async fn list_apps(State(db): State<AppDb>) -> Json<Vec<App>> {
    Json(db.get_apps())
}

// ✅ 正确 — handler 委托给服务
async fn list_apps(State(svc): State<AppService>) -> Json<Vec<App>> {
    Json(svc.list_apps().await.unwrap_or_default())
}
```

### 5.3 禁止在传输层写业务逻辑

```rust
// ❌ 错误 — handler 中有鼠标限幅逻辑
async fn mouse_move(Json(req): Json<MouseMove>) -> impl IntoResponse {
    let dx = req.dx.clamp(-1000, 1000);
    let dy = req.dy.clamp(-1000, 1000);
    send_input(dx, dy).await;
}

// ✅ 正确 — 验证在协议边界，逻辑在服务层
async fn mouse_move(Json(req): Json<MouseMove>) -> impl IntoResponse {
    let validated = req.validate().map_err(|e| (400, e.to_string()))?;
    mouse_service::handle_move(validated).await
        .map_err(|e| (500, e.to_string()))
}
```

### 5.4 禁止无类型数据流

```dart
// ❌ 错误 — 数据以无类型 Map 流经应用
Future<Map<String, dynamic>> getSystemInfo() async { ... }

// ✅ 正确 — 从 wire 到 UI 全程类型化
Future<SystemInfo> getSystemInfo() async {
  final raw = await transport.request(SystemInfoRequest());
  return SystemInfo.fromJson(raw);
}
```

---

## 6. 配置管理

### 6.1 环境变量

| 变量 | 用途 | 层级 |
|------|------|------|
| `QINGLINK_LOG_LEVEL` | 覆盖日志级别 | 所有 |
| `QINGLINK_CONFIG_DIR` | 自定义配置目录 | desktop |
| `QINGLINK_TEST_MODE` | 启用测试辅助 | desktop（仅开发） |
| `QINGLINK_SERVICE_PIPE` | 自定义管道名 | service |

### 6.2 文件位置

| 平台 | 配置路径 | 数据路径 |
|------|---------|---------|
| Windows | `%APPDATA%/QingLink/` | `%LOCALAPPDATA%/QingLink/` |
| 未来 Linux | `~/.config/qinglink/` | `~/.local/share/qinglink/` |

### 6.3 密钥管理

- Token：移动端用 `flutter_secure_storage`，桌面端用加密 SQLite
- PIN：永不持久化，仅在配对期间存于内存
- 管道密码：每会话随机生成，通过 stdin 传输（非 env）

---

## 7. GPUI 线程模型（关键）

GPUI（Zed 编辑器的 UI 框架）有严格的线程模型约束。桌面端必须遵循以下规则，否则会导致数据竞争或 UI 崩溃。

### 7.1 GPUI 线程模型概览

```
┌─────────────────────────────────────────────────────────────┐
│                    GPUI 主线程（UI 线程）                      │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  GPUI 事件循环 (app.run(|cx| { ... }))                  │ │
│  │  - 窗口管理                                              │ │
│  │  - 渲染周期 (render(&mut self, window, cx))             │ │
│  │  - 事件处理（点击、窗口事件）                              │ │
│  │  - 持有所有 View/Entity 的可变引用                        │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌───────────────┐   ┌─────────────────┐   ┌──────────────────┐
│ cx.spawn()    │   │ ServiceRuntime  │   │  单例监听器       │
│ 后台任务       │   │ (独立线程 +     │   │  (独立线程)      │
│               │   │  Tokio Runtime) │   │                  │
│ - 定时器轮询   │   │                 │   │  TCP Listener    │
│ - 状态同步     │   │  Axum/WS 服务器 │   │  (127.0.0.1:3722)│
│               ││   │  服务发现       │   │                  │
└───────────────┘   └─────────────────┘   └──────────────────┘
```

### 7.2 核心线程规则

| 规则 | 说明 |
|------|------|
| **UI 操作只能在 GPUI 线程执行** | 所有 `cx.notify()`、`cx.open_window()`、View 更新必须在 GPUI 主线程 |
| **`cx.spawn()` 用于后台异步任务** | 后台任务通过 `view.update(async_cx, \|view, cx\| { ... })` 回到 UI 线程 |
| **`async_cx.background_executor()`** | GPUI 提供的异步执行器，用于 spawn 内的定时器和异步操作 |
| **`cx.update()` / `view.update()`** | 从后台线程安全地更新 View 状态，GPUI 内部同步 |
| **禁止跨线程直接修改 View** | 不能从 `std::thread` 或 `tokio::spawn` 直接修改 View 状态 |

### 7.3 共享状态模式

桌面端使用多种模式在 GPUI 线程和后台线程间共享状态：

#### 模式 A：`Arc<Mutex<T>>` + 轮询（推荐用于 UI 频繁读取）

```rust
// 共享状态定义
let pairing_snapshot = Arc::new(Mutex::new(None));

// 后台线程写入
if let Ok(mut guard) = pairing_snapshot.lock() {
    *guard = Some(new_request);
}

// GPUI spawn 任务读取并更新 View
cx.spawn(async move |view, async_cx| {
    loop {
        async_cx.background_executor()
            .timer(Duration::from_millis(250))
            .await;
        if view.update(async_cx, |view, _cx| {
            view.flush_pending_pairing();
            _cx.notify();
        }).is_err() {
            break;
        }
    }
}).detach();
```

**适用场景：** UI 需要定期检查后台状态变化（如配对请求、服务状态）。

#### 模式 B：`tokio::sync::watch`（推荐用于配置/期望状态）

```rust
// 创建 watch channel
let (desired_sender, desired_receiver) = watch::channel(DesiredServiceState::Running);

// UI 发送期望状态变更
self.desired_service.send(DesiredServiceState::Paused)?;

// 后台线程接收
let current = *desired_receiver.borrow();
```

**适用场景：** 单向状态传递（UI → 后台），如服务启停、自动启动切换。

#### 模式 C：`broadcast::channel`（推荐用于事件通知）

```rust
// 创建广播 channel
let (pairing_events, _) = broadcast::channel(16);

// 后台线程发送事件
pairing_events.send(PairingEvent::NewRequest(request))?;

// 前台订阅者接收
let mut subscriber = pairing_events.subscribe();
```

**适用场景：** 一对多事件通知，如配对请求、连接状态变化。

#### 模式 D：`Arc<AtomicBool>`（推荐用于简单标志位）

```rust
let exiting = Arc::new(AtomicBool::new(false));

// 任意线程设置
exiting.store(true, Ordering::Release);

// UI 线程读取
if self.exiting.load(Ordering::Acquire) {
    cx.quit();
}
```

**适用场景：** 简单的布尔标志，如退出信号、激活请求。

### 7.4 禁止的模式

```rust
// ❌ 错误 — 从 std::thread 直接修改 View 状态（绕过 GPUI）
std::thread::spawn(move || {
    let result = do_work();
    let _ = state.lock().map(|mut s| s.result_message = Some(result));
    // 问题：没有调用 cx.notify()，UI 不会刷新
    // 问题：state 锁与 GPUI 内部状态可能产生竞争
});

// ❌ 错误 — 从 tokio::spawn 直接操作 UI 元素
tokio::spawn(async move {
    cx.notify();  // 编译错误！cx 不是 Send
});

// ❌ 错误 — 持有 MutexGuard 跨越 await 点
let guard = mutex.lock().unwrap();
some_async().await;  // 锁跨越 await，可能导致死锁或阻塞 UI
drop(guard);

// ✅ 正确 — 需要通知 UI 时，使用回调或 channel
std::thread::spawn(move || {
    let result = do_work();
    let _ = result_sender.send(result); // 发送到 channel
});

// 然后在 cx.spawn 中监听 channel：
cx.spawn(async move |view, async_cx| {
    loop {
        if let Ok(result) = result_receiver.try_recv() {
            view.update(async_cx, |view, cx| {
                view.message = Some(result);
                cx.notify();
            })?;
        }
        async_cx.background_executor()
            .timer(Duration::from_millis(100)).await;
    }
}).detach();
```

### 7.5 后台任务生命周期管理

```rust
struct RemoteDashboard {
    // 存储 join handle 或取消 token
    background_tasks: Vec<Task<()>>,
}

impl RemoteDashboard {
    fn spawn_background_tasks(&mut self, cx: &mut Context<Self>) {
        // 任务 1：轮询配对状态
        let handle = cx.spawn(async move |view, async_cx| {
            loop {
                async_cx.background_executor()
                    .timer(Duration::from_millis(250)).await;
                if view.update(async_cx, |view, cx| {
                    view.flush_pending_pairing(cx);
                }).is_err() {
                    break; // View 已销毁，退出任务
                }
            }
        });
        self.background_tasks.push(handle.detach());
    }
}

impl Drop for RemoteDashboard {
    fn drop(&mut self) {
        // GPUI 会在 View 销毁时自动停止 spawn 的任务
        // 但需要手动停止独立的 std::thread 和 tokio runtime
    }
}
```

### 7.6 ServiceRuntime 线程模型

ServiceRuntime 运行在独立线程，拥有独立的 Tokio runtime：

```
┌─────────────────────────────────────────────────────────────┐
│                      GPUI 主线程                              │
│  RemoteDashboard View                                       │
│  - desired_service: watch::Sender                           │
│  - pairing: Arc<Mutex<Option<...>>>                         │
│  - service_status: Arc<Mutex<ServiceStatus>>                │
└──────────────────────────┬──────────────────────────────────┘
                           │ watch channel / Mutex
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   ServiceRuntime 线程                         │
│  tokio::runtime::Runtime                                    │
│  - Axum HTTP/WS 服务器                                       │
│  - mDNS 服务发现                                             │
│  - 服务监控循环                                               │
│  - 配对事件处理                                               │
└─────────────────────────────────────────────────────────────┘
```

**交互规则：**
- UI → 后台：通过 `watch::Sender` 发送期望状态
- 后台 → UI：通过 `Arc<Mutex<>>` 更新状态 + GPUI 轮询读取
- 配对事件：通过 `broadcast::channel` 通知

### 7.7 常见陷阱

| 陷阱 | 后果 | 解决方案 |
|------|------|---------|
| 从 `std::thread` 修改 View 状态 | UI 不刷新、数据竞争 | 使用 `cx.spawn()` + `view.update()` |
| 持有锁跨越 `await` | 死锁、UI 卡顿 | 在 `await` 前 `drop(guard)` |
| 在 `cx.spawn` 内执行阻塞操作 | 阻塞 GPUI 事件循环 | 使用 `async_cx.background_executor().spawn()` |
| 后台线程 panic 导致 Mutex poison | 后续 `.lock().ok()` 静默失败 | 用 `.map_err()` 传播 poison 错误 |
| 忘记调用 `cx.notify()` | 状态变化但 UI 不重绘 | 每次状态变更后必须 notify |

---

## 8. 版本策略

- 协议版本在 `qinglink-protocol/Cargo.toml` 和 schema `protocol/qinglink-v2.schema.json` 中定义
- 破坏性协议变更需要 schema 版本升级
- 移动端和桌面端必须在 WS 握手时协商协议版本
- 向后兼容：支持 N 和 N-1 协议版本
