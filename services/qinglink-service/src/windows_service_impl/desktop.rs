//! 交互式会话的选择与代理桌面附加。

use anyhow::Result;

pub(super) fn attach_agent_desktop() -> Result<()> {
    use super::input::attach_owned_input_desktop;
    let name = attach_owned_input_desktop("attach_agent_desktop")?;
    tracing::info!(desktop = %name, "agent thread desktop attached");
    Ok(())
}

/// Select the active console session, falling back to an active RDP session.
pub(super) fn active_interactive_session() -> Option<u32> {
    #[repr(C)]
    struct SessionInfo {
        session_id: u32,
        station_name: *mut u16,
        state: i32,
    }
    #[link(name = "wtsapi32")]
    unsafe extern "system" {
        fn WTSEnumerateSessionsW(
            server: *mut std::ffi::c_void,
            reserved: u32,
            version: u32,
            sessions: *mut *mut SessionInfo,
            count: *mut u32,
        ) -> i32;
        fn WTSFreeMemory(memory: *mut std::ffi::c_void);
    }
    const WTS_ACTIVE: i32 = 0;
    let mut sessions = std::ptr::null_mut();
    let mut count = 0u32;
    // SAFETY: Windows allocates the session array and we free it below.
    let ok =
        unsafe { WTSEnumerateSessionsW(std::ptr::null_mut(), 0, 1, &mut sessions, &mut count) }
            != 0;
    if !ok || sessions.is_null() {
        return None;
    }
    // SAFETY: the API returned `count` initialized entries.
    let entries = unsafe { std::slice::from_raw_parts(sessions, count as usize) };
    let mut console = None;
    let mut rdp = None;
    for entry in entries {
        if entry.state != WTS_ACTIVE {
            continue;
        }
        let name = if entry.station_name.is_null() {
            String::new()
        } else {
            let mut length = 0usize;
            // SAFETY: WTS owns a terminated UTF-16 station name.
            unsafe {
                while length < 256 && *entry.station_name.add(length) != 0 {
                    length += 1;
                }
                String::from_utf16_lossy(std::slice::from_raw_parts(entry.station_name, length))
            }
        };
        if name.eq_ignore_ascii_case("Console") {
            console = Some(entry.session_id);
        } else if name.to_ascii_lowercase().starts_with("rdp-") {
            rdp = Some(entry.session_id);
        }
    }
    // SAFETY: the pointer came from WTSEnumerateSessionsW.
    unsafe { WTSFreeMemory(sessions.cast()) };
    console.or(rdp)
}
