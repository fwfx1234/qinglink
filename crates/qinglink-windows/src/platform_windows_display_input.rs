fn wide_array_string(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

fn display_mode(_device_name: &str, mode: &DEVMODEW) -> DisplayModeResponse {
    DisplayModeResponse {
        width: mode.dmPelsWidth,
        height: mode.dmPelsHeight,
        refresh_millihz: mode.dmDisplayFrequency.saturating_mul(1000),
    }
}

const COMMON_DISPLAY_RESOLUTIONS: &[(u32, u32)] = &[(1920, 1080), (2560, 1440), (3840, 2160)];
const PREFERRED_REFRESH_MILLIHZ: u32 = 60_000;

fn enumerate_supported_display_modes(device_name: &str) -> Vec<DisplayModeResponse> {
    let device = to_wide_z(device_name);
    let mut modes = Vec::new();
    for index in 0..256_u32 {
        let mut mode = DEVMODEW {
            dmSize: mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let ok = unsafe {
            EnumDisplaySettingsExW(
                windows::core::PCWSTR(device.as_ptr()),
                ENUM_DISPLAY_SETTINGS_MODE(index),
                &mut mode,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
            .as_bool()
        };
        if !ok {
            break;
        }
        if mode.dmPelsWidth == 0 || mode.dmPelsHeight == 0 {
            continue;
        }
        let item = display_mode(device_name, &mode);
        if !modes.iter().any(|candidate: &DisplayModeResponse| {
            candidate.width == item.width
                && candidate.height == item.height
                && candidate.refresh_millihz == item.refresh_millihz
        }) {
            modes.push(item);
        }
    }
    modes.sort_by_key(|mode| (mode.width, mode.height, mode.refresh_millihz));
    modes
}

fn select_common_display_modes(
    supported: &[DisplayModeResponse],
    current: &DisplayModeResponse,
) -> Vec<DisplayModeResponse> {
    COMMON_DISPLAY_RESOLUTIONS
        .iter()
        .filter_map(|&(width, height)| {
            supported
                .iter()
                .filter(|mode| mode.width == width && mode.height == height)
                .min_by_key(|mode| {
                    let is_current = mode.width == current.width
                        && mode.height == current.height
                        && mode.refresh_millihz == current.refresh_millihz;
                    (
                        !is_current,
                        mode.refresh_millihz.abs_diff(PREFERRED_REFRESH_MILLIHZ),
                        std::cmp::Reverse(mode.refresh_millihz),
                    )
                })
                .cloned()
        })
        .collect()
}

#[cfg(test)]
mod display_mode_tests {
    use super::*;

    fn mode(width: u32, height: u32, refresh_millihz: u32) -> DisplayModeResponse {
        DisplayModeResponse {
            width,
            height,
            refresh_millihz,
        }
    }

    #[test]
    fn common_mode_selection_prefers_current_then_nearest_sixty_hz() {
        let supported = vec![
            mode(1280, 720, 60_000),
            mode(1920, 1080, 60_000),
            mode(1920, 1080, 144_000),
            mode(2560, 1440, 59_000),
            mode(2560, 1440, 60_000),
            mode(3840, 2160, 30_000),
            mode(3840, 2160, 120_000),
        ];

        let selected = select_common_display_modes(&supported, &mode(1920, 1080, 144_000));
        let actual: Vec<_> = selected
            .iter()
            .map(|item| (item.width, item.height, item.refresh_millihz))
            .collect();

        assert_eq!(
            actual,
            vec![
                (1920, 1080, 144_000),
                (2560, 1440, 60_000),
                (3840, 2160, 30_000),
            ]
        );
    }

    #[test]
    fn common_mode_selection_excludes_unsupported_and_nonstandard_resolutions() {
        let supported = vec![
            mode(1280, 720, 60_000),
            mode(1366, 768, 60_000),
            mode(1920, 1080, 60_000),
            mode(3440, 1440, 100_000),
        ];

        let selected = select_common_display_modes(&supported, &mode(1366, 768, 60_000));

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].width, 1920);
        assert_eq!(selected[0].height, 1080);
        assert_eq!(selected[0].refresh_millihz, 60_000);
    }
}

pub fn get_displays() -> anyhow::Result<Vec<DisplayInfoResponse>> {
    let mut displays = Vec::new();
    let mut attached_displays = 0usize;
    for index in 0..32_u32 {
        let mut device = DISPLAY_DEVICEW {
            cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        let ok = unsafe { EnumDisplayDevicesW(None, index, &mut device, 0).as_bool() };
        if !ok {
            break;
        }
        if !device
            .StateFlags
            .contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP)
        {
            continue;
        }
        attached_displays += 1;
        let id = wide_array_string(&device.DeviceName);
        if id.is_empty() {
            continue;
        }
        let device_name = to_wide_z(&id);
        let mut current = DEVMODEW {
            dmSize: mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let has_current = unsafe {
            EnumDisplaySettingsExW(
                windows::core::PCWSTR(device_name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut current,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
            .as_bool()
        };
        if !has_current {
            continue;
        }
        let current_mode = display_mode(&id, &current);
        displays.push(DisplayInfoResponse {
            id: id.clone(),
            name: wide_array_string(&device.DeviceString),
            primary: device.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE),
            modes: select_common_display_modes(
                &enumerate_supported_display_modes(&id),
                &current_mode,
            ),
            current_mode,
        });
    }
    anyhow::ensure!(
        attached_displays == 0 || !displays.is_empty(),
        "无法读取任何已连接显示器的当前显示模式"
    );
    anyhow::ensure!(!displays.is_empty(), "未检测到连接到 Windows 桌面的显示器");
    Ok(displays)
}

pub fn set_display_mode(
    display_id: &str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
) -> anyhow::Result<()> {
    let device = to_wide_z(display_id);
    let mut mode = DEVMODEW {
        dmSize: mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    let ok = unsafe {
        EnumDisplaySettingsExW(
            windows::core::PCWSTR(device.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut mode,
            ENUM_DISPLAY_SETTINGS_FLAGS(0),
        )
        .as_bool()
    };
    anyhow::ensure!(ok, "无法读取当前显示模式");
    let current_mode = display_mode(display_id, &mode);
    let supported = enumerate_supported_display_modes(display_id);
    let allowed = select_common_display_modes(&supported, &current_mode);
    anyhow::ensure!(
        allowed.iter().any(|candidate| {
            candidate.width == width
                && candidate.height == height
                && candidate.refresh_millihz == refresh_millihz
        }),
        "该显示模式不在常用分辨率候选列表中"
    );
    mode.dmPelsWidth = width;
    mode.dmPelsHeight = height;
    mode.dmDisplayFrequency = refresh_millihz / 1000;
    mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
    let result = unsafe {
        ChangeDisplaySettingsExW(
            windows::core::PCWSTR(device.as_ptr()),
            Some(&mode),
            None,
            CDS_FULLSCREEN,
            None,
        )
    };
    anyhow::ensure!(result == DISP_CHANGE_SUCCESSFUL, "Windows 拒绝了该显示模式");
    Ok(())
}

pub fn mouse_move(dx: i32, dy: i32) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
    anyhow::ensure!(
        dx.abs() <= qinglink_protocol::MAX_MOUSE_MOVE_DELTA
            && dy.abs() <= qinglink_protocol::MAX_MOUSE_MOVE_DELTA,
        "鼠标位移过大"
    );
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_move",
        pin: None,
        dx: Some(dx),
        dy: Some(dy),
        button: None,
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_move(dx, dy);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    if inserted == 1 { Ok(()) } else { require_hid_input(&request) }
}

pub fn mouse_move_absolute(x: u32, y: u32) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEINPUT,
        MOUSEEVENTF_VIRTUALDESK, SendInput,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    anyhow::ensure!(width > 0 && height > 0, "无法读取虚拟屏幕分辨率");
    // The protocol carries physical screen coordinates. Convert them to the
    // 0..65535 virtual-screen range required by MOUSEEVENTF_ABSOLUTE.
    let abs_x = ((((x as i64 - left as i64) * 65535) / (width as i64 - 1).max(1))
        .clamp(0, 65535)) as i32;
    let abs_y = ((((y as i64 - top as i64) * 65535) / (height as i64 - 1).max(1))
        .clamp(0, 65535)) as i32;
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_move_absolute(x, y);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: abs_x,
                dy: abs_y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == 1, "SendInput 绝对定位鼠标移动失败");
    Ok(())
}

pub fn mouse_button(button: &str) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let (down, up) = match button {
        "left" => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        _ => anyhow::bail!("不支持的鼠标按键"),
    };
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_button",
        pin: None,
        dx: None,
        dy: None,
        button: Some(button),
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_button(button, true)
            .and_then(|_| crate::secure_desktop::send_mouse_button(button, false));
    }
    ensure_remote_input_desktop()?;
    let inputs = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: down,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: up,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let inserted = unsafe { SendInput(&inputs, mem::size_of::<INPUT>() as i32) };
    if inserted == 2 { Ok(()) } else { require_hid_input(&request) }
}

