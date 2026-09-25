// === 窗口管理 ===

/// 窗口信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowInfo {
    pub hwnd: usize,
    pub title: String,
    pub pid: u32,
    pub exe_path: Option<String>,
    pub is_visible: bool,
    pub is_foreground: bool,
    pub is_fullscreen: bool,
    pub is_topmost: bool,
}

/// 一次性枚举与 Alt+Tab 一致的顶层窗口（不轮询，调用完释放）。
pub fn enum_windows() -> Vec<WindowInfo> {
    use std::sync::Mutex;

    let results: Arc<Mutex<Vec<WindowInfo>>> = Arc::new(Mutex::new(Vec::new()));

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let results = &*(lparam.0 as *const Arc<Mutex<Vec<WindowInfo>>>);
        if is_alt_tab_window(hwnd) {
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let exe_path = if pid != 0 {
                let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
                if let Ok(handle) = process {
                    if !handle.is_invalid() {
                        let mut path_buf = [0u16; 260];
                        let len = GetModuleFileNameExW(Some(handle), None, &mut path_buf);
                        let _ = CloseHandle(handle);
                        if len > 0 {
                            Some(String::from_utf16_lossy(&path_buf[..len as usize]))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            let hwnd_ptr = hwnd.0 as usize;
            let is_fullscreen = is_fullscreen_window(hwnd);
            let is_topmost = is_topmost_window(hwnd);
            lock_unpoisoned(results).push(WindowInfo {
                hwnd: hwnd_ptr,
                title,
                pid,
                exe_path,
                is_visible: true,
                is_foreground: false,
                is_fullscreen,
                is_topmost,
            });
        }
        windows::core::BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(enum_callback), LPARAM(&results as *const _ as isize));
    }

    let mut windows = Arc::try_unwrap(results)
        .unwrap_or_else(|_| unreachable!("window enumeration callbacks have completed"))
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // 统一标记前台窗口
    let foreground = unsafe { GetForegroundWindow().0 as usize };
    for w in &mut windows {
        if w.hwnd == foreground {
            w.is_foreground = true;
        }
    }

    windows
}

/// Fast list variant used by the mobile window switcher.  It deliberately
/// avoids opening every target process and avoids DWM/monitor queries; those
/// calls dominate list latency when many applications are open.
pub fn enum_windows_light() -> Vec<WindowInfo> {
    use std::sync::Mutex;

    let results: Arc<Mutex<Vec<WindowInfo>>> = Arc::new(Mutex::new(Vec::new()));

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let results = &*(lparam.0 as *const Arc<Mutex<Vec<WindowInfo>>>);
        if is_alt_tab_window(hwnd) {
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            lock_unpoisoned(results).push(WindowInfo {
                hwnd: hwnd.0 as usize,
                title,
                pid,
                exe_path: None,
                is_visible: true,
                is_foreground: false,
                is_fullscreen: false,
                // TOPMOST is a window style bit and is cheap to query; keep
                // it in the light representation so Session 0 callers do
                // not lose the user's pin state through the session agent.
                is_topmost: is_topmost_window(hwnd),
            });
        }
        windows::core::BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(enum_callback), LPARAM(&results as *const _ as isize));
    }
    let mut windows = Arc::try_unwrap(results)
        .unwrap_or_else(|_| unreachable!("window enumeration callbacks have completed"))
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let foreground = unsafe { GetForegroundWindow().0 as usize };
    for window in &mut windows {
        window.is_foreground = window.hwnd == foreground;
    }
    windows
}

/// Applies the window traits that determine whether a top-level window is an
/// Alt+Tab candidate.  This avoids exposing helper, owned, and off-desktop
/// windows that happen to be visible to `EnumWindows`.
fn is_alt_tab_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }

        let has_owner = GetWindow(hwnd, GW_OWNER)
            .map(|owner| !owner.0.is_null())
            .unwrap_or(false);
        if has_owner && ex_style & WS_EX_APPWINDOW.0 == 0 {
            return false;
        }

        !is_window_cloaked(hwnd)
    }
}

fn is_window_cloaked(hwnd: HWND) -> bool {
    unsafe {
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
    }
}

