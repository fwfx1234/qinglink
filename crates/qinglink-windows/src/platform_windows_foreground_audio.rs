/// 查询当前前台窗口的标题、进程 ID 与可执行文件路径。
///
/// # Errors
/// 当 `GetForegroundWindow` 返回无效句柄（当前没有前台窗口）时返回
/// “No foreground window”。窗口标题或进程路径取不到时不算错误，对应字段为空。
pub fn get_foreground_window_info() -> anyhow::Result<ForegroundResponse> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_invalid() {
            anyhow::bail!("No foreground window");
        }

        // 获取窗口标题
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..len as usize]);

        // 获取进程 ID
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        // 获取进程路径
        let path = if pid != 0 {
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

        let process_name = if let Some(ref p) = path {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        Ok(ForegroundResponse {
            pid,
            hwnd: hwnd.0 as usize,
            title,
            path,
            process_name,
        })
    }
}

/// 前台窗口事件钩子的全局状态
type ForegroundHook = std::sync::Arc<dyn Fn(isize) + Send + Sync>;
static FOREGROUND_HOOK_STATE: std::sync::Mutex<Option<ForegroundHook>> =
    std::sync::Mutex::new(None);

fn set_hook_state(callback: Box<dyn Fn(isize) + Send + Sync>) {
    *lock_unpoisoned(&FOREGROUND_HOOK_STATE) = Some(std::sync::Arc::from(callback));
}

fn take_hook_state() -> Option<std::sync::Arc<dyn Fn(isize) + Send + Sync>> {
    lock_unpoisoned(&FOREGROUND_HOOK_STATE).take()
}

/// 设置前台窗口变化事件钩子
/// 当窗口变化时，调用 callback(hwnd)
/// 返回 hook_handle，需要配合 message_pump 使用
///
/// # Errors
/// `SetWinEventHook` 返回无效钩子句柄时返回“SetWinEventHook failed”，
/// 并清除刚登记的回调状态。
pub fn set_foreground_hook(
    callback: Box<dyn Fn(isize) + Send + Sync>,
) -> anyhow::Result<windows::Win32::UI::Accessibility::HWINEVENTHOOK> {
    use windows::Win32::UI::Accessibility::SetWinEventHook;
    use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_FOREGROUND;

    unsafe {
        set_hook_state(callback);

        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            0, // WINEVENT_OUTOFCONTEXT
        );

        if hook.is_invalid() {
            take_hook_state();
            anyhow::bail!("SetWinEventHook failed");
        }

        Ok(hook)
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_FOREGROUND;
    if event == EVENT_SYSTEM_FOREGROUND {
        let cb = lock_unpoisoned(&FOREGROUND_HOOK_STATE).clone();
        if let Some(cb) = cb.as_ref() {
            cb(hwnd.0 as isize);
        }
    }
}

/// 注销前台窗口切换的 WinEvent 钩子并清理回调状态。
pub fn unhook_foreground(hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK) {
    unsafe {
        use windows::Win32::UI::Accessibility::UnhookWinEvent;
        let _ = UnhookWinEvent(hook);
        take_hook_state();
    }
}

// === 音量控制 ===

fn with_endpoint_volume<T>(
    f: impl FnOnce(&windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
    };
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};

    let _com = ComApartment::init().context("初始化音量控制 COM 失败")?;
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        f(&endpoint)
    }
}

fn audio_output_name(device: &windows::Win32::Media::Audio::IMMDevice) -> anyhow::Result<String> {
    use windows::Win32::Foundation::PROPERTYKEY;
    use windows::Win32::System::Com::{STGM_READ, StructuredStorage::PropVariantClear};
    use windows::Win32::System::Variant::VT_LPWSTR;

    let friendly_name_key = PROPERTYKEY {
        fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
        pid: 14,
    };

    unsafe {
        let store = device.OpenPropertyStore(STGM_READ)?;
        let mut value = store.GetValue(&friendly_name_key)?;
        let name = if value.Anonymous.Anonymous.vt == VT_LPWSTR {
            // PROPVARIANT 拥有此缓冲区。下方的 PropVariantClear
            // 是负责释放它的唯一所有者。
            pwstr_to_string(value.Anonymous.Anonymous.Anonymous.pwszVal)
        } else {
            String::new()
        };
        let _ = PropVariantClear(&mut value);
        Ok(name)
    }
}

