# 轻联 QingLink

面向可信局域网的 Windows PC 远程控制工具：Windows 桌面伴侣 + 高权限 Windows 输入服务 +
Flutter 手机客户端。手机发现并配对 PC 后，可远程控制应用、进程、系统电源与键鼠输入。
QingLink 是独立项目，不依赖 QingQi crate，也不向 QingQi 主程序加载远控插件。

> 开发阶段。远控 HTTP/WebSocket 通信尚未启用 TLS/WSS，只应部署在可信局域网，
> 不要将服务端口暴露到公网。

## 结构

**数据只有一个所有者：Windows 服务。桌面端与手机都是它的客户端。**

```text
                    ┌────────────────────────────────────────────┐
                    │ QingLinkService.exe   LocalSystem Windows 服务│
                    │ · 全部业务数据（应用目录、重命名、令牌…）      │
                    │ · 局域网协议 TCP 3721 / 发现 UDP 3723        │
                    │ · 命名管道：桌面管理通道                       │
                    └───────┬───────────────────────┬────────────┘
        命名管道（授权桌面端）  │                       │  TCP/UDP（已配对手机）
                    ┌───────┴────────────┐   ┌──────┴─────────────┐
                    │ QingLink.exe        │   │ Flutter 手机端      │
                    │ · 托盘图标           │   │ · 控制端            │
                    │ · 配对提示弹窗       │   │ · 不持有业务数据     │
                    │ · 本地 Web 服务 3722 │   └────────────────────┘
                    │ · 日志采集与查询     │
                    └───────┬────────────┘
                            │ HTTP + WS /api/admin/v1/*
                    ┌───────┴────────────┐
                    │ 浏览器管理界面       │
                    │ Vue 3 SPA（内嵌 exe）│
                    └────────────────────┘
```

由此推出两条对使用者重要的结论：

1. **数据跟随 PC，不跟随控制端。** 应用改名、隐藏、常用（我的库）、首页置顶、图标、
   已配对设备都存在服务端；换手机、换浏览器、换用户会话，只要连的是同一台 PC，看到的就是
   同一份数据。手机端只保留连接信息、输入法与红外遥控这类设备相关的本地设置。
2. **手机是控制端，详细管理在本机。** 应用改名/隐藏/删除/常用属于服务端管理面，由本机
   Web 界面暴露；桌面端自己不直接读写业务数据库（它对 `%ProgramData%\QingLink` 没有权限，
   这正是结构上的强制）。

完整的模块划分、线程模型与设计取舍见 [`docs/architecture.md`](docs/architecture.md)。

## 端口与数据

| 端口 | 用途 |
| --- | --- |
| 3722 | 桌面端本地管理界面（**仅回环**，无认证；安全性来自只允许本机访问 + 服务的管道授权） |
| 3721 | 服务对外协议：`/api/v2/events` 控制、`/api/v2/input` 高频输入 |
| 3723 | UDP 局域网发现 |

| 数据 | 位置 |
| --- | --- |
| 应用目录、重命名、隐藏、常用与首页置顶、图标、已配对设备 | `%ProgramData%\QingLink\qinglink.db`（服务所有，ACL 仅 SYSTEM + 管理员） |
| 桌面端设置 | `%LOCALAPPDATA%\QingLink\qinglink.db` |
| 手机端连接信息、输入法/触控板设置、快捷键方案、红外遥控 | 手机本地（令牌在平台安全存储） |
| 日志文本 | `%LOCALAPPDATA%\QingLink\logs\qinglink.<日期>.log`（按天轮转，长期保留） |

`QINGLINK_DATA_DIR` 可覆盖数据目录，用于测试或便携部署。

## 功能

- UDP 局域网发现；由 PC 确认的 6 位验证码配对（60 秒失效，桌面锁屏时不下发），
  长期令牌存放在手机平台安全存储中。
- 应用库、Steam 游戏与自定义目录扫描、启动、进程查看与管理；扫描按"每个目录一个主程序"
  给出推荐项，手机可逐个屏蔽并在「已屏蔽」里恢复。
- 游戏可保存主显示器启动分辨率；启动前尝试切换，跟踪游戏进程退出后恢复原模式。
- **图标按条目管理**：可逐条重扫或指定图标文件，来源会一并记录（`shell` 表示只拿到默认
  图标），落库前统一规范为 128×128；换图标立即对所有客户端生效。
- **常用（我的库）归服务端**：手机「库 → 常用」、手机首页置顶与管理端「常用」是同一份
  数据，升级时会把旧版本存在手机上的列表自动迁移。
- **本机 Web 管理界面**：左侧导航 + 大留白布局，浅色/深色双主题；链路诊断与一键修复、
  配对确认、应用目录管理（新增/启动/重命名/隐藏/删除/常用/置顶/批量操作/重扫图标、
  逐条目详情抽屉；选文件由桌面端弹出**原生选择窗口**，不必手输路径）、设备撤销、
  日志查看与下载、诊断包导出、桌面端设置；长说明收进 `?` tooltip，不占版面。
