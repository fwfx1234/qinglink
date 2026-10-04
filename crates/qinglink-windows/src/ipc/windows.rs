//! Windows 实现：所有内核可见存储的存活期都长于 I/O 完成。
//!
//! 本文件只做子模块声明与重导出，公开路径保持为
//! `qinglink_windows::ipc::<符号>`。实现按职责拆在同目录子模块中：
//!
//! - `connection`：已连接管道的帧读写与 JSON 转换（`PipeConnection`）；
//! - `overlapped`：`OVERLAPPED` 挂起 I/O 的状态机、取消与内核可见存储的生命周期；
//! - `request`：一次请求-应答往返；
//! - `connect`：客户端连接建立与有界重试；
//! - `pipe_path`：本地管道路径校验；
//! - `win32`：事件句柄、等待与截止时间预算的 Win32 封装。

mod connect;
mod connection;
mod overlapped;
mod pipe_path;
mod request;
mod win32;

pub use connection::PipeConnection;

pub(super) use request::request;
