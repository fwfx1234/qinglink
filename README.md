# 轻联 QingLink

轻联（QingLink）是一套面向可信局域网的 Windows PC 远程控制工具，包含 Windows 桌面伴侣、高权限 Windows 输入服务和 Flutter 手机客户端。手机可发现并配对 PC，之后可远程控制应用、进程、系统、电源及键鼠输入。QingLink 是独立项目，不依赖 QingQi crate，也不向 QingQi 主程序加载远控插件。

> 项目当前处于开发阶段。远控 HTTP/WebSocket 通信尚未启用 TLS/WSS，应只在可信局域网使用；不要将服务端口暴露到公网。

## 功能概览

- UDP 局域网发现；通过 PC 确认的 6 位验证码配对，长期令牌存放在手机平台安全存储中。
- PC 应用库、Steam 游戏与自定义目录扫描、应用启动、进程查看与管理。
- 文件浏览、系统与电源控制、显示/音频设置、鼠标键盘和触控板输入。
- Windows 服务支持登录前令牌认证及安全桌面输入；新设备配对仍需桌面端确认。
- 手机端提供 Android/iOS Flutter 应用，并包含可选的本机红外遥控功能（取决于设备能力）。

## 架构

```text
Flutter 手机端 (apps/mobile)
       | UDP 发现 + HTTP 配对 + WebSocket 控制
       v
QingLinkService (LocalSystem) <--> Windows 会话输入代理
       ^
       | 本机命名管道
Windows 桌面端 (apps/desktop, GPUI)
       |
       +-- qinglink-server: Axum API、控制服务和 Windows 平台适配
       +-- qinglink-storage: SQLite 路径、迁移和连接池

共享 Rust crate: qinglink-protocol / qinglink-domain / qinglink-discovery
移动端传输 DTO: apps/mobile/lib/src/protocol
协议契约与夹具: protocol/
```

Rust workspace 位于仓库根目录。`qinglink-windows` 封装 Windows 系统能力，`qinglink-ui` 提供共享 UI 常量；`qinglink-server` 的非 Windows 平台使用存根实现。线上传输契约以 `protocol/qinglink-v2.schema.json` 为准，细节见 [`protocol/v2.md`](protocol/v2.md)。

默认 TCP 控制端口为 `3721`，UDP 发现端口为 `3723`。`/api/v2/events` 用于控制事件，`/api/v2/input` 用于高频输入。配对请求验证码有效期为 60 秒。

## 环境要求

- Windows 10/11 x64 用于运行桌面端、输入服务以及完整系统控制功能。
- Rust stable（支持 Rust 2024 edition）和 Cargo。
- Flutter/Dart（SDK 约束见 [`apps/mobile/pubspec.yaml`](apps/mobile/pubspec.yaml)）；Android 或 iOS 构建还需对应平台 SDK。
- 发布构建需准备签名后的 `QingLinkService.exe`。正式发布建议使用 Authenticode 签名。

## 开发

从仓库根目录运行 Windows 桌面端（服务默认监听端口 `3721`）：

```powershell
cargo run -p qinglink-desktop --bin QingLink
```

开发版缺少输入服务发布资源时仍可启动，但安全输入安装入口不可用。运行移动端：

```powershell
cd apps/mobile
flutter pub get
flutter run
```

常用检查：

```powershell
# 仓库根目录
cargo fmt --check
cargo test --workspace

# apps/mobile 目录
flutter analyze
flutter test
```

`cargo clippy --workspace --all-targets -- -D warnings` 目前不是通过状态：仓库已有 lint/documentation warning，严格模式会将其作为错误处理。不要将该命令的失败误认为测试失败。

## 使用

1. 在 Windows PC 启动 QingLink 桌面端。需要服务托管的输入和登录前控制时，在桌面端的服务设置中安装/启动 QingLinkService，并按 UAC 提示确认。
2. 在手机启动 QingLink，选择发现到的 PC 并发起配对。
3. 在 PC 桌面端确认手机请求，将显示的 6 位验证码输入手机。验证码在 60 秒后失效。
4. 配对成功后选择该 PC 即可使用控制功能；PC 更换局域网地址后，客户端会根据稳定 PC ID 更新地址。

## 发布构建

正式构建前，将签名后的 `QingLinkService.exe` 放到 `apps/desktop/assets/input/`。仓库提供的脚本会构建服务、复制发布资源并构建桌面端：

```powershell
.\scripts\build-release.ps1
```

资源检查可单独运行：

```powershell
.\scripts\verify-input-assets.ps1
```

桌面端服务安装会请求 UAC，并将服务安装到 `%ProgramFiles%\QingLink\Input`。不要在不可信网络暴露服务；协议 v2 当前使用 HTTP/WebSocket 明文传输。

## 项目进度

- 已实现 Rust Windows 桌面端、Windows 输入服务、Flutter 手机端、v2 发现/配对/控制协议及 SQLite 存储。
- Rust workspace 单元、集成及协议夹具测试已覆盖发现以外的核心服务行为；少数会启动或终止真实进程的功能测试默认忽略。
- 最近一次检查：`cargo fmt --check`、`cargo test --workspace`、`flutter analyze` 和 `flutter test` 均通过；Rust 测试 99 项通过、4 项忽略，Flutter 测试 78 项通过。
- 发布/签名、真实 Windows 服务安装、安全桌面交互及各手机设备上的端到端验证仍需在目标环境确认。

## 协议与维护资料

- [`protocol/v2.md`](protocol/v2.md)：API、发现、配对和安全约定。
- [`protocol/README.md`](protocol/README.md)：跨语言 DTO 与 Schema/fixture 验证说明。
- [`apps/mobile/README.md`](apps/mobile/README.md)：移动端配对与开发说明。

如需从旧 QingQi 安装迁移，可单独审阅并以管理员身份运行 [`scripts/remove-legacy-qingqi-input.ps1`](scripts/remove-legacy-qingqi-input.ps1)。该脚本会停止/删除旧服务并卸载匹配的旧驱动；请确认目标机器和驱动后再执行。