- **系统托盘**：打开 QingLink、重启服务、保持唤醒、退出；直接注册 Win32 通知区域图标，
  explorer 重启后自动重新注册。
- **配对提示弹窗**：手机发起配对时桌面端弹出置顶窗口显示 6 位配对码（可取消），
  配对成功 / 超时 / 锁屏后自动关闭；浏览器没打开也能完成配对。
- **日志子系统**：内存环形缓冲（5000 条，供界面查询与实时推送）+ 按天轮转的文本日志；
  **日志不写数据库**。
- 文件浏览、系统与电源控制、显示/音频设置、鼠标键盘与触控板输入。
- Windows 服务支持登录前令牌认证与安全桌面输入；新设备配对仍需桌面端确认。
- 手机端为 Android/iOS Flutter 应用，含可选的本机红外遥控功能。

## 环境要求

- Windows 10/11 x64（桌面端、输入服务与完整系统控制功能）。
- Rust stable（需支持 edition 2024）与 Cargo。
- Node.js 20+（仅构建 Web 界面；已有 `web-ui/dist` 时非必需）。
- Windows SDK / Visual C++ 构建工具（`rusqlite` 使用 `bundled`，需要 C 编译器）。
- Flutter/Dart（约束见 [`apps/mobile/pubspec.yaml`](apps/mobile/pubspec.yaml)）；
  Android 或 iOS 构建还需对应平台 SDK。
- 发布构建需准备 `QingLinkService.exe`（建议 Authenticode 签名；安装默认不校验，见「发布构建」）。

## 快速开始

```powershell
# 仓库根目录
# 1. 先生成安全输入服务资源（构建产物，不在版本控制里，全新 clone 必须做一次）
cargo build -p qinglink-service --bin QingLinkService
Copy-Item .\target\debug\QingLinkService.exe .\apps\desktop\assets\input\QingLinkService.exe -Force

# 2. 构建并运行桌面端；会自动构建并内嵌 Web 界面
cargo run --bin QingLink
```

跳过第 1 步桌面端照样能跑，但"安装服务"会提示「未内嵌安全输入服务」：
`apps/desktop/assets/input/QingLinkService.exe` 是 `services/qinglink-service` 的产物，
本仓库只放源代码与资源，不跟踪任何构建产物。

debug 构建是 console 子系统进程，日志默认按 `info` 输出到运行它的终端，同时写入
`%LOCALAPPDATA%\QingLink\logs\`；release 构建是无控制台的纯托盘形态。启动后监听
<http://127.0.0.1:3722>，托盘左键单击即打开界面。

```powershell
$env:RUST_LOG = 'debug'                   # 或 $env:QINGLINK_LOG_LEVEL = 'debug'
target\debug\QingLink.exe --self-check    # 托盘/日志/服务体检，回答"托盘图标为什么看不见"
target\debug\QingLink.exe --check-service # 只探一次服务命名管道并打印完整错误链
```

移动端：

```powershell
cd apps\mobile
flutter pub get
flutter run
```

使用流程、托盘图标看不见与"服务未连接"的排障、完整性级别、自检字段判读、命名管道被受限
环境拒绝等，见 [`docs/development.md`](docs/development.md) 第 4、5 节。

## 开发与校验

改完代码按 [`AGENTS.md`](AGENTS.md) §6 的静态检查收口（本仓库不新增测试代码、不运行测试套件）：

```powershell
cargo fmt --all --check
cargo check --workspace --all-targets

# 文件规模：本仓库没有校验脚本（AGENTS.md §7），按 §2.3 的清单与 §2.4 的原生命令自查
(Get-Content <file>).Count

# 前端
cd apps\desktop\web-ui
npm install
npm run typecheck  # 只做类型检查
npm run build      # vue-tsc 类型检查 + vite build
npm run dev        # 开发服务器：默认 5173，/api 与 ws 已代理到桌面端 3722（需先跑桌面端）

# 移动端
cd apps\mobile; flutter analyze
```

**本仓库没有构建脚本**：跨语言编排全部由 [`apps/desktop/build.rs`](apps/desktop/build.rs) 承担
（等价于 Tauri 的 `tauri.conf.json` 加 CLI），日常只用 `cargo` / `npm` / `flutter` 的原生命令。
规则见 [`AGENTS.md`](AGENTS.md) §7。

前端构建有两条路径：标准 `vite build` 与 `npm run build:fallback`（无子进程的 Rollup
构建，用于受限环境阻止 esbuild 派生服务进程的情形），`build.rs` 会依次尝试。细节与排障见
[`docs/development.md`](docs/development.md)。

**改了 `services/` 或 `crates/` 之后，桌面端不会自动带上新的服务端。** `QingLinkService.exe`
是内嵌资源（构建产物，不入库），必须重建并刷新它，"修复服务"才会真正升级：

```powershell
# 1. 退出正在运行的桌面端（它占着 target\debug\QingLink.exe）
# 2. 重建服务并刷新内嵌资源
cargo build -p qinglink-service --bin QingLinkService
Copy-Item .\target\debug\QingLinkService.exe .\apps\desktop\assets\input\QingLinkService.exe -Force
# 3. 最后重建桌面端（build.rs 会在资源比服务源码旧时给出 cargo:warning）
cargo build -p qinglink-desktop --bin QingLink
# 4. 启动后在界面「诊断」页点「修复服务（升级）」，确认 UAC
```

顺序不可颠倒：服务的安装会登记桌面端可执行文件的 SHA256，先装服务再重建桌面端会让登记
失效，界面会提示"桌面端未授权"，需要再修复一次。

## 发布构建

本仓库不提供发布脚本（[`AGENTS.md`](AGENTS.md) §7），发布就是下面四条命令：

```powershell
# 1. 构建服务
cargo build --release -p qinglink-service --bin QingLinkService

