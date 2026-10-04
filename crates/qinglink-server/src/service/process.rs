//! 进程服务：枚举并分页进程列表，并提供终止、挂起、恢复与优先级调整操作。

use std::collections::HashSet;
use std::time::{Duration, Instant};

use sysinfo::System;

use crate::platform::{ProcessActions, get_process_info};
use crate::protocol::responses::{ForegroundResponse, ProcessInfo, ProcessListResponse};

/// 进程快照缓存 TTL：`refresh_all` + 全量窗口枚举成本高，1 秒内复用。
const SNAPSHOT_TTL: Duration = Duration::from_secs(1);

/// 一次系统枚举的快照，包含过滤/排序/分页所需的全部原始数据。
#[derive(Debug)]
struct ProcessSnapshot {
    processes: Vec<ProcessInfo>,
    foreground_pid: Option<u32>,
    visible_pids: HashSet<u32>,
}

/// 进程管理器：缓存一次系统枚举结果，并对外提供进程查询与操作。
#[derive(Debug)]
pub struct ProcessManager {
    system: System,
    snapshot: Option<(Instant, ProcessSnapshot)>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    /// 创建管理器并立即完成一次全量系统枚举。
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            snapshot: None,
        }
    }

    /// 使下一次查询强制重新枚举系统。
    pub fn refresh(&mut self) {
        self.snapshot = None;
    }

    /// 仅在快照过期时执行昂贵的系统枚举。
    fn refresh_snapshot_if_stale(&mut self) {
        if self
            .snapshot
            .as_ref()
            .is_some_and(|(taken_at, _)| taken_at.elapsed() < SNAPSHOT_TTL)
        {
            return;
        }
        self.system.refresh_all();
        let foreground_pid = crate::platform::get_foreground_window_info()
            .ok()
            .map(|p| p.pid);
        let visible_pids: HashSet<u32> = crate::platform::enum_windows()
            .into_iter()
            .filter(|window| window.is_visible)
            .map(|window| window.pid)
            .collect();
        let processes: Vec<ProcessInfo> = self
            .system
            .processes()
            .values()
            .map(get_process_info)
            .collect();
        self.snapshot = Some((
            Instant::now(),
            ProcessSnapshot {
                processes,
                foreground_pid,
                visible_pids,
            },
        ));
    }

    /// 查询进程列表：支持关键词搜索、用户/全部过滤与分页。
    pub fn list_processes(
        &mut self,
        search: Option<&str>,
        page: usize,
        page_size: usize,
        filter: Option<&str>,
    ) -> ProcessListResponse {
        self.refresh_snapshot_if_stale();
        let Some((_, snapshot)) = self.snapshot.as_ref() else {
            return ProcessListResponse {
                processes: Vec::new(),
                total: 0,
                page,
                page_size,
                has_more: false,
            };
        };

        let search_lower = search.map(|s| s.to_lowercase());
        let filter_val = filter.unwrap_or("user");
        let show_all = filter_val == "all";

        let foreground_pid = snapshot.foreground_pid;
        let visible_pids = &snapshot.visible_pids;

        let mut processes: Vec<ProcessInfo> = snapshot
            .processes
            .iter()
            .filter(|p| {
                // 搜索过滤
                let search_match = search_lower.as_ref().is_none_or(|q| {
                    p.name.to_lowercase().contains(q)
                        || p.pid.to_string().contains(q)
                        || p.path
                            .as_ref()
                            .is_some_and(|path| path.to_lowercase().contains(q))
                });
                if !search_match {
                    return false;
                }

                // 用户进程过滤：排除系统进程
                if !show_all && Self::is_system_process(p) {
                    return false;
                }

                true
            })
            .cloned()
            .collect();

        // 与桌面任务管理器一致的分组：应用（有可见窗口）/ 后台 / Windows 进程。
        for process in &mut processes {
            process.group = if visible_pids.contains(&process.pid) {
                "app".to_string()
            } else if Self::is_system_process(process) {
                "system".to_string()
            } else {
                "background".to_string()
            };
        }

        // Mobile control surfaces put the actionable processes first.  Keep a
        // deterministic order within each group so pagination does not jump.
        processes.sort_by(|a, b| {
            let activity = |p: &ProcessInfo| {
                if foreground_pid == Some(p.pid) {
                    0
                } else if visible_pids.contains(&p.pid) {
                    1
                } else {
                    2
                }
            };
            activity(a)
                .cmp(&activity(b))
                .then_with(|| b.cpu_usage.total_cmp(&a.cpu_usage))
                .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
                .then_with(|| b.started_at.cmp(&a.started_at))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.pid.cmp(&b.pid))
        });

        let total = processes.len();

        let page_size = page_size.clamp(1, 200);
        let start = page.saturating_mul(page_size);
        let paged = if start < total {
            processes[start..(start + page_size).min(total)].to_vec()
        } else {
            Vec::new()
        };

        ProcessListResponse {
            processes: paged,
            total,
            page,
            page_size,
            has_more: start.saturating_add(page_size) < total,
        }
    }

    /// 判断是否为系统进程（用于用户进程过滤）
    fn is_system_process(p: &ProcessInfo) -> bool {
        // 系统 idle 和 system 进程
        if p.pid <= 4 {
            return true;
        }

        // 检查路径是否在系统目录
        if let Some(ref path) = p.path {
            let path_lower = path.to_lowercase();
            // 排除系统目录中的进程（但保留有用户交互的）
            if path_lower.starts_with("c:\\windows\\system32")
                || path_lower.starts_with("c:\\windows\\syswow64")
                || path_lower.starts_with("c:\\windows\\winsxs")
                || path_lower.starts_with("c:\\windows\\microsoft.net")
                || path_lower.starts_with("c:\\windows\\")
            {
                // 保留 cmd, powershell 等用户工具
                let name_lower = p.name.to_lowercase();
                let user_tools = [
                    "cmd.exe",
                    "powershell.exe",
                    "pwsh.exe",
                    "wt.exe",
                    "notepad.exe",
                    "mspaint.exe",
                    "calc.exe",
                    "snippingtool.exe",
                    "explorer.exe",
                ];
                if user_tools.iter().any(|tool| name_lower.contains(tool)) {
                    return false;
                }
                return true;
            }
        }

        false
    }

    /// 获取当前前台窗口及其所属进程信息。
    ///
    /// # Errors
    /// 无法获取前台窗口 / 进程信息（例如没有前台窗口或系统调用失败）时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    pub fn get_foreground(&self) -> anyhow::Result<ForegroundResponse> {
        crate::platform::get_foreground_window_info()
    }

    /// 强制终止指定 PID 的进程。
    ///
    /// # Errors
    /// 打开进程或调用终止进程的系统接口失败（如 PID 不存在、权限不足）时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    pub fn kill_process(&self, pid: u32) -> anyhow::Result<()> {
        ProcessActions::kill(pid)
    }

    /// 挂起（暂停）指定 PID 的进程。
    ///
    /// # Errors
    /// 打开进程或调用挂起进程的系统接口失败（如 PID 不存在、权限不足）时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    pub fn suspend_process(&self, pid: u32) -> anyhow::Result<()> {
        ProcessActions::suspend(pid)
    }

    /// 恢复先前被挂起的进程。
    ///
    /// # Errors
    /// 打开进程或调用恢复进程的系统接口失败（如 PID 不存在、权限不足）时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    pub fn resume_process(&self, pid: u32) -> anyhow::Result<()> {
        ProcessActions::resume(pid)
    }

    /// 调整指定进程的优先级（如 `high`、`normal`、`low`）。
    ///
    /// # Errors
    /// `priority` 不是受支持的优先级名称，或打开 / 调整进程优先级失败
    /// （如 PID 不存在、权限不足）时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn set_process_priority(&self, pid: u32, priority: &str) -> anyhow::Result<()> {
        crate::platform::set_priority(pid, priority)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn list_processes_returns_results() {
        let mut pm = ProcessManager::new();
        let result = pm.list_processes(None, 0, 50, None);

        assert!(!result.processes.is_empty(), "应返回至少一个进程");
        assert!(result.total > 0, "总进程数应大于 0");
        assert_eq!(result.page, 0);
        assert_eq!(result.page_size, 50);
    }

    #[test]
    fn list_processes_pagination() {
        let mut pm = ProcessManager::new();
        let page1 = pm.list_processes(None, 0, 10, None);
        let page2 = pm.list_processes(None, 1, 10, None);

        assert_eq!(page1.processes.len(), 10);
        assert_eq!(page2.processes.len(), 10);

        // 两页的进程不应完全相同
        if !page1.processes.is_empty() && !page2.processes.is_empty() {
            let first_pid_page1 = page1.processes[0].pid;
            let first_pid_page2 = page2.processes[0].pid;
            assert_ne!(first_pid_page1, first_pid_page2, "不同页应有不同的进程");
        }
    }

    #[test]
    fn list_processes_search_filter() {
        let mut pm = ProcessManager::new();
        let result = pm.list_processes(Some("svchost"), 0, 50, Some("all"));

        // 搜索 "svchost" 应该能找到 Windows 服务宿主进程
        if !result.processes.is_empty() {
            for proc in &result.processes {
                let name_match = proc.name.to_lowercase().contains("svchost");
                let path_match = proc
                    .path
                    .as_ref()
                    .map(|p| p.to_lowercase().contains("svchost"))
                    .unwrap_or(false);
                assert!(name_match || path_match, "搜索结果应包含关键词 'svchost'");
            }
        }
    }

    #[test]
    fn list_processes_info_complete() {
        let mut pm = ProcessManager::new();
        let result = pm.list_processes(None, 0, 10, None);

        for proc in &result.processes {
            assert!(proc.pid > 0, "PID 应大于 0");
            assert!(!proc.name.is_empty(), "进程名不应为空");
        }
    }

    #[test]
    fn get_foreground_returns_valid_info() {
        let pm = ProcessManager::new();
        let result = pm.get_foreground();

        if let Ok(info) = result {
            assert!(info.pid > 0, "前台窗口 PID 应大于 0");
            assert!(!info.process_name.is_empty(), "前台进程名不应为空");
        }
    }
}
