#![allow(clippy::unwrap_used, clippy::zombie_processes)]

/// 功能验证测试
///
/// 这些测试验证系统操作是否真正生效，不仅仅检查返回值。
/// 操作类测试（启动/终止进程等）标记为 #[ignore]，需手动执行：
///
/// ```bash
/// cargo test -p qinglink-server --test functional_test -- --ignored
/// ```
///
/// 信息查询类测试（系统状态、进程列表等）可直接运行：
///
/// ```bash
/// cargo test -p qinglink-server --test functional_test
/// ```
use std::process::Command;
use std::time::Duration;

use qinglink_server::service::process::ProcessManager;
use qinglink_server::service::system::SystemService;

// ============================================================
// 进程管理功能测试 — 验证操作后系统状态真正改变
// ============================================================

/// 辅助：用独立方法验证进程是否存在（不依赖被测试的代码）
fn process_exists_in_system(pid: u32) -> bool {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    sys.process(sysinfo::Pid::from_u32(pid)).is_some()
}

/// 辅助：获取进程中匹配名称的所有 PID
fn find_processes_by_name(name: &str) -> Vec<u32> {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    sys.processes()
        .values()
        .filter(|p| {
            p.name()
                .to_string_lossy()
                .to_lowercase()
                .contains(&name.to_lowercase())
        })
        .map(|p| p.pid().as_u32())
        .collect()
}

#[test]
#[ignore = "会启动真实进程，需手动执行"]
fn functional_launch_notepad_then_verify_in_process_list() {
    // === 操作前：确认 notepad 不在运行 ===
    let before_pids = find_processes_by_name("notepad");
    for pid in &before_pids {
        let pm = ProcessManager::new();
        let _ = pm.kill_process(*pid);
    }
    std::thread::sleep(Duration::from_millis(500));

    // === 执行操作 ===
    let result = qinglink_server::platform::launch_app(r"C:\Windows\System32\notepad.exe", &[]);
    assert!(result.is_ok(), "launch_app 应返回 Ok: {:?}", result);

    // === 操作后：等待并验证 ===
    std::thread::sleep(Duration::from_secs(2));

    // 方法 1：通过被测试的 ProcessManager 验证
    let mut pm = ProcessManager::new();
    let list_result = pm.list_processes(Some("notepad"), 0, 50, Some("all"));
    assert!(
        !list_result.processes.is_empty(),
        "启动后进程列表应包含 notepad"
    );

    // 方法 2：通过独立 sysinfo 交叉验证
    let sys_pids = find_processes_by_name("notepad");
    assert!(
        !sys_pids.is_empty(),
        "独立 sysinfo 验证：系统进程列表应包含 notepad"
    );

    // 方法 3：验证进程 PID 有效且信息完整
    for proc in &list_result.processes {
        assert!(proc.pid > 0, "PID 应大于 0");
        assert!(!proc.name.is_empty(), "进程名不应为空");
        assert!(
            proc.name.to_lowercase().contains("notepad"),
            "进程名应包含 notepad"
        );
    }

    // === 清理 ===
    for pid in &sys_pids {
        let _ = pm.kill_process(*pid);
    }
}

#[test]
#[ignore = "会启动并终止真实进程，需手动执行"]
fn functional_kill_process_then_verify_gone() {
    // === 操作前：启动测试进程 ===
    let child = Command::new(r"C:\Windows\System32\notepad.exe")
        .spawn()
        .expect("启动测试进程失败");

    let test_pid = child.id();
    std::thread::sleep(Duration::from_secs(2));

    // 验证进程确实在运行
    assert!(
        process_exists_in_system(test_pid),
        "操作前：测试进程应在运行"
    );

    // === 执行操作 ===
    let pm = ProcessManager::new();
    let kill_result = pm.kill_process(test_pid);
    assert!(
        kill_result.is_ok(),
        "kill_process 应返回 Ok: {:?}",
        kill_result
    );

    // === 操作后：验证进程已终止 ===
    std::thread::sleep(Duration::from_secs(1));

    // 方法 1：通过独立 sysinfo 验证
    assert!(
        !process_exists_in_system(test_pid),
        "操作后：进程应已从系统消失"
    );

    // 方法 2：通过 ProcessManager 搜索验证
    let mut pm2 = ProcessManager::new();
    let search_result = pm2.list_processes(Some(&test_pid.to_string()), 0, 50, Some("all"));
    assert!(
        search_result.processes.is_empty(),
        "操作后：进程列表中不应再找到该 PID"
    );
}

