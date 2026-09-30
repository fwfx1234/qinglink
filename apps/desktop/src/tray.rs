use anyhow::Result;
use resvg::usvg::{self, Transform};
use tiny_skia::Pixmap;
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

const OPEN_ID: &str = "qinglink.open";
const PAUSE_ID: &str = "qinglink.pause";
const AUTOSTART_ID: &str = "qinglink.autostart";
const KEEP_AWAKE_ID: &str = "qinglink.keep_awake";
const RESTART_ID: &str = "qinglink.restart";
const EXIT_ID: &str = "qinglink.exit";
const MAIN_TRAY_ID: &str = "qinglink.tray.main";
const DEFAULT_TOOLTIP: &str = "轻联 QingLink · 服务启动中";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Open,
    TogglePause,
    ToggleAutostart,
    ToggleKeepAwake,
    Restart,
    Exit,
}

pub struct TrayController {
    icon: TrayIcon,
    pause: CheckMenuItem,
    autostart: CheckMenuItem,
    keep_awake: CheckMenuItem,
    /// 当前工具提示文本，补注册时沿用，避免状态回退。
    tooltip: std::cell::RefCell<String>,
}

impl TrayController {
    /// 在 `Application::run` 的主 UI 线程创建托盘控制器。
    ///
    /// GPUI 在调用启动回调之前已经创建了 Windows 平台窗口和消息队列；随后
    /// `GetMessageW` 会分发同一线程上 tray-icon 隐藏窗口收到的消息。
    pub fn new(paused: bool, autostart_enabled: bool) -> Result<Self> {
        let menu = Menu::new();
        let open = MenuItem::with_id(OPEN_ID, "打开轻联", true, None);
        let pause = CheckMenuItem::with_id(PAUSE_ID, "暂停服务", true, paused, None);
        let autostart =
            CheckMenuItem::with_id(AUTOSTART_ID, "开机启动", true, autostart_enabled, None);
        let keep_awake = CheckMenuItem::with_id(KEEP_AWAKE_ID, "保持唤醒", true, false, None);
        let restart = MenuItem::with_id(RESTART_ID, "重启桌面端", true, None);
        let exit = MenuItem::with_id(EXIT_ID, "退出", true, None);
        menu.append_items(&[
            &open,
            &PredefinedMenuItem::separator(),
            &pause,
            &autostart,
            &keep_awake,
            &restart,
            &PredefinedMenuItem::separator(),
            &exit,
        ])?;

        let icon_data = qinglink_icon().or_else(|error| {
            tracing::warn!(%error, "自定义图标生成失败，使用 fallback 图标");
            fallback_icon()
        })?;
        // 不要使用 `with_guid`：Windows 一旦用 `guidItem` 标识图标，之后**每一次**
        // `Shell_NotifyIcon` 调用都必须带 `NIF_GUID`，否则图标不会被显示
        // （MSDN NOTIFYICONDATAW #troubleshooting 第 1 条）。tray-icon 0.25.1 里
        // `TrayIcon::set_tooltip` 恰好漏掉了 `NIF_GUID`（windows/mod.rs 中唯一未
        // `apply_guid` 的调用），而 `set_status` 每次渲染都会刷新 tooltip，图标因此
        // 从未真正出现在通知区域。第 2 条还规定 GUID 注册与 exe 路径绑定，未签名的
        // exe 一旦换路径注册即失效。身份保持 `(hWnd, uID)`，与可正常工作的 qingqi
        // 实现一致。
        let icon = TrayIconBuilder::new()
            .with_id(MAIN_TRAY_ID)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip(DEFAULT_TOOLTIP)
            .with_icon(icon_data)
            .build()?;
        icon.set_visible(true)?;
        Ok(Self {
            icon,
            pause,
            autostart,
            keep_awake,
            tooltip: std::cell::RefCell::new(DEFAULT_TOOLTIP.to_string()),
        })
    }

    pub fn next_action(&self) -> Option<TrayAction> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            return match event.id.0.as_str() {
                OPEN_ID => Some(TrayAction::Open),
                PAUSE_ID => Some(TrayAction::TogglePause),
                AUTOSTART_ID => Some(TrayAction::ToggleAutostart),
                KEEP_AWAKE_ID => Some(TrayAction::ToggleKeepAwake),
                RESTART_ID => Some(TrayAction::Restart),
                EXIT_ID => Some(TrayAction::Exit),
                _ => None,
            };
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                return Some(TrayAction::Open);
            }
        }
        None
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
            DEFAULT_TOOLTIP.to_string()
        };
        *self.tooltip.borrow_mut() = tooltip.clone();
        if let Err(error) = self.icon.set_tooltip(Some(&tooltip)) {
            tracing::debug!(%error, "托盘 tooltip 更新失败");
        }
    }

    pub fn set_autostart(&self, enabled: bool) {
        self.autostart.set_checked(enabled);
    }

    pub fn set_keep_awake(&self, enabled: bool) {
        self.keep_awake.set_checked(enabled);
    }

    pub fn show_notification(&self, title: &str, message: &str) {
        let tooltip = format!("{title} · {message}");
        *self.tooltip.borrow_mut() = tooltip.clone();
        if let Err(error) = self.icon.set_tooltip(Some(&tooltip)) {
            tracing::debug!(%error, "托盘 tooltip 更新失败");
        }
    }

    /// 探测 Shell 是否已经认识这个托盘图标（内部调用 `Shell_NotifyIconGetRect`）。
    ///
    /// `TrayIconBuilder::build()` 不会上报 `NIM_ADD` 失败（tray-icon 0.25.1 会静默
    /// 忽略），所以只有这里返回 `Some` 才说明图标真的注册成功；返回 `None` 表示
    /// explorer/任务栏尚未就绪，或注册未生效。
    pub fn registration(&self) -> Option<tray_icon::Rect> {
        self.icon.rect()
    }
}