/// 仅按下或仅抬起鼠标按键，用于按住拖选等需要持续按住的操作。
pub fn mouse_button_state(button: &str, pressed: bool) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let flag = match (button, pressed) {
        ("left", true) => MOUSEEVENTF_LEFTDOWN,
        ("left", false) => MOUSEEVENTF_LEFTUP,
        ("right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => MOUSEEVENTF_RIGHTUP,
        _ => anyhow::bail!("不支持的鼠标按键状态"),
    };
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_button_state",
        pin: None,
        dx: None,
        dy: None,
        button: Some(button),
        pressed: Some(pressed),
        delta: None,
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_button(button, pressed);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flag,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    if inserted == 1 { Ok(()) } else { require_hid_input(&request) }
}

pub fn mouse_wheel(delta: i32) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    };
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_wheel",
        pin: None,
        dx: None,
        dy: None,
        button: None,
        pressed: None,
        delta: Some(delta),
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_wheel(delta);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: (delta.clamp(-20, 20) * 120) as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    if inserted == 1 { Ok(()) } else { require_hid_input(&request) }
}

fn ensure_remote_input_desktop() -> anyhow::Result<()> {
    // Secure-desktop restrictions are intentionally disabled: remote input is
    // allowed even while Windows is locked.
    Ok(())
}