/// 返回活动的渲染端点，并标记控制台默认端点。
///
/// # Errors
/// COM 初始化失败（“初始化音频输出枚举 COM 失败”）、
/// `CoCreateInstance(MMDeviceEnumerator)` 失败、`EnumAudioEndpoints` 或
/// `GetCount` 失败，以及枚举过程中 `Item`/`GetId` 失败时返回错误；默认端点查询与
/// 友好名称读取失败会被忽略（名称退化为设备 ID）。
pub fn get_audio_outputs() -> anyhow::Result<AudioOutputsResponse> {
    use windows::Win32::Media::Audio::{
        DEVICE_STATE_ACTIVE, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
    };
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};

    let _com = ComApartment::init().context("初始化音频输出枚举 COM 失败")?;
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let default_id = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .ok()
            .and_then(|device| device.GetId().ok())
            .map(take_pwstr);
        let devices = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = devices.GetCount()?;
        let mut outputs = Vec::with_capacity(count as usize);
        for index in 0..count {
            let device = devices.Item(index)?;
            let id = take_pwstr(device.GetId()?);
            let name = audio_output_name(&device)
                .ok()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| id.clone());
            outputs.push(AudioOutputDevice {
                is_default: default_id.as_deref() == Some(id.as_str()),
                id,
                name,
            });
        }
        Ok(AudioOutputsResponse { devices: outputs })
    }
}

/// 将选定的端点设置为所有 Windows 渲染角色的默认值。
///
/// # Errors
/// `device_id` 去空白后为空（“Audio output device ID cannot be empty”）；
/// `get_audio_outputs()` 自身失败；设备 ID 不在活动渲染端点列表中（“Audio output
/// device not found or inactive”）；COM 初始化失败（“初始化音频输出切换 COM 失败”）；
/// `CoCreateInstance(IPolicyConfig)` 失败；或任一
/// `set_default_endpoint(Console/Multimedia/Communications)` 失败时返回错误。
pub fn set_audio_output(device_id: &str) -> anyhow::Result<()> {
    use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia};
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};

    if device_id.trim().is_empty() {
        anyhow::bail!("Audio output device ID cannot be empty");
    }
    let outputs = get_audio_outputs()?;
    if !outputs.devices.iter().any(|device| device.id == device_id) {
        anyhow::bail!("Audio output device not found or inactive");
    }

    let _com = ComApartment::init().context("初始化音频输出切换 COM 失败")?;
    let device_id_wide = to_wide_z(device_id);
    unsafe {
        let policy: IPolicyConfig =
            CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)?;
        let device_id = windows::core::PCWSTR(device_id_wide.as_ptr());
        policy.set_default_endpoint(device_id, eConsole)?;
        policy.set_default_endpoint(device_id, eMultimedia)?;
        policy.set_default_endpoint(device_id, eCommunications)?;
    }
    Ok(())
}

/// 获取系统主音量状态。
///
/// # Errors
/// COM 初始化失败（“初始化音量控制 COM 失败”）、`CoCreateInstance` 失败、
/// `GetDefaultAudioEndpoint`/`Activate` 失败，或
/// `GetMasterVolumeLevelScalar`/`GetMute` 失败时返回错误。
pub fn get_volume() -> anyhow::Result<VolumeStatus> {
    with_endpoint_volume(|endpoint| unsafe {
        let scalar = endpoint.GetMasterVolumeLevelScalar()?.clamp(0.0, 1.0);
        let muted = endpoint.GetMute()?.as_bool();
        Ok(VolumeStatus {
            percent: ((scalar * 100.0).round() as u8).min(100),
            muted,
        })
    })
}

/// 设置系统主音量 (0-100)。
///
/// # Errors
/// COM 初始化失败（“初始化音量控制 COM 失败”）、默认渲染端点无法取得或激活失败，
/// 或 `SetMasterVolumeLevelScalar` 失败时返回错误。`percent` 超过 100 会被截断为
/// 100，不报错。
pub fn set_volume(percent: u8) -> anyhow::Result<()> {
    let scalar = (percent.min(100) as f32) / 100.0;
    with_endpoint_volume(|endpoint| unsafe {
        endpoint.SetMasterVolumeLevelScalar(scalar, std::ptr::null())?;
        Ok(())
    })
}

