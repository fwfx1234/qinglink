# 架构规范 — QingLink

基于 [Rust crate 生态系统模式](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html) 和 [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html) 原则。

---

## 1. Crate 依赖图

```
叶子 crate（不依赖任何内部 crate）：
    qinglink-protocol      线型 DTO 与 schema 契约
    qinglink-domain        配对状态机
    qinglink-storage       SQLite 持久化与路径解析
    qinglink-version       产品版本号与自增构建号

中间层：
    qinglink-discovery  ──▶ qinglink-protocol
    qinglink-windows    ──▶ qinglink-protocol

核心层：
    qinglink-server     ──▶ qinglink-protocol, qinglink-domain,
                            qinglink-storage, qinglink-windows,
                            qinglink-version

应用层：
    apps/desktop            ──▶ qinglink-protocol, qinglink-storage,
                                qinglink-windows, qinglink-version
    services/qinglink-service ──▶ qinglink-protocol, qinglink-discovery,
                                  qinglink-server, qinglink-storage,
                                  qinglink-windows, qinglink-version
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
| `qinglink-version` | 无（叶子 crate） | 任何内部 crate |
| `qinglink-discovery` | `qinglink-protocol` | server, storage, domain, windows |
| `qinglink-windows` | `qinglink-protocol` | server, storage, discovery, domain |
| `qinglink-server` | `qinglink-protocol`, `qinglink-domain`, `qinglink-storage`, `qinglink-windows`, `qinglink-version` | discovery, apps/*, services/* |
| `apps/desktop`（`qinglink-desktop`） | `qinglink-protocol`, `qinglink-storage`, `qinglink-windows`, `qinglink-version` | `qinglink-server`（自带 Web 层）, services/* |
| `services/qinglink-service`（`qinglink-service`） | `qinglink-protocol`, `qinglink-discovery`, `qinglink-server`, `qinglink-storage`, `qinglink-windows`, `qinglink-version` | apps/* |

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
// crates/qinglink-protocol/src/pairing.rs
/// 客户端发起配对时提交的自身信息。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingRequest {
    /// 发起配对的设备唯一标识。
    pub device_id: String,
    /// 发起配对的设备展示名称。
    pub device_name: String,
}
```

注意：**不是每个请求都要一个独立 DTO**。`WS /api/v2/events` 的上行请求是
`WsClientRequest` 的枚举变体（`crates/qinglink-server/src/protocol/requests_ws.rs`），
字段直接长在变体上，不要为某个变体再造一个 `*Request` 结构体。

### 2.2 `qinglink-domain` — 领域逻辑

**目的：** 无外部依赖的纯领域逻辑。

**包含：**
- `PairingManager`：待确认配对请求表、PIN 校验、尝试次数限流与 60 秒 TTL
- `PairingRequest` / `PairingResult` / `PairingError`（`pairing.rs`）
- 输入边界：`MAX_DEVICE_ID_LEN` / `MAX_DEVICE_NAME_LEN` / `MAX_PENDING_REQUESTS` / `MAX_ATTEMPTS`

**规则：**
- 禁止数据库访问、网络 I/O、平台 API
- 仅纯函数和确定性状态转换
- 无需任何外部设置即可测试

```rust
// 不可信输入必须走 try_request：它先校验设备元数据与待处理请求数上限。
let request = manager.try_request(device_id, device_name)?;
// 确认时同时核对 device_id，避免别的设备消耗本请求的 PIN 尝试次数。
let result = manager.confirm_for_device(request.request_id, device_id, pin)?;
```

`PairingManager` 刻意不 `derive(Debug)`，手写实现只输出 `pending` 条数——请求里带着 PIN，
`Debug` 输出会把 PIN 打进日志。

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
- 令牌校验与设备授权（`server/auth.rs`，**不是**中间件）
- 请求分发到服务层
- 连接注册表
- 游戏启动显示模式协调：持久化主显示器快照，跟踪新进程与子进程并在退出后恢复
- 保持唤醒：专用常驻线程串行持有 `SetThreadExecutionState` 请求，状态只在服务进程本次运行内有效

**规则：**
- Handler 要薄 — 5 行内委托给服务层
- 禁止直接数据库查询（使用 repository）
- 每个 WS handler 必须有超时
- `SetThreadExecutionState` 是 per-thread 状态，不得在 WS handler 或 `spawn_blocking` 工作线程上调用；保持唤醒统一走常驻线程
- 错误响应统一走 `WsServerResponse::error(code, message)`：线格式是
  `{"type":"error","data":{"code":...,"message":...}}`，**没有**顶层 `code` / `message`，
  也没有独立的 `ErrorResponse` 类型
