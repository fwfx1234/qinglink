//! 托盘菜单，直接构建在 Win32 菜单 API 上。
//!
//! 菜单刻意保持简短，与产品规格一致：打开界面、重启服务、保持机器唤醒、退出。
//! 其它一切都放在 Web 界面里，那里才是真正的管理面。
//!
//! 命令 id 从 1 开始，因为 0 被 `TrackPopupMenu` 保留，表示"用户关闭了菜单"；
//! id 为 0 会让每次关闭都看起来像点了第一项。

use anyhow::{Context, Result};

use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, MF_SEPARATOR, MF_STRING,
};
use windows::core::w;

/// Ids handed to `AppendMenuW` and returned through `WM_COMMAND`.
pub const OPEN_UI_ID: usize = 1;
pub const RESTART_SERVICE_ID: usize = 2;
pub const KEEP_AWAKE_ID: usize = 3;
pub const EXIT_ID: usize = 4;

/// Mask for the command id inside `WM_COMMAND`'s `wparam`.
pub const MENU_ID_MASK: u32 = 0xffff;

/// `MF_CHECKED`, as the `u32` that `CheckMenuItem` expects.
const CHECKED: u32 = 0x0000_0008;

/// A user action selected from the tray menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    /// Open the management interface in the default browser.
    OpenUi,
    /// Restart `QingLinkService`. Requires elevation, so it raises a UAC prompt.
    RestartService,
    /// Toggle the keep-awake (prevent sleep) state.
    ToggleKeepAwake,
    /// Quit the desktop process, leaving the service running.
    Exit,
}

impl TrayAction {
    /// Map a `WM_COMMAND` id onto an action.
    pub fn from_command_id(id: usize) -> Option<Self> {
        match id {
            OPEN_UI_ID => Some(Self::OpenUi),
            RESTART_SERVICE_ID => Some(Self::RestartService),
            KEEP_AWAKE_ID => Some(Self::ToggleKeepAwake),
            EXIT_ID => Some(Self::Exit),
            _ => None,
        }
    }
}

/// Map a `WM_COMMAND` id onto an action.
pub fn action_for(id: u32) -> Option<TrayAction> {
    TrayAction::from_command_id(id as usize)
}

/// The constructed popup menu.
///
/// The `HMENU` is owned here and destroyed on drop; `TrackPopupMenu` only
/// borrows it.
pub struct TrayMenu {
    handle: HMENU,
}

impl std::fmt::Debug for TrayMenu {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("TrayMenu").finish_non_exhaustive()
    }
}

impl TrayMenu {
    /// Build the menu.
    pub fn build() -> Result<Self> {
        // SAFETY: no inputs; the returned handle is owned by `Self`.
        let menu = unsafe { CreatePopupMenu() }.context("cannot create the tray popup menu")?;
        // SAFETY: every call below appends one item to a live, owned menu and
        // passes a static, NUL-terminated label.
        unsafe {
            append(menu, MF_STRING, OPEN_UI_ID, w!("打开 QingLink"))?;
            append(menu, MF_SEPARATOR, 0, w!(""))?;
            append(menu, MF_STRING, RESTART_SERVICE_ID, w!("重启服务"))?;
            append(menu, MF_STRING, KEEP_AWAKE_ID, w!("保持唤醒"))?;
            append(menu, MF_SEPARATOR, 0, w!(""))?;
            append(menu, MF_STRING, EXIT_ID, w!("退出"))?;
        }
        Ok(Self { handle: menu })
    }

    /// The native handle, for `TrackPopupMenu` and `CheckMenuItem`.
    pub fn handle(&self) -> HMENU {
        self.handle
    }

    /// Tick or untick a checkable item.
    pub fn set_checked(&self, id: usize, checked: bool) {
        // SAFETY: the menu handle is live and owned by this thread; the id is
        // one this module created.
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::CheckMenuItem(
                self.handle,
                id as u32,
                if checked { CHECKED } else { 0 },
            )
        };
    }
}

impl Drop for TrayMenu {
    fn drop(&mut self) {
        // SAFETY: the handle was created in `build` and is destroyed once.
        let _ = unsafe { DestroyMenu(self.handle) };
    }
}

/// Append one item, turning a failure into an error instead of a silent gap.
///
/// # Safety
/// `menu` must be a live popup menu and `label` a NUL-terminated string.
unsafe fn append(
    menu: HMENU,
    flags: windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS,
    id: usize,
    label: windows::core::PCWSTR,
) -> Result<()> {
    // SAFETY: upheld by the caller contract.
    unsafe { AppendMenuW(menu, flags, id, label) }.context("cannot append a tray menu item")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_ids_map_to_expected_actions() {
        assert_eq!(action_for(OPEN_UI_ID as u32), Some(TrayAction::OpenUi));
        assert_eq!(
            action_for(RESTART_SERVICE_ID as u32),
            Some(TrayAction::RestartService)
        );
        assert_eq!(
            action_for(KEEP_AWAKE_ID as u32),
            Some(TrayAction::ToggleKeepAwake)
        );
        assert_eq!(action_for(EXIT_ID as u32), Some(TrayAction::Exit));
    }

    #[test]
    fn separators_and_unknown_commands_are_ignored() {
        assert_eq!(action_for(0), None);
        assert_eq!(action_for(MENU_ID_MASK + 1), None);
    }
}
