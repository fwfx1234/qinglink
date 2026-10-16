# 架构规范 — QingLink

基于 [Rust crate 生态系统模式](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html) 和 [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html) 原则。

---

## 1. Crate 依赖图

```
叶子 crate（不依赖任何内部 crate）：
    qinglink-protocol      线型 DTO 与 schema 契约
    qinglink-domain        配对状态机与能力协商
    qinglink-storage       SQLite 持久化与路径解析

中间层：
    qinglink-discovery  ──▶ qinglink-protocol
    qinglink-windows    ──▶ qinglink-protocol, qinglink-domain

核心层：
    qinglink-server     ──▶ qinglink-protocol, qinglink-domain,
                            qinglink-storage, qinglink-windows

应用层：
    apps/desktop            ──▶ qinglink-protocol, qinglink-storage, qinglink-windows
    services/qinglink-service ──▶ qinglink-protocol, qinglink-discovery,
                                  qinglink-server, qinglink-storage, qinglink-windows
```

**apps/desktop 不依赖 `qinglink-server`**：桌面端自带 Web 管理面，服务端逻辑只在
`qinglink-service` 进程里运行。箭头方向即允许的依赖方向，反向依赖一律禁止。

### 依赖规则（严格）

下表按各 crate 真实 `Cargo.toml` 的内部依赖（`path = ...`）列出，**这张表以
`Cargo.toml` 为准**；新增内部依赖必须同时更新本表。

| Crate | 可依赖（内部 crate） | 不可依赖 |
|-------|--------|---------|
| `qinglink-protocol` | 无（叶子 crate） | 任何内部 crate |
| `qinglink-domain` | 无（叶子 crate） | 任何内部 crate |
| `qinglink-storage` | 无（叶子 crate） | 任何内部 crate |
| `qinglink-discovery` | `qinglink-protocol` | server, storage, domain, windows |
| `qinglink-windows` | `qinglink-protocol`, `qinglink-domain` | server, storage, discovery |
| `qinglink-server` | `qinglink-protocol`, `qinglink-domain`, `qinglink-storage`, `qinglink-windows` | discovery, apps/*, services/* |
| `apps/desktop`（`qinglink-desktop`） | `qinglink-protocol`, `qinglink-storage`, `qinglink-windows` | `qinglink-server`（自带 Web 层）, services/* |
| `services/qinglink-service`（`qinglink-service`） | `qinglink-protocol`, `qinglink-discovery`, `qinglink-server`, `qinglink-storage`, `qinglink-windows` | apps/* |

`qinglink-domain` 与 `qinglink-storage` 是**叶子 crate**：领域逻辑与持久化都必须能
独立编译，不得为了方便反向拉入 `qinglink-protocol` 或任何其它内部 crate。

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
- 数据库连接池注册表（`DatabaseService`：按路径/键取 r2d2 池）
- Schema 迁移（仅追加，见 `migrations.rs`）
- 数据目录与文件路径解析（`AppPaths`）

**规则：**
- 所有查询必须使用参数化语句（禁止字符串拼接）
- 迁移仅追加（禁止修改已有迁移文件）
- 连接池大小与超时集中在该 crate 内定义，调用方不自行建池
- **该 crate 是叶子 crate**：不得依赖 `qinglink-protocol` / `qinglink-domain` 或任何
  其它内部 crate；业务模型与仓库模式属于上层（`qinglink-server`），不要下沉到这里

### 2.4 `qinglink-server` — HTTP/WS 服务器

**目的：** 基于 Axum 的 HTTP 和 WebSocket 服务器。

**包含：**
- 路由处理器（`/api/v2/*`）
- WebSocket 生命周期管理
- 认证中间件
- 请求分发到服务层
- 连接注册表
- 游戏启动显示模式协调：持久化主显示器快照，跟踪新进程与子进程并在退出后恢复

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
│              │                    │── 显示 PIN 弹窗 ──▶ Web UI │
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

### 5.5 禁止超大文件

单文件超过行数上限属于结构违规，而不是风格问题：它通常意味着一个文件承担了多个职责，
crate/模块边界名存实亡。

| 对象 | 硬上限 |
|------|--------|
| 源码单文件（`.rs` / `.dart` / `.ts`） | 500 行 |
| `.vue` 单文件组件 | 400 行 |
| 单个函数 / 方法 | 120 行 |
| 文档单篇 | 1200 行 |

- 完整规则与棘轮清单见仓库根目录 [`AGENTS.md`](../../AGENTS.md) §2。
- 已有超限文件列在 [`AGENTS.md`](../../AGENTS.md) §2.3 的表格里，
  只许减少、不许增加；这是**优先重构目标清单**。
- 禁止把代码藏在 `include!`、`#[cfg(...)]`、注释块、字符串或 `part` 指令中来规避统计。
- 禁止以"文件太大"为由跨 crate 反向依赖或绕过既定模块职责——先按职责拆分，再保持依赖方向。

**校验：** 本仓库没有行数校验脚本（[`AGENTS.md`](../../AGENTS.md) §7），用 §2.4 的原生命令自查：

```powershell
(Get-Content <file>).Count
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

## 7. 桌面端线程模型

桌面端不使用 GUI 框架：主线程负责托盘与配对提示弹窗（两者都直接调用 Win32 API）与窗口消息，
Web 管理面跑在独立线程的 Axum 上，两者通过 `HostCommand` 通信；后台任务在 Tokio runtime 上。
**Web 处理函数不得直接触碰只属于 main 线程的状态**，必须走 `HostCommand`。
完整的进程/线程划分与职责边界见 [`docs/architecture.md`](../../docs/architecture.md) 第 4 节与第 8 节。

---

## 8. 版本策略

- 协议版本在 `qinglink-protocol/Cargo.toml` 和 schema `protocol/qinglink-v2.schema.json` 中定义
- 破坏性协议变更需要 schema 版本升级
- 移动端和桌面端必须在 WS 握手时协商协议版本
- 向后兼容：支持 N 和 N-1 协议版本