- 所有中间件使用 `tower::Service` 模式

```rust
// crates/qinglink-server/src/server/lifecycle.rs
Router::new()
    .route("/api/v2/health", axum::routing::get(pairing::health))
    .route("/api/v2/pairing/requests", axum::routing::post(pairing::create_request))
    .route("/api/v2/pairing/confirm", axum::routing::post(pairing::confirm_request))
    .route("/api/v2/pairing/requests/:request_id", axum::routing::delete(pairing::cancel_request))
    .route("/api/v2/events", axum::routing::get(ws_handler::ws_handler))
    .route("/api/v2/input", axum::routing::get(input_ws_handler::input_ws_handler))
    .layer(Extension(state))
```

鉴权不是中间件：除 `/api/v2/health` 与配对的三个端点外，`WS /api/v2/events` 上的
`authenticate` 是唯一无需预先认证的指令，其余请求在路由表之前先过
`handle_client_request_in_generation`。

### 2.5 `qinglink-windows` — 平台层

**目的：** Windows 特定的 OS 集成（WinAPI 的安全包装器）。

**包含：**
- 音频控制（音量、静音）
- 显示管理（枚举显示模式、切换分辨率与刷新率）
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

**目的：** 应答移动端的 UDP 广播探测，让手机在没有配置的情况下找到 PC。

**实现：** 自定义 UDP 协议，**不是 mDNS**——本项目没有任何 mDNS 依赖。
监听 `3723`（`qinglink_protocol::DISCOVERY_PORT`），收包后与
`DISCOVERY_MAGIC`（`qinglink-discover-v1`）逐字节比较，不等于该字符串的包直接丢弃；
命中的包回一个 JSON：`{"id","name","port","version":2,"mac"}`。

**规则：**
- 保持"无状态、无认证、只读"：发现应答不携带令牌，也不授予任何权限
- 只在 TCP `3721` 绑定成功之后启动应答器（`network.rs` 在
  `RemoteServer::run_with_shutdown` 返回后才 `UdpDiscoveryResponder::start`）
- 后台线程 + 500ms 读超时轮询 `AtomicBool`，不要在这里引入 async 运行时依赖
- 协议语义见 [`protocol/v2.md`](../../protocol/v2.md)「UDP 发现」

### 2.7 `qinglink-version` — 产品版本号

**目的：** 桌面端与服务端共用的版本号，避免两处各写一份而逐渐不一致。

**规则：**
- 语义版本只有一处：`Cargo.toml` 的 `[workspace.package] version`。
- 展示用 `qinglink_version::VERSION`（`0.1.0+13`）；不要再用 `env!("CARGO_PKG_VERSION")`
  展示给用户——那样看不到构建号，也无法区分两次改动。
- 构建号由本 crate 的 `build.rs` 按源码指纹自增，状态在 `build-number.txt`，**必须随改动提交**；
  该文件刻意不在 `cargo:rerun-if-changed` 的监视集合里。
- Windows 版本资源不要写死在 `.rc` 里，用 `qinglink_version::version_info_rc` 在构建脚本中生成。
- 桌面端与服务端的版本号必须来自同一次构建：两者都直接依赖本 crate。

---

## 3. WebSocket 协议契约

**本节不复述契约内容**——完整契约只有两份，改协议先读它们：
[`protocol/v2.md`](../../protocol/v2.md)（按主题讲语义）与
[`protocol/requests.md`](../../protocol/requests.md)（上行变体索引、载荷字段、错误码）。

只记四条判据：

- **信封**：`{"type": ..., "request_id": ..., "data": ...}`。`request_id` 由客户端放在
  **顶层**，服务端 `serialize_ws_message` 回写到顶层；服务端主动推送的事件帧没有它。
- **类型标签**：由 `#[serde(tag = "type", content = "data")]` 决定，请求字段**平铺在
  `type` 同级**，没有 `payload` 容器。
- **错误帧**：`{"type":"error","data":{"code":...,"message":...}}`——没有 `ok`，没有顶层
  `code`/`message`，错误码在 `data.code`。HTTP 端是另一套（`{"ok":false,"error":{...}}`），
  两者字段名不可互换。
