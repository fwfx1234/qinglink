//! 桌面端日志：采集、实时流、环形缓冲区与持久化文本文件。
//!
//! ```text
//! tracing::info!(..)
//!        ├─► LogCollector ──┬─► LogBuffer     (最近 5000 条，回答"刚刚发生")
//!        │                   └─► broadcast     (实时 WebSocket 订阅者)
//!        └─► fmt layer ────────► 磁盘上的滚动文本文件 (历史)
//! ```
//!
//! 没有日志数据库。`tracing_appender` 写出的文本文件就是持久记录：它能挺过崩溃，
//! 不依赖本应用即可阅读，也不会像打开 SQLite 连接池那样，在数据目录缓慢、被锁定
//! 或拒绝访问时拖慢启动。需要查询的内容都放在内存中，内存是日志页面的数据来源；
//! 更早的记录以文件形式下载。
//!
//! 两个内存 sink 都如实上报各自的失败而不是掩盖它们：广播通道对慢订阅者返回
//! `RecvError::Lagged`，这是唯一准确的反压信号。
//!
//! 本模块按职责拆分。子模块是私有的，调用方用到的每个符号都在下面重导出，因此
//! `crate::logging::<name>` 路径（`bootstrap.rs`、`web/api/logs.rs`、
//! `web/api/diagnostics.rs`）保持不变：
//!
//! * [`buffer`] — 内存环形缓冲区与查询谓词；
//! * [`collector`] — 填充内存 sink 的 `tracing` 层；
//! * [`entry`] — 一条记录及其严重级别；
//! * [`directory`] — 磁盘上的日志目录及其文件；
//! * [`log_file`] — 文件名校验与有界尾部读取；
//! * [`service_log`] — 服务端文本日志的解析与分页；
//! * [`subscriber`] — 过滤层、文件 sink 与级别开关；
//! * [`system`] — 组装完成的 [`LogSystem`] 及其查询接口。

mod buffer;
mod collector;
mod directory;
mod entry;
mod log_file;
mod service_log;
mod subscriber;
mod system;

// The re-exports below are the module's public interface, not local shortcuts.
// They are deliberately not imported here: callers reach them as
// `crate::logging::<name>`, so the compiler sees no in-module use for the ones
// only the sibling modules consume.
#[allow(unused_imports)]
pub use buffer::{LogBuffer, LogFilter};
#[allow(unused_imports)]
pub use collector::LogCollector;
#[allow(unused_imports)]
pub use directory::LogFileInfo;
#[allow(unused_imports)]
pub use entry::{LogEntry, LogLevel};
#[allow(unused_imports)]
pub use subscriber::level_override_from_environment;
#[allow(unused_imports)]
pub use system::{BUFFER_CAPACITY, LogPage, LogStats, LogSystem, ServiceLogPage};