# 2.（建议）对服务做 Authenticode 签名，并复核结果（Status 必须是 Valid）
signtool sign /fd SHA256 /tr <时间戳服务器> /td SHA256 /a .\target\release\QingLinkService.exe
Get-AuthenticodeSignature .\target\release\QingLinkService.exe

# 3. 刷新桌面端的发布资源
Copy-Item .\target\release\QingLinkService.exe .\apps\desktop\assets\input\QingLinkService.exe -Force

# 4. 构建桌面端，缺资源直接失败（自更新执行器已内置于本 exe）
$env:QINGLINK_REQUIRE_INPUT_ASSETS = '1'
cargo build --release -p qinglink-desktop --bin QingLink
```

产物 `target\release\QingLink.exe` 已内嵌 Web 界面，单文件即可分发；服务安装到
`%ProgramFiles%\QingLink\Input`，安装时会请求 UAC。签名状态可用
`Get-AuthenticodeSignature .\apps\desktop\assets\input\QingLinkService.exe` 检查。
安装与「修复服务」**默认不校验** Authenticode 签名，debug 与 release 都能安装本机自建产物
（内嵌资源的 SHA256 始终核对）；要强制签名，在启动桌面端之前设置
`$env:QINGLINK_REQUIRE_SIGNED_INPUT = '1'`，此时提权助手会拒绝未签名的服务资源或桌面端。

**自动发布。** 仓库提供两条 GitHub Actions 流水线（无需发布脚本）：

| 流水线 | 触发 tag | 产物 |
| --- | --- | --- |
| `.github/workflows/desktop-release.yml` | `desktop-v0.1.0+29` | `QingLink.exe` + `SHA256SUMS` |
| `.github/workflows/mobile-release.yml` | `mobile-v1.0.0+5` | Android APK + 同名 `.apk.sha256` |

两端各只发布两个资产：程序本体与校验清单（GNU sha256sum 行格式）。桌面端自更新
由下载好的新 exe 副本以 `--self-update` 启动完成，不再单独发布更新器 exe；旧版本客户端
无法从新 release 自动更新，需手动下载安装。移动端清单刻意命名为 `<APK>.sha256`，让
已安装的旧版客户端也能取到校验文件。两份清单都可用 `sha256sum -c` 校验。

tag 是版本号唯一真相源：桌面流水线从 tag 解析构建号注入 `QINGLINK_CI_BUILD_NUMBER`，
保证产物版本与 tag 一致（详见 [`docs/development.md`](docs/development.md) §2.4）。
两端启动后自动检查更新，设置/更新页也提供手动检查入口：发现新版本即后台下载并校验，
下载完成后由用户手动触发安装（安装会退出程序或拉起系统安装器）。移动端 iOS 不支持侧载，
仅提示跳转发页面。详见 [`docs/architecture.md`](docs/architecture.md) §9.5。

## 文档索引

| 文档 | 内容 |
| --- | --- |
| [`AGENTS.md`](AGENTS.md) | **工程约束唯一权威**：大文件禁令与行数上限、文档同步映射、测试策略、完成定义 |
| [`docs/architecture.md`](docs/architecture.md) | **结构总纲**：组件、数据归属、桌面端进程模型、托盘与日志子系统、关键时序 |
| [`docs/web-api.md`](docs/web-api.md) | **桌面管理接口契约**：REST 端点、WebSocket 事件、错误码、前端约束 |
| [`docs/development.md`](docs/development.md) | 构建、运行、`--self-check` 自检、排障、改动约定 |
| [`protocol/v2.md`](protocol/v2.md) | 移动端对外协议（配对、发现、控制） |
| [`protocol/requests.md`](protocol/requests.md) | 上行请求类型索引：95 个 `WsClientRequest` 变体的线格式名、载荷字段、响应类型与错误码 |
| [`protocol/README.md`](protocol/README.md) | 跨语言 DTO 与 Schema/fixture 验证说明 |
| [`apps/mobile/README.md`](apps/mobile/README.md) | 移动端配对与开发说明 |
| [`.opencode/rules/`](.opencode/rules) | Rust / Dart / 架构的分语言编码规范细节 |
