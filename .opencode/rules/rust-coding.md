# Rust 编码规范 — QingLink

基于 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) 和 [rust-lang/rust](https://github.com/rust-lang/rust) 官方约定。

---

## 1. 命名规范

遵循 [RFC 430: 命名约定](https://rust-lang.github.io/api-guidelines/naming.html)。

| 项目 | 规范 | 示例 | 来源 |
|------|------|------|------|
| 模块/文件 | `snake_case` | `connection_registry.rs` | 标准 |
| 结构体/枚举/trait | `PascalCase` | `WsConnection`, `AuthToken` | 标准 |
| 函数/方法 | `snake_case` | `validate_token()`, `get_setting()` | 标准 |
| 局部变量 | `snake_case` | `device_name`, `reconnect_attempt` | 标准 |
| 常量/静态 | `SCREAMING_SNAKE_CASE` | `MAX_BACKOFF_SECONDS`, `DEFAULT_TIMEOUT` | 标准 |
| 类型参数 | 短 `PascalCase` | `T`, `E`, `C` | 标准 |
| 生命周期 | 短小写 | `'a`, `'ctx` | 标准 |
| Cargo feature | `kebab-case` | `runtime_shaders` | Cargo |

**使用完整单词** — 不使用歧义缩写：
- `message` 而非 `msg`
- `connection` 而非 `conn`（除非局部作用域 < 5 行）
- `database` 而非 `db`（除非在 `app_db` 字段名中）
- `index` 而非 `idx` 或 `i`（除非在简单循环中）

**Getter/Setter** 遵循 Rust 约定：
```rust
fn name(&self) -> &str          // getter: name()
fn set_name(&mut self, s: &str) // setter: set_name()
fn get_name(&self) -> &str      // 错误 — 不要加 get_ 前缀
```

---

## 2. Import 组织

遵循 `rustfmt` 默认分组 + 项目自定义排序：

```rust
// 1. 标准库（std, core, alloc）
use std::collections::HashMap;
use std::sync::Arc;

// 2. 外部 crate（第三方）
use axum::Extension;
use serde::Deserialize;
use tokio::sync::Mutex;

// 3. 内部 crate（qinglink-*）
use qinglink_protocol::PairingRequest;
use qinglink_domain::PairingState;

// 4. 本地模块导入（super, self）
use super::auth::TokenStore;
use crate::server::ServerState;
```

**规则：**
- 尽量一行一个 `use`（关系密切的可用多路径 `use foo::{bar, baz}`）
- 禁止使用 `use *` 通配符**导入**（`use x::*;` 引入未列名的符号）
- 禁止函数内 `use` — 所有 import 在模块顶部
- 内部路径使用 `crate::`，不用超过一层的相对 `super::` 链
- 重导出（`pub use`）只允许出现在 `lib.rs` 与**模块门面**（目录模块的 `mod.rs`）中：拆分被广泛导入的文件后，门面用它把子模块的公开符号还原到原路径，保证调用方零改动。
  - 门面优先**逐项列出**（`pub use window::{focus_window, enum_windows};`）；符号面较大时允许 `pub use window::*;`，但该 glob 只能出现在门面文件里，不得用在普通模块的 `use` 上。

---

## 3. 类型系统

### 3.1 结构体

- 派生宏优先级：`Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize`
- 超过 3 个字段时，优先使用 `#[derive(Default)]` + 字段初始化而非构造函数
- `new()` 作为主构造函数；可选字段用 `with_*()` 变体
- 用标记结构体替代无变体的枚举

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            max_connections: 16,
        }
    }
}

// 使用 Default 覆盖的构造方式 — 符合 Rust 习惯
let config = ServerConfig {
    port: 9090,
    ..Default::default()
};
```

### 3.2 枚举

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WsState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
}
```

- 库 crate 错误枚举使用 `#[derive(Error)]`（来自 `thiserror`）
- 穷尽匹配：仅在需要未来兼容时添加 `_` 分支，并加 `unreachable!()` 注释

### 3.3 类型别名

```rust
pub type ConnectionMap = Arc<RwLock<HashMap<DeviceId, Connection>>>;
pub type RequestId = Uuid;
```

### 3.4 Newtype 模式保证领域安全

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(Uuid);

impl DeviceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

---

## 4. 错误处理

### 4.1 错误类型策略

