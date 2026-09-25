use anyhow::Result;
use std::sync::{
    OnceLock,
    atomic::{AtomicIsize, Ordering},
    mpsc,
};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

const OPEN_ID: &str = "qinglink.open";
const PAUSE_ID: &str = "qinglink.pause";
const AUTOSTART_ID: &str = "qinglink.autostart";
const RESTART_ID: &str = "qinglink.restart";
const EXIT_ID: &str = "qinglink.exit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Open,
    TogglePause,
    ToggleAutostart,
    Restart,
    Exit,
}

pub struct TrayController {
    _icon: TrayIcon,
    pause: CheckMenuItem,
    autostart: CheckMenuItem,
    events: mpsc::Receiver<TrayAction>,
}

static EVENT_SENDER: OnceLock<mpsc::Sender<TrayAction>> = OnceLock::new();
static MAIN_WINDOW_HANDLE: AtomicIsize = AtomicIsize::new(0);

pub fn set_main_window_handle(handle: isize) {
    MAIN_WINDOW_HANDLE.store(handle, Ordering::Release);
}

fn show_main_window() {
    let handle = MAIN_WINDOW_HANDLE.load(Ordering::Acquire);
    if handle != 0 {
        qinglink_windows::window::show_native_window(handle);
    }
}

impl TrayController {
    pub fn new(paused: bool, autostart_enabled: bool) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let _ = EVENT_SENDER.set(sender.clone());
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = match event.id.0.as_str() {
                OPEN_ID => {
                    show_main_window();
                    TrayAction::Open
                }
                PAUSE_ID => TrayAction::TogglePause,
                AUTOSTART_ID => TrayAction::ToggleAutostart,
                RESTART_ID => TrayAction::Restart,
                EXIT_ID => TrayAction::Exit,
                _ => return,
            };
            let _ = sender.send(action);
        }));
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window();
                if let Some(sender) = EVENT_SENDER.get() {
                    let _ = sender.send(TrayAction::Open);
                }
            }
        }));
        let menu = Menu::new();
        let open = MenuItem::with_id(OPEN_ID, "打开轻联", true, None);
        let pause = CheckMenuItem::with_id(PAUSE_ID, "暂停服务", true, paused, None);
        let autostart =
            CheckMenuItem::with_id(AUTOSTART_ID, "开机启动", true, autostart_enabled, None);
        let restart = MenuItem::with_id(RESTART_ID, "重启桌面端", true, None);
        let exit = MenuItem::with_id(EXIT_ID, "退出", true, None);
        menu.append_items(&[
            &open,
            &PredefinedMenuItem::separator(),
            &pause,
            &autostart,
            &restart,
            &PredefinedMenuItem::separator(),
            &exit,
        ])?;
        let icon_data = qinglink_icon().or_else(|e| {
            tracing::warn!(error = %e, "自定义图标生成失败，使用 fallback 图标");
            fallback_icon()
        })?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("轻联 QingLink · 服务启动中")
            .with_icon(icon_data)
            .build()?;
        Ok(Self {
            _icon: icon,
            pause,
            autostart,
            events: receiver,
        })
    }

    pub fn next_action(&self) -> Option<TrayAction> {
        self.events.try_recv().ok()
    }

    pub fn set_paused(&self, paused: bool) {
        self.pause.set_checked(paused);
        self.set_status(paused, !paused, None);
    }

    pub fn set_status(&self, paused: bool, running: bool, error: Option<&str>) {
        let tooltip = if let Some(error) = error {
            format!("轻联 QingLink · 服务异常：{error}")
        } else if paused {
            "轻联 QingLink · 服务已暂停".to_string()
        } else if running {
            "轻联 QingLink · 服务运行中".to_string()
        } else {
            "轻联 QingLink · 服务启动中".to_string()
        };
        let _ = self._icon.set_tooltip(Some(&tooltip));
    }

    pub fn set_autostart(&self, enabled: bool) {
        self.autostart.set_checked(enabled);
    }

    pub fn show_notification(&self, title: &str, message: &str) {
        let _ = self
            ._icon
            .set_tooltip(Some(&format!("{} · {}", title, message)));
    }
}

fn qinglink_icon() -> Result<Icon> {
    const SIDE: usize = 32;
    let mut rgba = vec![0_u8; SIDE * SIDE * 4];
    for y in 0..SIDE {
        for x in 0..SIDE {
            let index = (y * SIDE + x) * 4;
            let inside = (4..28).contains(&x) && (4..28).contains(&y);
            let link = (8..13).contains(&y) && (7..25).contains(&x)
                || (19..24).contains(&y) && (7..25).contains(&x)
                || (11..22).contains(&x) && (10..22).contains(&y);
            if inside {
                rgba[index..index + 4].copy_from_slice(if link {
                    &[255, 255, 255, 255]
                } else {
                    &[37, 174, 96, 255]
                });
            }
        }
    }
    Icon::from_rgba(rgba, SIDE as u32, SIDE as u32).map_err(Into::into)
}

fn fallback_icon() -> Result<Icon> {
    const SIDE: usize = 32;
    let mut rgba = vec![0_u8; SIDE * SIDE * 4];
    for y in 0..SIDE {
        for x in 0..SIDE {
            let index = (y * SIDE + x) * 4;
            let cx = SIDE / 2;
            let cy = SIDE / 2;
            let dx = (x as isize - cx as isize).abs() as usize;
            let dy = (y as isize - cy as isize).abs() as usize;
            let dist = dx + dy;
            if dist < 12 {
                rgba[index..index + 4].copy_from_slice(&[37, 174, 96, 255]);
            }
        }
    }
    Icon::from_rgba(rgba, SIDE as u32, SIDE as u32).map_err(Into::into)
}