- **错误码与消息类型**：唯一注册表是 `protocol/requests.md`。本文件**不再维护第二份**。

---

## 4. 通信流程

**拓扑**：`Flutter App` ↔ `QingLinkService`（TCP `3721` 的 HTTP/WS、UDP `3723` 发现）；
桌面端 `QingLink.exe` 是同一服务的**另一个客户端**，走命名管道 `\\.\pipe\QingLinkControlV2`。
手机与桌面端**不直接通信**。

- **发现**（见 [`protocol/v2.md`](../../protocol/v2.md)「UDP 发现」）：手机广播
  `qinglink-discover-v1`，服务在 TCP `3721` 就绪后回 JSON，手机按 `id` 去重。
- **配对**（见「配对」）：`POST /api/v2/pairing/requests` 建请求 → PC 弹窗显示 6 位 PIN →
  `POST /api/v2/pairing/confirm` 换长期令牌。请求本身不授予权限，响应里没有 `token`。
- **控制**：带令牌在 `WS /api/v2/events` 发 `authenticate`，之后按
  `protocol/requests.md` 的类型收发；高频指针走 `WS /api/v2/input`。
- **会话关闭**（见「会话关闭」）：服务端先下发
  `{"type":"session_closed","data":{"code":...}}` 再断连，客户端按 `code` 决定是否清令牌。

---

## 5. 禁止模式

### 5.1 禁止重复实现

| 逻辑 | 唯一授权位置 |
|------|-------------|
| WS 心跳（发 `health`、超时判定） | `apps/mobile/lib/src/services/ws_client.dart`（`_startHealthChecks`，默认 10 s 间隔 / 5 s 超时，间隔可由会话层经 `setHealthInterval` 透传） |
| 会话级重连调度与退避曲线 | `apps/mobile/lib/src/services/remote_session_connection.dart`（`scheduleReconnect`：首次 500 ms，之后 1 s、2 s、4 s…上限 30 s，离线期可切固定 3 s） |
| 传输层自动重连 | `ws_client.dart` 保留该能力，但会话层建连时传 `autoReconnect: false`（`apps/mobile/lib/src/services/remote_session_types.dart`）把它关掉；两套定时器同时跑会互相抖动，新增重连逻辑一律挂在会话层 |
| 令牌持久化 | `apps/mobile/lib/src/providers/settings_notifier.dart`（`saveToken` / `clearToken`，经 `SecureTokenStore`）；令牌失效后的降级由 `session_connector.dart` 处理 |
| 配对请求状态机 | `crates/qinglink-domain/src/pairing.rs`（`PairingManager`）；桌面端设备 PIN 的生成与校验在 `crates/qinglink-server/src/service/mod.rs` |

### 5.2 禁止跨层访问

handler 只做三件事：解析入参、调用下层、把失败映射成错误码。禁止在 handler 里持有
数据库连接、命名管道句柄，或直接调 Win32 API。

```rust
// ✅ 真实写法（apps/desktop/src/web/api/service.rs）：Web handler 只把命令交给
// ServiceClient，命名管道、请求号与重连都由它负责
pub async fn pause(
    State(state): State<Arc<WebState>>,
    Json(request): Json<PauseRequest>,
) -> Result<Json<Value>, ApiError> {
    state
        .service
        .call(DesktopCommand::SetPaused { paused: request.paused })
        .await?;
    Ok(ok(&json!({ "accepted": true, "paused": request.paused })))
}
```

反例（**示意**，不对应任何真实文件）：handler 里 `rusqlite::Connection::open(&db)?`
然后自己拼 `UPDATE settings ...`，或者直接 `CreateNamedPipeW` + 手写帧。这类写法一旦
出现，schema 变更与鉴权就散落在路由层，无法统一收口。

### 5.3 禁止在传输层写业务逻辑

```rust
// ✅ 真实写法（crates/qinglink-server/src/server/ws_handler_display_input.rs）：
// 阈值来自协议层，输入注入来自平台层，handler 只钳制 + 映射错误码
fn handle_mouse_move(dx: i32, dy: i32) -> WsServerMessage {
    let dx = dx.clamp(
        -crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
        crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
    );
    let dy = dy.clamp(
        -crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
        crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
    );
    match crate::platform::mouse_move(dx, dy) {
        Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
        Err(error) => {
            tracing::warn!(dx, dy, %error, "mouse move failed");
            mouse_input_error(&error)
        }
    }
}
```