遵循 [thiserror](https://docs.rs/thiserror) + [anyhow](https://docs.rs/anyhow) 最佳实践：

| Crate 层级 | 错误类型 | 使用的 crate |
|-----------|---------|-------------|
| `qinglink-protocol` | 单元枚举或 `thiserror` | `thiserror` |
| `qinglink-domain` | `thiserror` 枚举 | `thiserror` |
| `qinglink-storage` | `thiserror` 枚举 | `thiserror` |
| `qinglink-server` | 内部用 `thiserror`，handler 用 `anyhow` | 两者 |
| `apps/desktop` | `anyhow::Error` | `anyhow` |
| `services/qinglink-service` | `anyhow::Error` | `anyhow` |

### 4.2 结构化错误（库 crate）

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("数据库连接失败: {0}")]
    ConnectionFailed(String),
    
    #[error("迁移 {version} 失败: {reason}")]
    MigrationFailed { version: u32, reason: String },
    
    #[error("记录未找到: {0}")]
    NotFound(String),
    
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}
```

**规则：**
- 每个变体必须有描述性的 `#[error("...")]` 消息
- 使用 `#[from]` 自动转换底层错误
- 使用 `#[error(transparent)]` 委托显示的包装变体
- 禁止直接使用 `String` 或 `&str` 作为错误类型

### 4.3 上下文错误（应用 crate）

```rust
use anyhow::{Context, Result, bail};

fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("读取配置文件失败: {}", path.display()))?;
    
    let config: Config = serde_json::from_str(&content)
        .context("解析配置 JSON 失败")?;
    
    if config.port == 0 {
        bail!("config.port 不能为零");
    }
    
    Ok(config)
}
```

### 4.4 Poisoned Lock 处理

```rust
// ✅ 正确 — 传播并附带上下文
let data = shared_state.lock()
    .map_err(|e| format!("状态锁 poisoned: {e}"))?;

// 对于不应 poison 的 Mutex（临界区很短）：
let guard = mutex.lock().expect("锁持有时间过长导致 poisoned");

// ❌ 错误 — poison 时静默返回 None
let data = shared_state.lock().ok().unwrap_or_default();
```

### 4.5 Clippy 配置

已在 `Cargo.toml` 中强制：
```toml
[workspace.lints.clippy]
dbg_macro = "deny"
todo = "warn"
unwrap_used = "warn"
```

**规则：**
- 生产代码中 `unwrap()` 为 **DENY** — 使用 `?`、`if let` 或 `match`
- `expect()` 需要注释说明原因：`expect("不变量：在 new() 中初始化")`
- `todo!()` / `unimplemented!()` 为 **WARN** — 仅用于 WIP 分支，禁止合入
- 库 crate 中 **禁止** `panic!`
- `unreachable!()` 允许，需注释说明不变量
- **禁止** 用 `catch_unwind` 做控制流 — 使用 `Result`

---

## 5. Unsafe 代码

