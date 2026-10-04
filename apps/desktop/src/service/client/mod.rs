//! 服务管道上的请求-应答关联客户端。
//!
//! 设计要点：
//!
//! * 由一条专用 OS 线程独占管道。命名管道 I/O 是阻塞式的，而 Web 服务器跑在
//!   Tokio 运行时上，两者不能共用线程。
//! * 每次调用都带一个 `oneshot` 应答通道。调用方因此拿到的是**自己那次**的应答，
//!   而不是在共享的"最新响应"槽上互相抢。
//! * 变更绝不重放。一次超时的重命名可能仍已被服务执行，所以只告诉调用方请求失败，
//!   由下一份快照重建真相。
//! * 队列有界。服务不可用时调用快速失败，而不是堆积起来几分钟后才执行。
//!
//! 子模块按职责划分：
//!
//! * [`state`]：链路状态、快照视图与事件类型，含命令能力门控。
//! * [`inner`]：worker 线程与调用方共享的内核（状态、队列、唤醒计数、事件总线）。
//! * [`handle`]：交给 Web 层的调用句柄 [`ServiceClient`]。
//! * [`runtime`]：worker 线程的生命周期（启动、停机、退出时释放队列）。
//! * [`transport`]：阻塞式传输抽象与命名管道实现。
//! * [`envelope`]：请求/响应信封编解码、协议版本校验与能力协商。
//! * [`worker`]：worker 线程主循环（排空队列、重连退避、驱动快照刷新）。
//! * [`snapshot`]：快照刷新、状态迁移与变更事件派发。

mod envelope;
mod handle;
mod inner;
mod runtime;
mod snapshot;
mod state;
mod transport;
mod worker;

pub use handle::ServiceClient;
pub use runtime::ServiceRuntime;
pub use state::{ServiceEvent, ServiceLinkState, ServiceState, ServiceStatus};