/// 激活指定窗口
pub fn focus_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);

        // The WebSocket handler runs on a Tokio worker thread. Ensure it has
        // a message queue before using AttachThreadInput with GUI threads.
        let mut message = MSG::default();
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
        let caller_thread = GetCurrentThreadId();
        let foreground = GetForegroundWindow();
        let foreground_thread = if foreground.0.is_null() {
            0
        } else {
            GetWindowThreadProcessId(foreground, None)
        };
        let target_thread = GetWindowThreadProcessId(hwnd, None);
        let attached_foreground = foreground_thread != 0
            && foreground_thread != caller_thread
            && AttachThreadInput(caller_thread, foreground_thread, true).as_bool();
        let attached_target = target_thread != 0
            && target_thread != caller_thread
            && target_thread != foreground_thread
            && AttachThreadInput(caller_thread, target_thread, true).as_bool();

        // BringWindowToTop can fail for windows owned by a different thread or
        // integrity level even though SetForegroundWindow still succeeds.
        // Treat it as best-effort instead of aborting the whole activation.
        let _ = BringWindowToTop(hwnd);
        let activated = SetForegroundWindow(hwnd).as_bool();

        if attached_target {
            let _ = AttachThreadInput(caller_thread, target_thread, false);
        }
        if attached_foreground {
            let _ = AttachThreadInput(caller_thread, foreground_thread, false);
        }

        if !activated && GetForegroundWindow() != hwnd {
            anyhow::bail!("Windows 拒绝激活目标窗口");
        }
    }
    Ok(())
}

/// Wait for the first visible top-level window owned by a process and activate it.
/// GUI applications often create their process before creating the actual window.
pub fn focus_process(pid: u32) -> anyhow::Result<()> {
    const ATTEMPTS: u32 = 40;
    const INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

    for _ in 0..ATTEMPTS {
        if let Some(window) = enum_windows_light()
            .into_iter()
            .find(|window| window.pid == pid && window.is_visible)
        {
            return focus_window(window.hwnd);
        }
        std::thread::sleep(INTERVAL);
    }

    anyhow::bail!("进程 {pid} 未创建可激活的窗口")
}

/// 最小化窗口
pub fn minimize_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWMINIMIZED);
    }
    Ok(())
}

/// 最大化窗口
pub fn maximize_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWMAXIMIZED);
    }
    Ok(())
}

/// 还原窗口
pub fn restore_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
    }
    Ok(())
}

/// 关闭窗口。
///
/// 优先发送 `WM_CLOSE`，让应用自行优雅退出。刚启动的游戏等应用在初始化
/// 期间常处于忙等待或自绘消息循环，无法及时处理关闭消息，导致"关闭窗口"
/// 看起来无效；因此短暂等待后若窗口仍然存在，就强制结束其进程，行为与
/// 任务管理器的"结束任务"一致。
pub fn close_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))?;
    }

    // 给应用约 1.5 秒响应 WM_CLOSE；若窗口仍在则回退到结束进程。
    const GRACE_STEPS: u32 = 15;
    const GRACE_STEP: std::time::Duration = std::time::Duration::from_millis(100);
    for _ in 0..GRACE_STEPS {
        std::thread::sleep(GRACE_STEP);
        if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return Ok(());
        }
    }

    if pid != 0 {
        crate::process::kill(pid)?;
    }
    Ok(())
}

/// 按当前可见窗口的 z-order 切换前台窗口，行为接近 Alt+Tab。
pub fn switch_window(direction: WindowSwitchDirection) -> anyhow::Result<()> {
    let windows = enum_windows();
    if windows.is_empty() {
        anyhow::bail!("No switchable windows");
    }

    let foreground = unsafe { GetForegroundWindow().0 as usize };
    let current_index = windows.iter().position(|w| w.hwnd == foreground);
    let target_index = match direction {
        WindowSwitchDirection::Next => current_index.map_or(0, |i| (i + 1) % windows.len()),
        WindowSwitchDirection::Previous => current_index.map_or(windows.len() - 1, |i| {
            if i == 0 { windows.len() - 1 } else { i - 1 }
        }),
    };

    focus_window(windows[target_index].hwnd)
}

/// 移动/调整窗口大小
pub fn move_window(hwnd: usize, x: i32, y: i32, width: u32, height: u32) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            width as i32,
            height as i32,
            SWP_NOZORDER | SWP_SHOWWINDOW,
        )?;
    }
    Ok(())
}

/// 设置窗口置顶/取消置顶
pub fn set_always_on_top(hwnd: usize, enable: bool) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    let z_order = if enable { HWND_TOPMOST } else { HWND_NOTOPMOST };
    unsafe {
        SetWindowPos(hwnd, Some(z_order), 0, 0, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW)?;
    }
    Ok(())
}

fn checked_hwnd(raw: usize) -> anyhow::Result<HWND> {
    let hwnd = HWND(raw as *mut _);
    unsafe {
        anyhow::ensure!(
            !hwnd.0.is_null() && IsWindow(Some(hwnd)).as_bool(),
            "窗口已不存在"
        );
    }
    Ok(hwnd)
}