遵循 [The Rustnomicon: Unsafe 指南](https://doc.rust-lang.org/nomicon/)。

### 5.1 文档要求

每个 `unsafe fn` 必须有 `# Safety` 文档段落。每个 `unsafe` 块必须有 `// SAFETY:` 注释：

```rust
/// 向系统发送键盘输入事件。
///
/// 封装 `user32` 的 `SendInput`。确保所有按键都有匹配的释放，
/// 防止按键卡住。
///
/// # Safety
///
/// 调用者必须保证：
/// - `inputs` 指向有效的 `INPUT` 结构体数组
/// - `count` 准确反映数组长度（必须 > 0，≤ 256）
/// - 没有其他线程同时调用 `SendInput`（Windows 限制）
unsafe fn send_input(inputs: *const INPUT, count: u32) -> u32 {
    // SAFETY: inputs 由调用者验证（上面的 count 检查），count ≤ 256
    unsafe { SendInput(count, inputs, std::mem::size_of::<INPUT>() as i32) }
}
```

### 5.2 安全抽象边界

```rust
// ✅ 正确 — unsafe 封装在安全包装器中
pub fn press_key(vk_code: u16) -> Result<(), InputError> {
    let inputs = build_key_input(vk_code)
        .map_err(|e| InputError::InvalidKey(e))?;
    
    // SAFETY: inputs.len() == 2（按下 + 释放），由 build_key_input 验证
    let result = unsafe { send_input(inputs.as_ptr(), inputs.len() as u32) };
    
    if result == 0 {
        return Err(InputError::SendFailed(windows::core::Error::from_win32()));
    }
    Ok(())
}

// ❌ 错误 — 向调用者暴露 unsafe
pub unsafe fn raw_press_key(vk: u16) { ... }
```

### 5.3 Windows FFI 规则

- `qinglink-windows` 中所有 `extern "system"` 函数必须为 `unsafe fn`
- `qinglink-windows` 中的安全包装器必须将 Windows 错误转换为 `thiserror` 类型
- `HANDLE` 值用 RAII 类型包装，在 `Drop` 中调用 `CloseHandle`
- 原始指针（`*mut T`, `*const T`）使用前必须检查空

---

## 6. 异步 / Tokio

### 6.1 任务生成

```rust
// ✅ 正确 — JoinHandle 存储在结构体中
struct BackgroundWorker {
    handle: JoinHandle<()>,
}

impl BackgroundWorker {
    fn new() -> Self {
        let handle = tokio::spawn(async {
            // ...
        });
        Self { handle }
    }
}

impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
```

### 6.2 超时强制

```rust
// ✅ 正确 — 所有 I/O 操作必须有超时
use tokio::time::{timeout, Duration};

async fn read_pipe(pipe: &mut NamedPipe) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; 1024];
    let n = timeout(Duration::from_secs(5), pipe.read(&mut buf))
        .await
        .context("管道读取超时")??;
    buf.truncate(n);
    Ok(buf)
}
```

### 6.3 取消安全

- `tokio::spawn` 生成的任务必须是取消安全的（不会导致部分状态损坏）
- 需要优先级排序时使用 `tokio::select!` + `biased;`
- 临界区仅使用取消安全操作

### 6.4 异步中的锁

- 永远不要在 `.await` 点上持有 `.lock().await` 执行非平凡工作
- 优先使用消息传递（channel）而非共享可变状态
- 跨 await 点的锁使用 `tokio::sync::Mutex`（异步感知）
- 仅在不跨 await 的短临界区使用 `std::sync::Mutex`

---

## 7. Trait 和泛型

### 7.1 Trait 命名

- 能力 trait 用动词形式：`Readable`, `Writable`, `Connectable`, `Discoverable`
- 能力 trait 用 `-able` 后缀（Rust 约定）
- 标记 trait 用名词形式：`Send`, `Sync`, `Error`

### 7.2 泛型参数

```rust
// ✅ 正确 — 带 trait 约束的泛型
pub fn process_events<E: EventHandler>(handler: &mut E) -> Result<()> {
    // ...
}

// 泛型参数命名
pub trait Repository<T>
where
    T: Entity,
{
    fn find(&self, id: T::Id) -> Result<T>;
    fn save(&self, entity: &T) -> Result<()>;
}
```

### 7.3 Trait 实现

- `Display` 面向用户格式化，`Debug` 面向开发者
- `From<T>` / `TryFrom<T>` 做转换（可能失败时优先 `TryFrom`）
- `FromStr` 做字符串解析
- `Serialize` / `Deserialize` 做 wire 格式序列化

```rust
impl From<StorageError> for WsError {
    fn from(err: StorageError) -> Self {
        WsError::Internal(err.to_string())
    }
}
```

---

## 8. 文档

### 8.1 文档注释

每个公开 API 必须有文档注释（`///`）。遵循 [rustdoc 约定](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html)：

```rust
/// 向桌面端发送鼠标移动命令。
///
/// # 参数
///
/// * `dx` — 水平像素偏移，限制在 `[-1000, 1000]`
/// * `dy` — 垂直像素偏移，限制在 `[-1000, 1000]`
///
/// # 错误
///
/// WebSocket 关闭时返回 `Err(TransportError::Disconnected)`。
/// 5 秒内未收到 ACK 返回 `Err(TransportError::Timeout)`。
///
/// # 示例
///
/// ```
/// # use qinglink_server::send_mouse_move;
/// # async fn demo(ws: &mut WsConnection) {
/// send_mouse_move(ws, 10, -5).await.unwrap();
/// # }
/// ```
pub async fn send_mouse_move(ws: &mut WsConnection, dx: i32, dy: i32) -> Result<()> {
```

### 8.2 模块级文档

在 `lib.rs` 和模块文件顶部使用 `//!`：

```rust
//! QingLink 存储层。
//!
//! 提供 SQLite 持久化，支持迁移和连接池。
//! 所有操作通过 `spawn_blocking` 兼容异步。
```

### 8.3 注释风格

- **所有注释使用中文**（项目中文优先）
- `///` 用于公开 API 文档，`//!` 用于模块文档
- `//` 用于实现注释，`// FIXME:` 用于已知问题
- 禁止提交注释掉的代码
- 不使用 `TODO:` — 用 `// FIXME: <描述>` 或创建 GitHub issue

---

## 9. 测试

> **前置声明：本仓库当前不新增测试代码，也不运行测试套件**（见 [`AGENTS.md`](../../AGENTS.md) §4）。
> 本节仅在人类明确要求编写测试时适用；不要据此主动创建 `#[cfg(test)]` 模块或 `tests/` 文件。

### 9.1 测试组织

```rust
// 在 src/ 文件中：
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_request() {
        // ...
    }

    #[tokio::test]
    async fn connect_success() {
        // ...
    }
}

// 在 tests/integration.rs（集成测试）
```

### 9.2 异步测试

```rust
#[tokio::test]
async fn ws_handler_rejects_invalid_token() {
    let (mut client, server) = setup_test_pair().await;
    
    let result = timeout(Duration::from_secs(2), client.authenticate("invalid")).await;
    
    assert!(matches!(result, Err(AuthError::InvalidToken)));
}
```

### 9.3 测试工具

- 共享测试辅助函数放在 `tests/common/mod.rs` 或 crate 级 `test_util` 模块
- 使用 `#[cfg(test)]` 隔离测试代码
- 测试名称描述行为：`should_reject_expired_token`, `returns_empty_for_no_apps`

### 9.4 协议测试

所有协议类型必须通过 JSON Schema 验证：
```bash
# 运行 schema 验证
cargo test --package qinglink-protocol --test schema_validation
```

---

## 10. Workspace 配置

### 10.1 Lint 规则

```toml
[workspace.lints.rust]
unsafe_op_in_unsafe_fn = "warn"  // 要求 unsafe fn 内的 unsafe 块有安全注释
missing_docs = "warn"            // 公开 API 必须有文档
missing_debug_implementations = "warn"  // 公开类型必须有 Debug

[workspace.lints.clippy]
// 限制级 lint（错误）
dbg_macro = "deny"
todo = "warn"
unwrap_used = "warn"

// 严格级 lint（选择启用）
clippy::redundant_clone = "warn"
clippy::needless_pass_by_value = "warn"
clippy::missing_errors_doc = "warn"
clippy::missing_panics_doc = "warn"
```

### 10.2 Feature Flags

所有 crate 使用加法 feature：
```toml
[features]
default = ["std"]
std = []
```

**规则：**
- Feature 必须是加法的（不能有互斥 feature）
- 使用 `dep:` 语法激活可选依赖：`serde = ["dep:serde"]`

---

## 11. 文件规模（硬约束）

完整规则见仓库根目录 [`AGENTS.md`](../../AGENTS.md) §2；本节是 Rust 侧的落地要求。

| 对象 | 建议值 | 硬上限 |
|------|--------|--------|
| 单个 `.rs` 文件 | 400 行 | **500 行** |
| 单个函数 / 方法 | 80 行 | 120 行 |
| 单个 struct / enum / impl 块 | 300 行 | 400 行 |

**拆分原则：**

- 按职责拆模块，不按行数机械切（禁止 `xxx_part2.rs`）。
- `mod.rs` 只做模块声明与重导出，不承载实现；实现放同目录子模块。
- 大块 `impl` 按职责拆到多个文件，用 `impl Type` 分块而不是一个巨型 `impl`。
- Windows FFI / 平台代码按功能域拆分（输入、显示、电源、窗口目录各自成文件）。
- platform 相关的 `#[cfg(windows)]` 分支不要靠堆 `cfg` 撑大单文件。

**棘轮：** 现存超限文件列在仓库根目录 [`AGENTS.md`](../../AGENTS.md) §2.3 的表格里，
只许减少、不许增加；降到所属类型上限以内即移出该表。新增文件没有豁免。

**校验：** 本仓库没有行数校验脚本（[`AGENTS.md`](../../AGENTS.md) §7），用 §2.4 的原生命令自查：

```powershell
(Get-Content <file>).Count
```

**禁止：** 用压缩语句、超长行、删除必要空行、把代码塞进 `#[cfg(...)]` / 注释块 / `include!`
等方式规避行数统计——一经发现按未完成处理。

