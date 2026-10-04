//! 系统状态采集：复用一份 `sysinfo::System` 读取 CPU、内存与运行时间。

use std::sync::Mutex;

use sysinfo::System;

use crate::protocol::responses::SystemStatus;

/// 系统服务：复用一份 `sysinfo::System`，提供状态采集与电源 / 会话控制。
#[derive(Debug)]
pub struct SystemService {
    system: Mutex<System>,
}

impl Default for SystemService {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemService {
    /// 创建服务并初始化一份全量系统信息。
    pub fn new() -> Self {
        Self {
            system: Mutex::new(System::new_all()),
        }
    }

    /// 采集当前 CPU 占用率、内存使用与系统运行时间。
    pub fn get_status(&self) -> SystemStatus {
        let mut system = crate::lock_unpoisoned(&self.system);
        system.refresh_all();

        let cpu_usage = system.global_cpu_usage();
        let memory_total = system.total_memory();
        let memory_used = system.used_memory();
        let memory_percent = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };
        let uptime_seconds = System::uptime();

        SystemStatus {
            cpu_usage,
            memory_total,
            memory_used,
            memory_percent,
            uptime_seconds,
        }
    }
}
