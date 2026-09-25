# 修复计划：控制台窗口 + 配对弹窗异常

## 问题一：控制台窗口显示日志

### 原因
- `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` 
- 仅在 release 模式生效，debug 模式显示控制台

### 修复方案

**文件**: `apps/desktop/src/main.rs`

移除 `not(debug_assertions)` 条件，Windows 平台始终使用 Windows 子系统：

```rust
#![cfg_attr(windows, windows_subsystem = "windows")]
```

这样 debug 也不会弹出控制台窗口。

---

## 问题二：配对弹窗异常

### 原因分析
1. `WindowKind::PopUp` 在 GPUI 0.2.2 中创建独立窗口时行为异常
2. `window.remove_window()` 不是有效的 GPUI API
3. `cx.quit()` 退出整个应用而非仅关闭弹窗
4. 弹窗显示在软件内部而非独立窗口，内容空白后崩溃

### 修复方案：改用 In-App Modal

将配对弹窗从独立窗口改为软件内嵌模态框。

**文件**: `apps/desktop/src/view.rs`

#### 修改 1: 移除独立窗口弹窗逻辑

删除 `RemoteDashboard::update()` 中打开独立窗口的代码（约 131-153 行）。

#### 修改 2: 添加配对弹窗状态

在 `RemoteDashboard` 结构体中添加：
```rust
show_pairing_popup: bool,
```

#### 修改 3: 在主渲染中添加配对弹窗层

在 `RemoteDashboard::render()` 中，当有配对请求时，在顶层渲染一个居中模态框：

```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let has_pairing_request = self.pairing.lock().ok()
        .and_then(|v| v.clone())
        .is_some();
    
    div()
        .size_full()
        .child(/* 现有主内容 */)
        .when(has_pairing_request, |this| {
            this.child(render_pairing_modal(cx))
        })
}
```

#### 修改 4: 渲染配对模态框

```rust
fn render_pairing_modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let request = self.pairing.lock().ok().and_then(|v| v.clone());
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    
    if let Some(request) = request.filter(|r| r.expires_at_unix > now) {
        // 渲染模态背景 + 居中卡片
        div()
            .absolute()
            .size_full()
            .bg(rgba(0, 0, 0, 0.5))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(380.))
                    .bg(theme::SURFACE)
                    .p(px(24.))
                    .rounded(px(12.))
                    .flex_col()
                    .gap(px(12.))
                    .child(/* 配对内容 */)
            )
    } else {
        div().into_any_element()
    }
}
```

#### 修改 5: 删除 PairingPopup 结构体

独立的 `PairingPopup` 不再需要，删除整个结构体及其实现。

---

## 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `apps/desktop/src/main.rs` | 修改 subsystem 属性为始终生效 |
| `apps/desktop/src/view.rs` | 1. 移除独立窗口弹窗逻辑<br>2. 添加 in-app modal 渲染<br>3. 删除 PairingPopup 结构体 |
