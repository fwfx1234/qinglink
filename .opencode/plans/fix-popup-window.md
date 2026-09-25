# 修复计划：独立配对弹窗

## 问题分析

### 问题一：控制台窗口
- `windows_subsystem = "windows"` 仅 release 生效
- 修复：移除 `not(debug_assertions)` 条件

### 问题二：配对弹窗异常
- `cx.quit()` 退出整个应用
- 弹窗空白后崩溃

## GPUI 0.2.2 API 研究结论

1. `window.remove_window()` **是有效的**，设置 `removed = true`，下一帧移除窗口
2. 没有 `WindowLevel` 枚举，用 `WindowKind::PopUp` 实现置顶
3. `cx.quit()` 退出整个应用，不能用于关闭单个窗口
4. `WindowKind::PopUp` 在 Windows 上使用 `WS_EX_TOOLWINDOW`（无标题栏）

## 修复方案

### 修复 1: 始终消除控制台

**文件**: `apps/desktop/src/main.rs`

```rust
#![cfg_attr(windows, windows_subsystem = "windows")]
```

### 修复 2: 配对弹窗

**文件**: `apps/desktop/src/view.rs`

#### 2.1 移除 WindowLevel 导入

```rust
// 删除
use gpui::{
    AnyElement, AppContext, Bounds, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowKind,
    WindowLevel, WindowOptions, div, px, size,
};

// 改为
use gpui::{
    AnyElement, AppContext, Bounds, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowKind,
    WindowOptions, div, px, size,
};
```

#### 2.2 移除 WindowOptions 中的 level 字段

```rust
// 删除
let _ = cx.open_window(
    WindowOptions {
        ...
        level: Some(WindowLevel::AlwaysOnTop),  // 删除这行
        ...
    },
    ...
);

// 改为（kind: WindowKind::PopUp 已经提供置顶行为）
let _ = cx.open_window(
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(popup_bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("轻联配对".into()),
            ..Default::default()
        }),
        kind: WindowKind::PopUp,
        focus: true,
        show: true,
        is_movable: true,
        ..Default::default()
    },
    move |window, cx| cx.new(|cx| PairingPopup::new(pairing, state, window, cx)),
);
```

#### 2.3 修复 PairingPopup 的监控循环

```rust
// 之前（错误：cx.quit() 退出整个应用）
let pairing_monitor = Arc::clone(&pairing);
cx.spawn(async move |popup, async_cx| {
    let mut had_request = false;
    loop {
        async_cx.background_executor().timer(Duration::from_millis(200)).await;
        if popup.update(async_cx, |_, cx| {
            let current_request = pairing_monitor.lock().ok().and_then(|v| v.clone());
            if current_request.is_some() {
                had_request = true;
            } else if had_request {
                cx.quit();  // ❌ 错误：退出整个应用
                return;
            }
            cx.notify();
        }).is_err() {
            break;
        }
    }
}).detach();

// 改为（正确：使用 window.remove_window()）
let pairing_monitor = Arc::clone(&pairing);
cx.spawn(async move |popup, async_cx| {
    loop {
        async_cx.background_executor().timer(Duration::from_millis(200)).await;
        if popup.update(async_cx, |_, window, cx| {
            let current_request = pairing_monitor.lock().ok().and_then(|v| v.clone());
            if current_request.is_none() {
                window.remove_window();  // ✅ 正确：只关闭此窗口
            }
            cx.notify();
        }).is_err() {
            break;
        }
    }
}).detach();
```

#### 2.4 确认 render 中的 remove_window 正确

```rust
impl Render for PairingPopup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let request = self.pairing.lock().ok()
            .and_then(|value| value.clone())
            .filter(|request| request.expires_at_unix > now);
            
        let Some(request) = request else {
            window.remove_window();  // ✅ 这是有效的
            return div().into_any_element();
        };
        
        // ... 正常渲染配对内容 ...
    }
}
```

---

## 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `apps/desktop/src/main.rs` | 修改 subsystem 属性 |
| `apps/desktop/src/view.rs` | 1. 移除 WindowLevel 导入<br>2. 移除 level 字段<br>3. 修复监控循环使用 window.remove_window() |