#[test]
#[ignore = "会启动并挂起/恢复进程，需手动执行"]
fn functional_suspend_resume_then_verify_state() {
    // === 操作前：启动测试进程 ===
    let child = Command::new(r"C:\Windows\System32\notepad.exe")
        .spawn()
        .expect("启动测试进程失败");

    let test_pid = child.id();
    std::thread::sleep(Duration::from_secs(2));

    assert!(process_exists_in_system(test_pid), "进程应在运行");

    let pm = ProcessManager::new();

    // === 执行挂起 ===
    let suspend_result = pm.suspend_process(test_pid);
    assert!(
        suspend_result.is_ok(),
        "suspend 应返回 Ok: {:?}",
        suspend_result
    );

    std::thread::sleep(Duration::from_millis(500));

    // === 验证挂起状态（通过 sysinfo 检查 CPU 使用率接近 0）===
    {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        if let Some(proc) = sys.process(sysinfo::Pid::from_u32(test_pid)) {
            // 挂起后 CPU 使用率应为 0 或接近 0
            let cpu = proc.cpu_usage();
            assert!(cpu < 1.0, "挂起后 CPU 使用率应接近 0, 实际: {}", cpu);
        }
    }

    // === 执行恢复 ===
    let resume_result = pm.resume_process(test_pid);
    assert!(
        resume_result.is_ok(),
        "resume 应返回 Ok: {:?}",
        resume_result
    );

    std::thread::sleep(Duration::from_millis(500));

    // === 验证恢复后进程仍在 ===
    assert!(process_exists_in_system(test_pid), "恢复后进程仍应在运行");

    // === 清理 ===
    let pm2 = ProcessManager::new();
    let _ = pm2.kill_process(test_pid);
}

#[test]
#[ignore = "会修改进程优先级，需手动执行"]
fn functional_set_priority_then_verify() {
    use windows::Win32::System::Threading::{
        GetPriorityClass, OpenProcess, PROCESS_QUERY_INFORMATION,
    };

    // === 操作前：启动测试进程 ===
    let child = Command::new("cmd")
        .args(["/c", "timeout", "/t", "60", "/nobreak"])
        .spawn()
        .expect("启动测试进程失败");

    let test_pid = child.id();
    std::thread::sleep(Duration::from_secs(1));

    // 记录原始优先级
    let original_priority = unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, test_pid).unwrap();
        let priority = GetPriorityClass(handle);
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        priority
    };

    // === 执行操作：设置为高优先级 ===
    let pm = ProcessManager::new();
    let result = pm.set_process_priority(test_pid, "high");
    assert!(result.is_ok(), "set_priority 应返回 Ok: {:?}", result);

    // === 操作后：验证优先级已变更 ===
    std::thread::sleep(Duration::from_millis(500));

    let new_priority = unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, test_pid).unwrap();
        let priority = GetPriorityClass(handle);
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        priority
    };

    assert_ne!(
        original_priority, new_priority,
        "优先级应已变更: 原始={:?}, 新={:?}",
        original_priority, new_priority
    );
    assert_eq!(
        new_priority,
        windows::Win32::System::Threading::ABOVE_NORMAL_PRIORITY_CLASS.0,
        "优先级应为 ABOVE_NORMAL"
    );

    // === 清理 ===
    let pm2 = ProcessManager::new();
    let _ = pm2.kill_process(test_pid);
}

// ============================================================
// 系统信息功能测试 — 验证返回值与真实系统状态一致
// ============================================================

#[test]
fn functional_system_status_matches_sysinfo() {
    let service = SystemService::new();
    let status = service.get_status();

    // 交叉验证：使用独立 sysinfo 实例获取数据
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    // 内存总量验证（总量短时间内不变）
    let sys_total = sys.total_memory();
    assert_eq!(status.memory_total, sys_total, "内存总量应与 sysinfo 一致");

    // 已用内存验证（允许 50MB 误差，因为两次调用之间内存使用可能有波动）
    let sys_used = sys.used_memory();
    let used_diff = status.memory_used.abs_diff(sys_used);
    assert!(
        used_diff < 50 * 1024 * 1024,
        "已用内存差异应小于 50MB: 服务={}, sysinfo={}, 差异={}MB",
        status.memory_used,
        sys_used,
        used_diff / (1024 * 1024)
    );

    // 百分比计算验证（基于服务自身数据的一致性检查）
    let expected_percent = if status.memory_total > 0 {
        (status.memory_used as f64 / status.memory_total as f64) * 100.0
    } else {
        0.0
    };
    let diff = (status.memory_percent as f64 - expected_percent).abs();
    assert!(
        diff < 0.1,
        "内存百分比计算应自洽: 报告={}%, 计算={}%, 差异={}",
        status.memory_percent,
        expected_percent,
        diff
    );

    // 百分比与 sysinfo 交叉验证（允许 2% 误差）
    let sys_percent = if sys_total > 0 {
        (sys_used as f64 / sys_total as f64) * 100.0
    } else {
        0.0
    };
    let cross_diff = (status.memory_percent as f64 - sys_percent).abs();
    assert!(
        cross_diff < 2.0,
        "内存百分比与 sysinfo 差异应小于 2%: 服务={}%, sysinfo={}%",
        status.memory_percent,
        sys_percent
    );

    // 运行时间验证
    let sys_uptime = sysinfo::System::uptime();
    let uptime_diff = status.uptime_seconds.abs_diff(sys_uptime);
    assert!(
        uptime_diff < 5,
        "运行时间差异应小于 5 秒: 服务={}, sysinfo={}",
        status.uptime_seconds,
        sys_uptime
    );
}

