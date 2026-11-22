// A release build is the tray-only product: it must never flash a console
// window. A debug build is the opposite: `cargo run --bin QingLink` has to show
// its logs in the launching terminal, which only works when the process is a
// console-subsystem image that inherits the parent's standard handles.
//
// The earlier approach — a GUI-subsystem image in every build, plus an explicit
// `AttachConsole(ATTACH_PARENT_PROCESS)` and `SetStdHandle(CONOUT$)` — was
// measured to lose every record: re-pointing the standard handles to the console
// discards the pipes `cargo run` reads, and in a low-integrity session both
// `AttachConsole` and `CreateFileW("CONOUT$")` fail, leaving stderr unusable.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! QingLink 桌面端伴随程序。
//!
//! 进程只做五件事，别的都不做：
//!
//! 1. 持有通知区域图标——没有打开浏览器时，这是触达它的唯一入口；
//! 2. 在手机发起配对时弹出配对提示窗口（同样不依赖浏览器）；
//! 3. 在 `127.0.0.1:3722` 上提供管理界面及其 API；
//! 4. 采集、推流并持久化日志；
//! 5. 保持 `QingLinkService` 运行，并通过已授权的本地命名管道把管理命令转发给它。
//!
//! 所有业务数据都在服务里。见 `docs/architecture.md`。
//!
//! # 模块划分
//!
//! 本文件只保留入口、进程起始时刻与启动顺序的接线，实现按职责分在下面几个
//! 子模块里：
//!
//! - [`cli`]：命令行模式分发（提权辅助进程、`--check-service` 与正常启动）；
//! - [`bootstrap`]：启动装配（数据目录 → 日志 → 数据库 → Web → 托盘 → 弹窗 → 租约）；
//! - [`main_loop`]：主循环（HostCommand、托盘事件、配对弹窗、tooltip 同步、消息泵）；
//! - [`exit`]：退出序列（通知浏览器 → 停 IPC → 租约 → 数据库 → 日志 flush）；
//! - [`startup_trace`]：`QINGLINK_STARTUP_TRACE` 面包屑；
//! - [`pairing`]：手机发起配对时的桌面端弹窗（Win32 窗口 + 状态机）；
//! - [`admin_ui`]：管理界面端口 / URL 与"交给默认浏览器"。

mod admin_ui;
mod bootstrap;
mod cli;
mod exit;
mod input_install;
mod logging;
mod main_loop;
mod pairing;
mod preferences;
mod self_check;
mod service;
mod service_lifecycle;
mod startup;
mod startup_trace;
mod tray;
mod updater;
mod web;

use std::sync::OnceLock;
use std::time::Instant;

use anyhow::Result;

use crate::startup_trace::trace_step;

/// When `main` started, as opposed to when the web server did.
///
/// `/health` reports uptime so a caller can tell "just started" from "running a
/// while". Measuring from web-server creation was wrong: migration, the secure
/// input probe, the `--restart-parent` wait and the tray probe all happen
/// first, so a healthy process reported a few seconds of uptime.
static STARTED_AT: OnceLock<Instant> = OnceLock::new();

fn main() {
    if let Err(error) = run() {
        // The GUI subsystem does not reliably show Rust's implicit Result error
        // report. Keep the concrete startup cause visible in the launching
        // terminal, where `cargo run` users can act on it.
        eprintln!("QingLink 启动失败：{error:#}");
        std::process::exit(1);
    }
}

/// 装配顺序：命令行分流 → 启动序列 → 主循环 → 退出序列。
fn run() -> Result<()> {
    let started_at = *STARTED_AT.get_or_init(Instant::now);
    trace_step(started_at, "main entered");

    let arguments: Vec<String> = std::env::args().collect();

    let options = match cli::classify(&arguments, started_at)? {
        cli::Preflight::Done(result) => return result,
        cli::Preflight::Run(options) => options,
    };

    // 已有实例在运行或诊断命令已完成时，装配不会返回桌面端。
    let Some(mut desktop) = bootstrap::start(&options, started_at)? else {
        return Ok(());
    };

    let request = main_loop::run(&mut desktop);
    exit::shutdown(desktop, request);
    Ok(())
}