/// 设置默认输出设备是否静音。
///
/// # Errors
/// COM 初始化失败（“初始化音量控制 COM 失败”）、默认渲染端点无法取得或激活失败，
/// 或 `SetMute` 失败时返回错误。
pub fn set_muted(muted: bool) -> anyhow::Result<()> {
    with_endpoint_volume(|endpoint| unsafe {
        endpoint.SetMute(muted, std::ptr::null())?;
        Ok(())
    })
}

/// 将默认输出设备音量提高一个步进。
///
/// # Errors
/// COM 初始化失败（“初始化音量控制 COM 失败”）、默认渲染端点无法取得或激活失败，
/// 或 `VolumeStepUp` 失败时返回错误。
pub fn volume_up() -> anyhow::Result<()> {
    with_endpoint_volume(|endpoint| unsafe {
        endpoint.VolumeStepUp(std::ptr::null())?;
        Ok(())
    })
}

/// 将默认输出设备音量降低一个步进。
///
/// # Errors
/// COM 初始化失败（“初始化音量控制 COM 失败”）、默认渲染端点无法取得或激活失败，
/// 或 `VolumeStepDown` 失败时返回错误。
pub fn volume_down() -> anyhow::Result<()> {
    with_endpoint_volume(|endpoint| unsafe {
        endpoint.VolumeStepDown(std::ptr::null())?;
        Ok(())
    })
}

/// 在默认输出设备上切换静音状态。
///
/// # Errors
/// COM 初始化失败（“初始化音量控制 COM 失败”）、默认渲染端点无法取得或激活失败，
/// 或 `GetMute`/`SetMute` 失败时返回错误。
pub fn volume_mute() -> anyhow::Result<()> {
    with_endpoint_volume(|endpoint| unsafe {
        let muted = endpoint.GetMute()?.as_bool();
        endpoint.SetMute(!muted, std::ptr::null())?;
        Ok(())
    })
}

// === 媒体控制 ===

/// 模拟按下媒体播放/暂停键。
///
/// # Errors
/// 当前实现不返回 `Err`：`SendInput` 的失败被忽略，函数总是返回 `Ok(())`。
/// 返回值保留 `Result` 以维持接口稳定。
pub fn media_play_pause() -> anyhow::Result<()> {
    unsafe {
        simulate_keypress(windows::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PLAY_PAUSE);
    }
    Ok(())
}

/// 模拟按下媒体停止键。
///
/// # Errors
/// 当前实现不返回 `Err`：`SendInput` 的失败被忽略，函数总是返回 `Ok(())`。
/// 返回值保留 `Result` 以维持接口稳定。
pub fn media_stop() -> anyhow::Result<()> {
    unsafe {
        simulate_keypress(windows::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_STOP);
    }
    Ok(())
}

/// 模拟按下媒体下一曲键。
///
/// # Errors
/// 当前实现不返回 `Err`：`SendInput` 的失败被忽略，函数总是返回 `Ok(())`。
/// 返回值保留 `Result` 以维持接口稳定。
pub fn media_next() -> anyhow::Result<()> {
    unsafe {
        simulate_keypress(windows::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_NEXT_TRACK);
    }
    Ok(())
}

/// 模拟按下媒体上一曲键。
///
/// # Errors
/// 当前实现不返回 `Err`：`SendInput` 的失败被忽略，函数总是返回 `Ok(())`。
/// 返回值保留 `Result` 以维持接口稳定。
pub fn media_prev() -> anyhow::Result<()> {
    unsafe {
        simulate_keypress(windows::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PREV_TRACK);
    }
    Ok(())
}

unsafe fn simulate_keypress(vk_code: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        SendInput,
    };

    let vk = vk_code;
    let scan = unsafe {
        windows::Win32::UI::Input::KeyboardAndMouse::MapVirtualKeyW(
            vk.0 as u32,
            windows::Win32::UI::Input::KeyboardAndMouse::MAPVK_VK_TO_VSC,
        )
    };

    let down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan as u16,
                dwFlags: KEYEVENTF_EXTENDEDKEY,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan as u16,
                dwFlags: KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        SendInput(&[down], std::mem::size_of::<INPUT>() as i32);
        std::thread::sleep(std::time::Duration::from_millis(20));
        SendInput(&[up], std::mem::size_of::<INPUT>() as i32);
    }
}