#[test]
fn functional_mac_address_matches_system() {
    let service = SystemService::new();
    let service_mac = service.get_mac_address();

    // 交叉验证：通过 Win32 API 独立获取
    let system_macs = get_system_mac_addresses();

    match &service_mac {
        Some(service_mac) => {
            assert!(
                system_macs
                    .iter()
                    .any(|system_mac| system_mac.eq_ignore_ascii_case(service_mac)),
                "服务返回的 MAC 应属于系统适配器: 服务={}, 系统={:?}",
                service_mac,
                system_macs
            );
        }
        None => {
            assert!(
                system_macs.is_empty(),
                "系统存在可用 MAC 时服务不应返回 None: 系统={system_macs:?}"
            );
        }
    }
}

/// 独立获取系统 MAC 地址（不依赖被测试代码）
fn get_system_mac_addresses() -> Vec<String> {
    use std::alloc::{Layout, alloc, dealloc};
    use windows::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::AF_UNSPEC;

    unsafe {
        let mut size: u32 = 0;
        let result = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_PREFIX,
            None,
            None,
            &mut size,
        );
        if result != 0x000000EA && size == 0 {
            return Vec::new();
        }

        let layout = Layout::from_size_align_unchecked(size as usize, 8);
        let buffer = alloc(layout);
        if buffer.is_null() {
            return Vec::new();
        }

        let result = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_PREFIX,
            None,
            Some(buffer as *mut IP_ADAPTER_ADDRESSES_LH),
            &mut size,
        );

        if result != 0 {
            dealloc(buffer, layout);
            return Vec::new();
        }

        let mut macs = Vec::new();
        let mut adapter = buffer as *mut IP_ADAPTER_ADDRESSES_LH;
        while !adapter.is_null() {
            let physical_address = (*adapter).PhysicalAddress;
            let physical_address_length = (*adapter).PhysicalAddressLength as usize;

            if physical_address_length >= 6 {
                let mac = format!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    physical_address[0],
                    physical_address[1],
                    physical_address[2],
                    physical_address[3],
                    physical_address[4],
                    physical_address[5]
                );
                if mac != "00:00:00:00:00:00" {
                    macs.push(mac);
                }
            }
            adapter = (*adapter).Next;
        }

        dealloc(buffer, layout);
        macs
    }
}

#[test]
fn functional_foreground_window_matches_system() {
    let pm = ProcessManager::new();
    let result = pm.get_foreground();

    if let Ok(info) = result {
        // 验证 PID 对应的进程确实存在于系统
        assert!(
            process_exists_in_system(info.pid),
            "前台窗口 PID={} 对应的进程应存在于系统",
            info.pid
        );

        // 验证进程名与路径一致
        if let Some(ref path) = info.path {
            let path_name = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            assert_eq!(
                path_name.to_lowercase(),
                info.process_name.to_lowercase(),
                "进程名应与路径中的文件名一致"
            );

            // 验证路径在文件系统中存在
            assert!(
                std::path::Path::new(path).exists(),
                "前台进程路径应在文件系统中存在: {}",
                path
            );
        }
    }
}

// ============================================================
// 应用搜索功能测试 — 验证结果路径在文件系统中真实存在
// ============================================================

#[test]
fn functional_search_apps_paths_exist_on_disk() {
    let apps = qinglink_server::platform::search_installed_apps("");

    assert!(!apps.is_empty(), "应找到至少一个已安装应用");

    let mut existing_count = 0;
    for app in &apps {
        let path_exists = std::path::Path::new(&app.path).exists();
        if path_exists {
            existing_count += 1;
        }
    }

    // 至少 80% 的搜索结果路径应在磁盘上存在
    let ratio = existing_count as f64 / apps.len() as f64;
    assert!(
        ratio >= 0.8,
        "至少 80% 的应用路径应在磁盘上存在: {}/{} ({:.0}%)",
        existing_count,
        apps.len(),
        ratio * 100.0
    );
}