/// 把 [`TrayController::registration`] 的结果格式化成日志文本（纯函数，便于测试）。
pub fn describe_registration(report: Option<tray_icon::Rect>) -> String {
    match report {
        Some(rect) => format!(
            "已注册 rect=({},{},{}x{})",
            rect.position.x, rect.position.y, rect.size.width, rect.size.height
        ),
        None => String::from(
            "未注册；常见原因：1) 托盘/explorer 尚未就绪；2) Windows 11 把新图标收进溢出区，\
             需在「设置 → 个性化 → 任务栏 → 其他系统托盘图标」中允许显示「轻联 QingLink」；\
             3) 本进程完整性级别低于 shell（explorer 提权运行时，未提权进程的 \
             Shell_NotifyIcon 会被 UIPI 拒绝，GetLastError=5 ACCESS_DENIED）",
        ),
    }
}

fn qinglink_icon() -> Result<Icon> {
    const SIZE: u32 = 44;
    let rgba = rasterize_tray_svg(SIZE)?;
    let rgba = template_rgba(rgba);
    Icon::from_rgba(rgba, SIZE, SIZE).map_err(Into::into)
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
            if dx + dy < 12 {
                rgba[index..index + 4].copy_from_slice(&[37, 174, 96, 255]);
            }
        }
    }
    Icon::from_rgba(rgba, SIDE as u32, SIDE as u32).map_err(Into::into)
}

fn rasterize_tray_svg(size: u32) -> Result<Vec<u8>> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_data(include_bytes!("../assets/tray-icon.svg"), &options)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let svg_size = tree.size();
    let max_dim = svg_size.width().max(svg_size.height());
    let scale = size as f32 / max_dim / 1.08;
    let offset_x = (size as f32 - svg_size.width() * scale) * 0.5;
    let offset_y = (size as f32 - svg_size.height() * scale) * 0.5;
    let mut pixmap = Pixmap::new(size, size)
        .ok_or_else(|| anyhow::anyhow!("failed to allocate tray icon pixmap"))?;
    pixmap.fill(tiny_skia::Color::TRANSPARENT);
    resvg::render(
        &tree,
        Transform::from_translate(offset_x, offset_y).pre_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(unpremultiply_rgba(pixmap.data()))
}

fn template_rgba(rgba: Vec<u8>) -> Vec<u8> {
    let mut output = rgba;
    for chunk in output.chunks_exact_mut(4) {
        chunk[0] = 0;
        chunk[1] = 0;
        chunk[2] = 0;
    }
    output
}

fn unpremultiply_rgba(premultiplied: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(premultiplied.len());
    for chunk in premultiplied.chunks_exact(4) {
        let [red, green, blue, alpha] = [chunk[0], chunk[1], chunk[2], chunk[3]];
        if alpha == 0 {
            output.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let scale = alpha as f32 / 255.0;
        output.push((red as f32 / scale).round().min(255.0) as u8);
        output.push((green as f32 / scale).round().min(255.0) as u8);
        output.push((blue as f32 / scale).round().min(255.0) as u8);
        output.push(alpha);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{describe_registration, rasterize_tray_svg, template_rgba};

    #[test]
    fn tray_svg_rasterizes_to_transparent_template_icon() {
        let rgba = rasterize_tray_svg(44).expect("tray SVG should render");
        assert_eq!(rgba.len(), 44 * 44 * 4);
        assert_eq!(&rgba[..4], &[0, 0, 0, 0]);
        assert!(rgba.chunks_exact(4).any(|pixel| pixel[3] > 0));

        let template = template_rgba(rgba);
        assert!(
            template.chunks_exact(4).all(|pixel| {
                pixel[3] == 0 || (pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0)
            })
        );
    }

    #[test]
    fn unregistered_probe_explains_the_overflow_area() {
        let text = describe_registration(None);
        assert!(text.contains("未注册"), "{text}");
        assert!(text.contains("explorer"), "{text}");
        assert!(text.contains("其他系统托盘图标"), "{text}");
    }

    #[test]
    fn registered_probe_includes_the_screen_rect() {
        let rect = tray_icon::Rect {
            position: tray_icon::dpi::PhysicalPosition::new(3744.0, 1816.0),
            size: tray_icon::dpi::PhysicalSize::new(24, 24),
        };
        let text = describe_registration(Some(rect));
        assert!(text.contains("已注册"), "{text}");
        assert!(text.contains("3744"), "{text}");
        assert!(text.contains("24x24"), "{text}");
    }
}