/// 检测窗口是否置顶
fn is_topmost_window(hwnd: HWND) -> bool {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
        use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
        let ex_style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(hwnd, GWL_EXSTYLE);
        (ex_style as u32 & WS_EX_TOPMOST.0) != 0
    }
}

/// 检测窗口是否全屏
fn is_fullscreen_window(hwnd: HWND) -> bool {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).0 == 0 {
            return false;
        }
        rect.left == info.rcMonitor.left
            && rect.top == info.rcMonitor.top
            && rect.right == info.rcMonitor.right
            && rect.bottom == info.rcMonitor.bottom
    }
}

pub fn is_foreground_fullscreen() -> bool {
    unsafe { is_fullscreen_window(GetForegroundWindow()) }
}

pub fn search_installed_apps(query: &str) -> Vec<AppInfo> {
    use std::fs;

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let _com = ComApartment::init().ok();

    let dirs = [
        format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            std::env::var("ProgramData").unwrap_or_default()
        ),
        format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            std::env::var("APPDATA").unwrap_or_default()
        ),
    ];

    for dir in &dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_lnk = name.to_lowercase().ends_with(".lnk");
                        let display_name = if is_lnk {
                            name[..name.len() - 4].to_string()
                        } else {
                            name.clone()
                        };

                        if query.is_empty() || display_name.to_lowercase().contains(&query_lower) {
                            let resolved_path = if is_lnk {
                                resolve_lnk_target(&path).0
                            } else {
                                path.to_string_lossy().to_string()
                            };

                            results.push(AppInfo {
                                name: display_name,
                                path: resolved_path,
                            });
                        }
                    } else if metadata.is_dir()
                        && let Ok(sub) = fs::read_dir(&path)
                    {
                        for sub_entry in sub.flatten() {
                            if sub_entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                                let name = sub_entry.file_name().to_string_lossy().to_string();
                                let is_lnk = name.to_lowercase().ends_with(".lnk");
                                let display_name = if is_lnk {
                                    name[..name.len() - 4].to_string()
                                } else {
                                    name.clone()
                                };

                                if query.is_empty()
                                    || display_name.to_lowercase().contains(&query_lower)
                                {
                                    let resolved_path = if is_lnk {
                                        resolve_lnk_target(&sub_entry.path()).0
                                    } else {
                                        sub_entry.path().to_string_lossy().to_string()
                                    };

                                    results.push(AppInfo {
                                        name: display_name,
                                        path: resolved_path,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 过滤掉快捷方式指向的已失效目标路径，避免启动器展示无法运行的条目。
    results.retain(|app| std::path::Path::new(&app.path).exists());
    results.truncate(100);
    results
}

/// 解析 .lnk 快捷方式获取目标路径和原始参数字符串。
fn resolve_lnk_target(path: &std::path::Path) -> (String, Option<String>) {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink};
    use windows::core::{Interface, PCWSTR};

    let fallback = || (path.to_string_lossy().to_string(), None);

    unsafe {
        let link: IShellLinkW = match CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) {
            Ok(link) => link,
            Err(_) => return fallback(),
        };
        let persist: IPersistFile = match link.cast() {
            Ok(persist) => persist,
            Err(_) => return fallback(),
        };
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).is_err() {
            return fallback();
        }

        let mut target_buf = vec![0u16; 32768];
        let _ = link.GetPath(&mut target_buf, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32);
        let target = read_wide_string(&target_buf)
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let mut args_buf = vec![0u16; 32768];
        let arguments = if link.GetArguments(&mut args_buf).is_ok() {
            read_wide_string(&args_buf)
        } else {
            None
        };

        (target, arguments)
    }
}

/// Read a null-terminated UTF-16 buffer, returning `None` when empty.
fn read_wide_string(buffer: &[u16]) -> Option<String> {
    let len = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    (len > 0).then(|| String::from_utf16_lossy(&buffer[..len]))
}

/// COM 公寓线程初始化辅助
struct ComApartment {
    initialized: bool,
}

impl ComApartment {
    fn init() -> windows::core::Result<Self> {
        let result = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        if result.is_ok() {
            return Ok(Self { initialized: true });
        }
        if result == windows::Win32::Foundation::RPC_E_CHANGED_MODE {
            // The current thread already belongs to another valid COM
            // apartment. It remains usable, but must not be uninitialized here.
            return Ok(Self { initialized: false });
        }
        Err(windows::core::Error::from(result))
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { windows::Win32::System::Com::CoUninitialize() };
        }
    }
}

/// Set process priority (normal, high, low, etc.)
pub fn set_priority(pid: u32, priority: &str) -> anyhow::Result<()> {
    crate::process::set_priority(pid, priority)
}
