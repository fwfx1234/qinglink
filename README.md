# 轻联 QingLink

轻联是独立的局域网远程控制产品，包含 Windows PC companion、Flutter 手机端、输入服务和协议契约。它不依赖 `qingqi` 的任何 crate；QingQi 主程序也不会加载远控插件。

## 本地运行

在本目录执行：

```powershell
cargo run -p qinglink-desktop --bin QingLink
```

默认服务端口为 `3721`，局域网发现使用 UDP `3723`。Windows 服务启动后负责 TCP 远控接口和 UDP 发现，桌面端仅通过本机管道控制服务。手机首次连接时发起配对请求，桌面端显示 6 位匹配码；匹配码 60 秒后失效。

## 工作区

- `apps/desktop`：GPUI 窗口与服务组合根
- `apps/mobile`：Flutter 控制端
- `crates/qinglink-domain`：配对状态机和领域 trait
- `crates/qinglink-protocol`：API v2 DTO 与消息契约
- `crates/qinglink-server`：Axum HTTP/WS 服务和 Windows 远控能力
- `crates/qinglink-storage`：SQLite 路径、迁移和仓储
- `crates/qinglink-discovery`：UDP 局域网发现
- `services/qinglink-service`：高权限输入服务（系统服务与会话代理共用一个 EXE）

## 安全桌面输入发布

桌面端“服务设置”提供安全输入组件的安装、修复和卸载入口。开发构建在
缺少签名驱动时仍可运行，但安装按钮会显示“缺少发布包”。正式构建需要先
把签名后的输入服务、键盘驱动和鼠标驱动放入
`apps/desktop/assets/input`，然后执行：

```powershell
.\scripts\verify-input-assets.ps1
```

安装入口会请求 UAC，并把资源释放到 `%ProgramFiles%\QingLink\Input`。

## 检查

```powershell
cargo fmt --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace
flutter analyze
flutter test
```

如需清理旧 QingQi 原型服务，请以管理员身份执行：

```powershell
.\scripts\remove-legacy-qingqi-input.ps1
```