#[test]
fn functional_search_apps_case_insensitive() {
    let lower = qinglink_server::platform::search_installed_apps("notepad");
    let upper = qinglink_server::platform::search_installed_apps("NOTEPAD");
    let mixed = qinglink_server::platform::search_installed_apps("NoTePaD");

    // 不区分大小写的搜索应返回相同数量的结果
    assert_eq!(lower.len(), upper.len(), "大小写搜索应返回相同数量");
    assert_eq!(lower.len(), mixed.len(), "大小写搜索应返回相同数量");
}

#[test]
fn functional_search_results_contain_query() {
    let query = "win";
    let apps = qinglink_server::platform::search_installed_apps(query);

    for app in &apps {
        assert!(
            app.name.to_lowercase().contains(query),
            "搜索结果 '{}' 应包含查询 '{}'",
            app.name,
            query
        );
    }
}

// ============================================================
// 进程列表功能测试 — 验证数据完整性和准确性
// ============================================================

#[test]
fn functional_process_list_pids_match_system() {
    let mut pm = ProcessManager::new();
    let result = pm.list_processes(None, 0, 100, Some("all"));

    assert!(!result.processes.is_empty());

    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    // Processes may legitimately exit between the two snapshots. Require the
    // snapshots to substantially agree without treating normal exits as bugs.
    let still_running = result
        .processes
        .iter()
        .filter(|process| sys.process(sysinfo::Pid::from_u32(process.pid)).is_some())
        .count();
    assert!(
        still_running * 4 >= result.processes.len() * 3,
        "进程快照重合率过低: {still_running}/{}",
        result.processes.len()
    );
}

#[test]
fn functional_process_list_search_accurate() {
    let query = "explorer";
    let mut pm = ProcessManager::new();
    let result = pm.list_processes(Some(query), 0, 50, Some("all"));

    // 验证每个结果确实匹配搜索条件
    for proc in &result.processes {
        let name_match = proc.name.to_lowercase().contains(&query.to_lowercase());
        let path_match = proc
            .path
            .as_ref()
            .map(|p| p.to_lowercase().contains(&query.to_lowercase()))
            .unwrap_or(false);
        let pid_match = proc.pid.to_string() == query;

        assert!(
            name_match || path_match || pid_match,
            "搜索结果 '{}' 应匹配查询 '{}'",
            proc.name,
            query
        );
    }

    // 验证 explorer.exe 确实在结果中（Windows 系统应该有）
    if process_exists_in_system(
        find_processes_by_name("explorer")
            .first()
            .copied()
            .unwrap_or(0),
    ) {
        assert!(
            !result.processes.is_empty(),
            "explorer 在系统中运行，搜索结果不应为空"
        );
    }
}

#[test]
fn functional_process_list_info_accuracy() {
    let mut pm = ProcessManager::new();
    let result = pm.list_processes(None, 0, 10, Some("all"));

    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    // 抽样验证进程信息准确性
    for proc in result.processes.iter().take(5) {
        if let Some(sys_proc) = sys.process(sysinfo::Pid::from_u32(proc.pid)) {
            // 名称应一致
            let sys_name = sys_proc.name().to_string_lossy().to_string();
            assert_eq!(
                proc.name.to_lowercase(),
                sys_name.to_lowercase(),
                "PID={} 进程名应一致",
                proc.pid
            );

            // 内存使用量应在容差范围内（两次 sysinfo 快照之间进程内存可能
            // 发生变化，精确相等在活跃系统上无法保证）
            let mem_delta = if proc.memory_bytes > sys_proc.memory() {
                proc.memory_bytes - sys_proc.memory()
            } else {
                sys_proc.memory() - proc.memory_bytes
            };
            let tolerance = (sys_proc.memory() / 2).max(16 * 1024 * 1024);
            assert!(
                mem_delta <= tolerance,
                "PID={} 内存使用量应在容差范围内: 实际 {} vs 系统 {}",
                proc.pid,
                proc.memory_bytes,
                sys_proc.memory()
            );
        }
    }
}

#[test]
fn functional_process_list_total_consistent() {
    let mut pm = ProcessManager::new();

    // 获取总数（不分页）
    let full_result = pm.list_processes(None, 0, 10000, Some("all"));
    let reported_total = full_result.total;

    // 独立计算系统进程数
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let sys_count = sys.processes().len();

    // 总数应与系统进程数接近（允许少量差异，因为进程可能在扫描过程中创建/终止）
    let diff = reported_total.abs_diff(sys_count);
    assert!(
        diff <= 20,
        "报告的进程总数 ({}) 应与系统进程数 ({}) 接近",
        reported_total,
        sys_count
    );
}