pub fn send_text(text: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!text.is_empty() && text.len() <= 64 * 1024, "文字长度无效");
    // The network server runs as LocalSystem (session 0). Direct SendInput
    // from that session cannot reach the user's desktop, so route text through
    // the per-session agent just like mouse and HID input.
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_text(text);
    }
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
        VIRTUAL_KEY,
    };
    // 使用 KEYEVENTF_UNICODE 逐字符注入，不经过剪贴板：
    // 不会破坏用户剪贴板内容，也没有 Ctrl+V 与剪贴板刷新的竞态。
    let mut inputs: Vec<INPUT> = Vec::with_capacity(text.encode_utf16().count() * 2);
    for unit in text.encode_utf16() {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }
    let inserted = unsafe { SendInput(&inputs, mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(
        inserted == inputs.len() as u32,
        "SendInput 发送文字失败（可能被系统或 UAC 拦截）"
    );
    Ok(())
}

pub fn key_press(key: &str) -> anyhow::Result<()> {
    let usage = match key {
        "a" => 0x04, "b" => 0x05, "c" => 0x06, "d" => 0x07,
        "e" => 0x08, "f" => 0x09, "g" => 0x0a, "h" => 0x0b,
        "i" => 0x0c, "j" => 0x0d, "k" => 0x0e, "l" => 0x0f,
        "m" => 0x10, "n" => 0x11, "o" => 0x12, "p" => 0x13,
        "q" => 0x14, "r" => 0x15, "s" => 0x16, "t" => 0x17,
        "u" => 0x18, "v" => 0x19, "w" => 0x1a, "x" => 0x1b,
        "y" => 0x1c, "z" => 0x1d, "1" => 0x1e, "2" => 0x1f,
        "3" => 0x20, "4" => 0x21, "5" => 0x22, "6" => 0x23,
        "7" => 0x24, "8" => 0x25, "9" => 0x26, "0" => 0x27,
        "enter" => 0x28, "escape" => 0x29, "backspace" => 0x2a,
        "tab" => 0x2b, "space" => 0x2c, "delete" => 0x4c,
        "right" => 0x4f, "left" => 0x50, "down" => 0x51, "up" => 0x52,
        "home" => 0x4a, "end" => 0x4d, "pageup" => 0x4b, "pagedown" => 0x4e,
        _ => anyhow::bail!("不支持的按键: {}", key),
    };
    key_event(usage, 0, true)?;
    key_event(usage, 0, false)
}

fn require_hid_input(request: &LockInputPipeRequest<'_>) -> anyhow::Result<()> {
    let response = lock_input_pipe(request)?;
    anyhow::ensure!(response.ok, "{}: {}", response.code, response.message);
    Ok(())
}

pub fn key_event(usage: u8, modifiers: u8, pressed: bool) -> anyhow::Result<()> {
    anyhow::ensure!((0x04..=0x73).contains(&usage), "不支持的 HID usage");
    let request = LockInputPipeRequest {
        version: 1,
        command: "key_event",
        pin: None,
        dx: None,
        dy: None,
        button: None,
        pressed: Some(pressed),
        delta: None,
        usage: Some(usage),
        modifiers: Some(modifiers),
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_hid_key(usage, modifiers, pressed);
    }
    match send_hid_usage_with_send_input(usage, modifiers, pressed) {
        Ok(()) => Ok(()),
        Err(_) => require_hid_input(&request),
    }
}

fn send_hid_usage_with_send_input(usage: u8, modifiers: u8, pressed: bool) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput};
    let (scan, extended) = hid_usage_to_scan_code(usage)
        .ok_or_else(|| anyhow::anyhow!("HID usage 没有对应的 Windows 扫描码"))?;
    let mut inputs = Vec::new();
    // USB HID modifier bits.  Right-hand modifiers use the extended scan
    // code prefix; without it Windows interprets them as the left key.
    let modifier_scans = [
        (0x01, 0x1d, false), (0x02, 0x2a, false),
        (0x04, 0x38, false), (0x08, 0x5b, true),
        (0x10, 0x1d, true),  (0x20, 0x36, false),
        (0x40, 0x38, true),  (0x80, 0x5c, true),
    ];
    if pressed {
        for (bit, modifier_scan, modifier_extended) in modifier_scans {
            if modifiers & bit != 0 { inputs.push(scan_code_input(modifier_scan, true, modifier_extended)); }
        }
        inputs.push(scan_code_input(scan, true, extended));
    } else {
        inputs.push(scan_code_input(scan, false, extended));
        for (bit, modifier_scan, modifier_extended) in modifier_scans.into_iter().rev() {
            if modifiers & bit != 0 { inputs.push(scan_code_input(modifier_scan, false, modifier_extended)); }
        }
    }
    let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == inputs.len() as u32, "SendInput HID 按键失败");
    Ok(())
}