反例（**示意**，不对应任何真实文件）：同一个函数里内联 `SendInput` / `SetCursorPos`、
自己算 DPI 缩放，或把 `clamp(-1000, 1000)` 这类魔法数字抄进 handler。阈值必须来自
`crate::protocol::requests`，平台调用必须来自 `crate::platform`。

### 5.4 禁止无类型数据流

```dart
// ✅ 真实写法（apps/mobile/lib/src/providers/system_controller.dart）：
// 回包一离开传输层就解析成类型，上层只见到 SystemStatus
Future<void> refreshStatus() async {
  final response = await _bus.request({'type': 'get_system_status'});
  final value = WsEventAdapter.parseSystemStatus(response);
  if (value != null) state = state.copyWith(status: value, error: null);
}
```

请求体用 `{'type': '...'}` 这种 Map 是本仓库的既有写法；被禁止的是把**响应体**的
`Map<String, dynamic>` 继续往上抛，让 widget 里出现 `raw['cpu'] as num` 这类取值。

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

下表是**全部**生效的变量（逐个在源码里核对过）；新增变量必须同步本表，否则等于没有出处。

| 变量 | 用途 | 读取位置 |
|------|------|---------|
| `QINGLINK_DATA_DIR` | 覆盖数据目录，`AppPaths::resolve` 与 `resolve_system` 都优先用它（后者同时不再收紧 ACL） | desktop / service 运行时 |
| `QINGLINK_LOG_LEVEL` | 覆盖日志级别，优先于 `RUST_LOG` | desktop / service 运行时 |
| `RUST_LOG` | 同上；`QINGLINK_LOG_LEVEL` 未设时生效。裸级别名会展开成应用的两个 target | desktop 运行时 |
| `QINGLINK_STARTUP_TRACE` | 设任意值即把启动面包屑（`[qinglink-startup] +xx.xxxs`）逐条写 stderr | desktop 运行时 |
| `QINGLINK_REQUIRE_SIGNED_INPUT` | 设任意值即要求内嵌服务资源与桌面端自身签名均有效，否则提权助手拒绝安装 | desktop 运行时 |
| `QINGLINK_REQUIRE_INPUT_ASSETS` | 置 `1` 时内嵌服务资源缺失让**构建失败**；不设只打 `cargo:warning` | desktop `build.rs` |
| `QINGLINK_INPUT_SERVICE_PATH` | 指定内嵌服务二进制的来源目录（默认按 `assets/input/` → `target/{debug,release}/` 查找） | desktop `build.rs` |
| `QINGLINK_CI_BUILD_NUMBER` | 由 CI 从 tag 注入构建号，跳过源码指纹自增与状态写回 | `qinglink-version/build.rs` |
| `QINGLINK_BUILD_WEB` / `QINGLINK_SKIP_WEB_BUILD` | 强制重建前端 / 跳过前端构建直接内嵌现有 `dist` | desktop `build.rs` |

### 6.2 文件位置

只有两个数据根，没有"配置目录"这一层；两者都由 `AppPaths` 解析
（`crates/qinglink-storage/src/paths.rs`）：

| 所有者 | 路径 | 内容 | ACL |
|--------|------|------|-----|
| 桌面端（当前用户） | `%LOCALAPPDATA%\QingLink\`（`dirs::data_local_dir()`） | `qinglink.db`（桌面端偏好）、`logs\`、`updates\<版本>\`（更新包下载，见 `apps/desktop/src/updater/install.rs`） | 当前用户 |
| 服务（LocalSystem） | `%ProgramData%\QingLink\`（`ProgramData` 环境变量） | `qinglink.db`（业务表 `scanner_apps`、`scanner_scan_state`、`steam_games`、`launcher_usage`、`paired_devices`、`settings`、`tokens_v2`）、`config\pc-id`、`config\qinglink-pairing-pin`、`logs\` | `icacls /reset` + `/inheritance:r` 后只授 SYSTEM 与内置管理员完全控制；`logs\` 额外给 `BUILTIN\Users` 只读+执行 |

- `QINGLINK_DATA_DIR` 可整体覆盖（见 §6.1）；被覆盖时服务不再收紧 ACL，测试与便携部署才用它。
- `paths.rs` 里还有 `#[cfg(not(target_os = "windows"))]` 分支，退回用户 `data_local_dir()`；
  该分支下 ACL 收紧是空实现，服务日志的 `%ProgramData%` 解析同样只存在于 Windows 分支。

