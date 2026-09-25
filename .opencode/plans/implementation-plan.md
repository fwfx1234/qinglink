# 实现计划：静默启动 + 配对弹窗 + 锁屏/UAC 输入

## 阶段一：静默启动到托盘

### 1.1 消除控制台窗口

**文件**: `apps/desktop/src/main.rs`

在文件顶部添加：
```rust
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
```

这样 release 编译时为 Windows 子系统（无控制台），debug 编译保留控制台方便调试。

### 1.2 优化日志输出

**文件**: `apps/desktop/src/logging.rs`

release 模式移除 stderr 输出，只保留文件日志：

```rust
pub fn initialize(paths: &AppPaths) -> Result<WorkerGuard> {
    let log_path = paths.log_file("qinglink.log");
    let directory = log_path.parent().ok_or_else(|| anyhow::anyhow!("invalid QingLink log path"))?;
    let appender = tracing_appender::rolling::daily(directory, "qinglink.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let registry = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("qinglink=info".parse()?));
    
    #[cfg(debug_assertions)]
    let registry = registry.with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr));
    
    registry.with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(writer)).init();
    Ok(guard)
}
```

### 1.3 托盘启动通知

**文件**: `apps/desktop/src/tray.rs`

启动完成后显示托盘气泡提示"轻联服务已启动"。

---

## 阶段二：配对弹窗置顶并自动关闭

### 2.1 PairingPopup 窗口置顶

**文件**: `apps/desktop/src/view.rs`

修改 PairingPopup 创建时的 WindowOptions：
- 添加 `WindowLevel::AlwaysOnTop` 使窗口置顶
- 确保弹窗显示在最前面

### 2.2 配对完成后自动关闭

**文件**: `apps/desktop/src/view.rs`

在配对状态检测逻辑中：
- 监听配对成功事件
- 配对成功后自动关闭 PairingPopup 窗口
- 显示托盘气泡"配对成功"

### 2.3 托盘气泡通知

**文件**: `apps/desktop/src/tray.rs`

添加 `show_balloon_notification()` 方法：
- 配对请求到达时显示"新的配对请求"
- 配对成功后显示"设备已连接"

---

## 阶段三：增强锁屏/UAC 输入

### 3.1 添加 SendSAS 支持

**文件**: `crates/qinglink-windows/src/secure_desktop.rs`

```rust
/// 发送 Ctrl+Alt+Del (Secure Attention Sequence)
/// 需要 SYSTEM 权限或 uiAccess=true + 签名
pub fn send_sas() -> anyhow::Result<()> {
    #[cfg(windows)]
    unsafe {
        windows::Win32::System::StationsAndDesktops::SendSAS(false.into())?;
    }
    Ok(())
}
```

### 3.2 增强桌面切换逻辑

**文件**: `services/input-service/src/main.rs`

修改 `attach_agent_desktop()` 函数：

```rust
fn attach_agent_desktop() -> Result<()> {
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEWINDOW, DESKTOP_ENUMERATE,
        DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
        OpenInputDesktop, SetThreadDesktop,
    };
    
    let access = DESKTOP_ACCESS_FLAGS(
        DESKTOP_CREATEWINDOW.0 | DESKTOP_ENUMERATE.0 | DESKTOP_READOBJECTS.0
            | DESKTOP_SWITCHDESKTOP.0 | DESKTOP_WRITEOBJECTS.0,
    );
    
    // 尝试打开输入桌面
    let desktop = unsafe { OpenInputDesktop(Default::default(), false, access)? };
    let result = unsafe { SetThreadDesktop(desktop) };
    unsafe { CloseDesktop(desktop).ok() };
    
    if result.is_err() {
        // 失败时尝试直接打开 Winlogon 桌面
        use windows::Win32::System::StationsAndDesktops::OpenDesktopW;
        let winlogon_name: Vec<u16> = "Winlogon".encode_utf16().chain([0]).collect();
        let winlogon = unsafe { 
            OpenDesktopW(
                windows::core::PCWSTR(winlogon_name.as_ptr()),
                Default::default(),
                false,
                access,
            )? 
        };
        unsafe {
            SetThreadDesktop(winlogon)?;
            CloseDesktop(winlogon).ok();
        }
    }
    
    Ok(())
}
```

### 3.3 添加 secure_attention 管道命令

**文件**: `services/input-service/src/main.rs`

在 `handle_client()` 中添加新命令处理：

```rust
"secure_attention" => {
    let result = qinglink_windows::secure_desktop::send_sas();
    match result {
        Ok(()) => write_response(&mut stream, true, "SERVICE_OK", "SAS已发送"),
        Err(error) => write_response(&mut stream, false, "SAS_FAILED", &error.to_string()),
    }
    .inspect(|_| { std::mem::forget(stream); })
}
```

### 3.4 qinglink-server 暴露 secure_attention API

**文件**: `crates/qinglink-server/src/server/ws_handler_system.rs`

添加处理：
```rust
WsClientRequest::SecureAttention => {
    match crate::platform::send_sas() {
        Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
        Err(error) => WsServerMessage::Response(WsServerResponse::error("SAS_FAILED", &error.to_string())),
    }
}
```

### 3.5 协议层添加 SecureAttention 请求

**文件**: `crates/qinglink-protocol/src/` 和 `crates/qinglink-server/src/protocol/requests.rs`

添加 `WsClientRequest::SecureAttention` 变体。

---

## 文件修改清单

| 文件 | 修改类型 |
|------|----------|
| `apps/desktop/src/main.rs` | 添加 subsystem 属性 |
| `apps/desktop/src/logging.rs` | 条件编译 stderr 输出 |
| `apps/desktop/src/tray.rs` | 添加气泡通知方法 |
| `apps/desktop/src/view.rs` | 弹窗置顶 + 自动关闭 |
| `crates/qinglink-windows/src/secure_desktop.rs` | 添加 send_sas() |
| `services/input-service/src/main.rs` | 增强桌面切换 + secure_attention 命令 |
| `crates/qinglink-server/src/server/ws_handler_system.rs` | 添加 SecureAttention 处理 |
| `crates/qinglink-server/src/protocol/requests.rs` | 添加 SecureAttention 请求 |
| `crates/qinglink-protocol/src/requests.rs` | 添加 SecureAttention DTO（如需要） |

---

## 验证方式

### 阶段一验证
1. `cargo build --release`
2. 运行编译后的 exe，确认无控制台窗口弹出
3. 确认托盘图标正常显示

### 阶段二验证
1. 启动移动端发起配对
2. 确认弹窗置顶显示在所有窗口前面
3. 确认配对完成后弹窗自动关闭

### 阶段三验证
1. 锁定电脑 (Win+L)
2. 通过移动端发送输入，确认能在锁屏界面输入密码
3. 触发 UAC 操作，确认能在 UAC 界面操作