fn scan_code_input(scan: u16, pressed: bool, extended: bool) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE,
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT {
            wVk: Default::default(),
            wScan: scan,
            dwFlags: KEYEVENTF_SCANCODE
                | if extended { KEYEVENTF_EXTENDEDKEY } else { Default::default() }
                | if pressed { Default::default() } else { KEYEVENTF_KEYUP },
            time: 0,
            dwExtraInfo: 0,
        }},
    }
}

fn hid_usage_to_scan_code(usage: u8) -> Option<(u16, bool)> {
    const TABLE: &[(u8, u16)] = &[
        (0x04,0x1e),(0x05,0x30),(0x06,0x2e),(0x07,0x20),(0x08,0x12),(0x09,0x21),(0x0a,0x22),(0x0b,0x23),(0x0c,0x17),(0x0d,0x24),(0x0e,0x25),(0x0f,0x26),(0x10,0x32),(0x11,0x31),(0x12,0x18),(0x13,0x19),(0x14,0x10),(0x15,0x13),(0x16,0x1f),(0x17,0x14),(0x18,0x16),(0x19,0x2f),(0x1a,0x11),(0x1b,0x2d),(0x1c,0x15),(0x1d,0x2c),
        (0x1e,0x02),(0x1f,0x03),(0x20,0x04),(0x21,0x05),(0x22,0x06),(0x23,0x07),(0x24,0x08),(0x25,0x09),(0x26,0x0a),(0x27,0x0b),(0x28,0x1c),(0x29,0x01),(0x2a,0x0e),(0x2b,0x0f),(0x2c,0x39),(0x2d,0x0c),(0x2e,0x0d),(0x2f,0x1a),(0x30,0x1b),(0x31,0x2b),(0x33,0x27),(0x34,0x28),(0x35,0x29),(0x36,0x33),(0x37,0x34),(0x38,0x35),(0x39,0x3a),
        (0x3a,0x3b),(0x3b,0x3c),(0x3c,0x3d),(0x3d,0x3e),(0x3e,0x3f),(0x3f,0x40),(0x40,0x41),(0x41,0x42),(0x42,0x43),(0x43,0x44),(0x44,0x57),(0x45,0x58),(0x4c,0x53),(0x4f,0x4d),(0x50,0x4b),(0x51,0x50),(0x52,0x48),(0x4a,0x47),(0x4d,0x4f),(0x4b,0x49),(0x4e,0x51),
    ];
    let scan = TABLE.iter().find_map(|(candidate, scan)| (*candidate == usage).then_some(*scan))?;
    Some((scan, matches!(usage, 0x4f..=0x52)))
}