### 6.3 密钥管理

- **移动端令牌**：手机侧存平台安全存储（`flutter_secure_storage`，见
  `apps/mobile/lib/src/services/secure_token_store.dart`，写入入口是
  `apps/mobile/lib/src/providers/settings_notifier.dart` 的 `saveToken`）；服务端只存
  SHA-256（`tokens_v2.token_hash` 为主键，见 `crates/qinglink-server/src/server/auth.rs`），
  库被读走也拿不到可用令牌。
- **桌面端设备 PIN**：`RemoteControlService::generate_pin`
  （`crates/qinglink-server/src/service/mod.rs`）生成 6 位数字，内存持有并持久化到
  `%ProgramData%\QingLink\config\qinglink-pairing-pin`，服务重启后沿用；配对成功调
  `consume_pin` 删除该文件。连续 5 次校验失败后锁定 30 秒（`PIN_MAX_ATTEMPTS` /
  `PIN_LOCKOUT_SECS`），锁定期间正确 PIN 同样被拒。
- **单次配对请求的 PIN**：`PairingRequest.pin` 只活在 `PairingManager` 的内存里，60 秒
  TTL、最多 128 条待确认请求，不落盘；该管理器刻意不 `derive(Debug)`，手写 `Debug` 只输出
  `pending` 数量，PIN 不进日志。
- **桌面端身份**：没有共享密码。命名管道 DACL 为 `D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)`
  并带 `PIPE_REJECT_REMOTE_CLIENTS`（`services/qinglink-service/src/windows_service_impl/pipe.rs`）；
  会话驱动的 agent 管道进一步收紧到 `D:(A;;GA;;;SY)`。服务侧再用
  `GetNamedPipeClientProcessId` / `GetNamedPipeClientSessionId` 取对端 PID 与会话，要求
  处于交互式会话、且进程令牌 SID 与该会话用户一致，最后把进程映像路径与 SHA-256 同安装
  目录下的 `trusted-desktop.json`（`apps/desktop/src/input_install.rs` 写入）比对
  （`windows_service_impl/authorization.rs`）。桌面端被重建后哈希变化即判定未授权，需
  "修复服务"重新登记。
- **不要引入"管道密码"这类方案**：DACL 与可执行映像身份两道约束已经覆盖，再加一个每会话
  密钥只会多一个需要在进程间传输和轮换的秘密。

---

## 7. 桌面端线程模型

桌面端不使用 GUI 框架：主线程负责托盘与配对提示弹窗（两者都直接调用 Win32 API）与窗口消息，
Web 管理面跑在独立线程的 Axum 上，两者通过 `HostCommand` 通信；后台任务在 Tokio runtime 上。
**Web 处理函数不得直接触碰只属于 main 线程的状态**，必须走 `HostCommand`。
完整的进程/线程划分与职责边界见 [`docs/architecture.md`](../../docs/architecture.md) 第 4 节与第 8 节。

---

## 8. 版本策略

- **产品版本**（`0.1.0+<构建号>`）是另一件事，见 §2.7，不要与协议版本混用。
- 移动端↔服务端：协议版本由 URL 前缀 `/api/v2/` 与 `GET /api/v2/health` 的 `version`
  表达，当前**恒为 `2`**，没有多版本并存，也不做 N-1 兼容。破坏性变更就升 v3，
  旧客户端直接连不上——这是有意的取舍（协议共享半径只有手机端一个实现）。
- 桌面端↔服务端管道：`SERVICE_IPC_VERSION`（`service_ipc/envelope.rs`）是**唯一真正协商**
  的版本号。桌面端拿 `expected_version` 与服务回报的 `service_version` 比对，不等即进入
  `protocol_mismatch`，界面引导"修复服务（升级）"。
- 能力位（`Capabilities`）承担向前兼容：旧服务缺字段即为 `false`，桌面端据此返回
  `UNSUPPORTED`，而不是报协议错误。**新增命令优先加能力位，不要靠升版本号。**
- JSON Schema（`protocol/qinglink-v2*.schema.json`）的版本体现在 `$id` 的 `/v2` 段；
  改 schema 必须同批改 `fixtures/` 与 `apps/mobile/test/qinglink_contract_test.dart`。
