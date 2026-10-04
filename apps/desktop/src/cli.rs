//! 命令行模式分发。
//!
//! 参数定义在 [`crate::startup::StartupOptions`]；这里只决定"这次启动是哪种模式、
//! 由谁执行、在装配任何子系统之前还是之后退出"。
//!
//! 提权辅助进程（`--input-service-control`）与服务探针（`--check-service`）都必须
//! 在任何子系统之前完成，因此和正常启动一起在这里分流。

use std::time::Instant;

use anyhow::Result;

use crate::input_install;
use crate::self_check;
use crate::startup::StartupOptions;
use crate::startup_trace::trace_step;

/// 装配子系统之前的启动分流。
pub enum Preflight {
    /// 命令已经执行完毕，其返回值就是进程结果，不再启动任何东西。
    Done(Result<()>),
    /// 正常启动。
    Run(StartupOptions),
}

/// 判定本次启动走哪条路径。
pub fn classify(arguments: &[String], started_at: Instant) -> Result<Preflight> {
    // Elevated helper invocations run and exit before anything else is set up.
    if let Some(result) = input_install::handle_helper_command(arguments) {
        trace_step(started_at, "helper command dispatched");
        return Ok(Preflight::Done(result));
    }

    let options = StartupOptions::parse(arguments)?;
    trace_step(started_at, "startup options parsed");

    // The service probe talks to the pipe and prints its own report, so it runs
    // before any subsystem is created. Unlike `--self-check` it deliberately
    // touches the service, which is the whole point of the command.
    if options.check_service {
        trace_step(started_at, "running the service probe");
        return Ok(Preflight::Done(self_check::run_service_probe()));
    }

    Ok(Preflight::Run(options))
}
