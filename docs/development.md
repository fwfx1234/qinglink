# 开发与排障

配套阅读：[`architecture.md`](architecture.md)（结构）与 [`web-api.md`](web-api.md)（接口）。

---

## 1. 环境要求

| 组件 | 要求 |
| --- | --- |
| 操作系统 | Windows 10/11 x64（桌面端、Windows 服务、系统控制都需要） |
| Rust | stable，需支持 edition 2024（当前验证于 1.96） |
| Node.js | 20+（只为构建前端；仅在有 `web-ui/dist` 时可以没有） |
| Flutter/Dart | 仅移动端需要 |
| Visual C++ 构建工具 | `rusqlite` 使用 `bundled`，需要 C 编译器 |

---

## 2. 构建

### 2.1 桌面端

```powershell
# 仓库根目录
cargo build -p qinglink-desktop --bin QingLink
# 产物
target\debug\QingLink.exe
```

`build.rs` 会自动处理前端：`web-ui/dist` 缺失或比源码旧时，它会依次尝试
`npm run build`（Vite）与 `npm run build:fallback`（无子进程的 Rollup 构建），
然后把产物内嵌进 exe。两者都失败时会内嵌一个说明性占位页并打 `cargo:warning`，
**不会**让后端构建失败。

只改 Rust 时不会重复构建前端。想强制重建前端：

```powershell
$env:QINGLINK_BUILD_WEB = '1'
cargo build -p qinglink-desktop --bin QingLink
```

想跳过前端构建、直接使用现有 `dist`：

```powershell
$env:QINGLINK_SKIP_WEB_BUILD = '1'
```

`build.rs` 在两种"内嵌的东西已经过期"的情况下会打 `cargo:warning`，都不阻断构建：

- **服务资源旧于服务源码**：`apps/desktop/assets/input/QingLinkService.exe` 比
  `services/qinglink-service` 或 `crates/qinglink-windows` 的源码旧——按第 5 节
  「改了服务端代码」的步骤刷新。这两个源码树都在服务二进制里：管理协议在
  `services/`，图标管线在 `crates/qinglink-windows`，任何一处改动没重装服务，
  手机端和桌面端都会看到"没有图标"。
  该文件是构建产物、**不入库**（`.gitignore` 的 `/apps/desktop/assets/input/*.exe`）。
  全新 clone 里它根本不存在，这种情况 `build.rs` **不报警**（只有
  `QINGLINK_REQUIRE_INPUT_ASSETS=1` 才会让构建失败），症状出现在运行时：桌面端
  「安装服务」提示资源缺失，先按本文档 §2.3 生成它。
- **内嵌界面旧于前端源码**：`web-ui/dist/index.html` 落后于 `web-ui/src`。这发生在
  `QINGLINK_SKIP_WEB_BUILD=1` 或两条 npm 构建都失败时——此时内嵌的是**上一次的 `dist`**，
  不是占位页。所以**"构建成功"不等于"界面是新的"**；占位页只在 `dist/index.html`
  根本不存在时出现（`--self-check` 的 `web_interface.placeholder` 只回答"有没有界面"，
  不回答"新不新"）。

`build.rs` 还负责版本资源：`qinglink.rc` 只保留应用清单，`FILEVERSION` / `FileVersion`
由 `qinglink_version::version_info_rc` 按当前构建号生成后嵌入（见 §2.4）。版本号一变，
`build.rs` 就会因 `../../crates/qinglink-version/build-number.txt` 变化而重跑。

本仓库没有构建脚本：跨语言编排全部由 `build.rs` 承担，日常只用
`cargo` / `npm` / `flutter` 的原生命令（[`AGENTS.md`](../AGENTS.md) §7）。

### 2.2 前端

```powershell
cd apps\desktop\web-ui
npm install
npm run build          # 标准路径：vue-tsc 类型检查 + vite build
npm run build:fallback # 备选路径：无子进程构建，用途见下
npm run dev            # 开发服务器：默认 5173，/api 与 ws 已代理到桌面端 3722
```

**为什么有两条构建路径。** Vite 的 TS/SFC 转换依赖 esbuild，而 esbuild 通过派生
子进程启动自己的服务。在禁止 Node 派生带管道子进程的环境里，`vite build` 会以
`Error: spawn EPERM` 失败，栈顶是 esbuild 的 `ensureServiceIsRunning`。
`build/build.mjs` 用 Rollup 程序化 API + `@vue/compiler-sfc` + terser（全部在进程内）
复刻同一套生产构建，产出相同形状的 `dist/`。

在正常机器上两条路径都可用，`npm run build` 是首选；`build.rs` 会先试它。

前端构建产物不可用时的症状很好认：界面显示"界面资源未构建"的说明页，
`--self-check` 的 `web_interface.placeholder` 为 `true`。

**开发环境的两套 web 资源。** 界面在开发期有两个互不相同的来源，别把它们混起来
（Tauri 用 `devUrl` / `frontendDist` 两个字段表达的就是这件事）：

| 模式 | 资源来源 | 谁在服务 | 地址 |
| --- | --- | --- | --- |
| Dev | `web-ui/src`，Vite 内存编译 + HMR | Vite dev server | <http://127.0.0.1:5173> |
| Integrated | `web-ui/dist`，被 `build.rs` 内嵌进 exe | 桌面端本地 Web 服务 | <http://127.0.0.1:3722> |

- 只改 Vue / TS / CSS 时**不要** `cargo build`：5173 直接跑源码，Rust 端只当 API。
  此时 3722 上仍是旧界面，属正常现象。
- 5173 依赖桌面端在跑：`vite.config.ts` 已把 `/api` 与 WebSocket 代理到 3722。
  端口被占用时 Vite 会自动换端口，以终端输出为准。
- 要让 exe 带上新前端：`npm run build`，或直接 `cargo build -p qinglink-desktop`
  （`build.rs` 检测到 `src/` 里有文件比 `dist/index.html` 新就会自动重建，
  `node_modules` 缺失时还会先跑 `npm install`）。
- `QINGLINK_SKIP_WEB_BUILD=1` 会直接用现有 `dist`，可能在 exe 里静默内嵌过时界面；
  只在明确知情时使用。
- `dist/` 不入库（见 `web-ui/.gitignore`），仓库始终从源码构建。

### 2.2.1 主题（浅色默认 / 深色 / 跟随系统）

界面提供两套完整主题，右上角可切换：**浅色（默认）→ 深色 → 跟随系统** 三态循环。
**未持久化偏好时用的是浅色，不是跟随系统**——否则系统深色的机器永远看不到亮色主题。

- 颜色令牌全部在 `src/styles/light.css` 与 `src/styles/dark.css` 中成对定义，选择器是
  `[data-theme='light']` / `[data-theme='dark']`；几何与排版在 `src/styles/tokens.css`。
  **组件里不得出现颜色字面量**（`rgba(...)` / `#rrggbb`），需要新颜色就在两个主题文件里
  各加一个同名令牌——只加一个会让另一个主题静默继承错的值。
- 浅色令牌有对比度底线：正文与次要文本 ≥4.5:1，语义色落在自己的 soft 衬底上同样
  ≥4.5:1（所以 soft 用低 alpha + 压深的语义色）。控件边界只需视觉可辨（≈2:1），
  键盘可辨识性由 3px `--accent-ring` 焦点光环保证。改 `--text-3`、`--warning`
  或 soft 系列的 alpha 时务必重新核算。
- 几何与层次集中在 `src/styles/tokens.css`：圆角（控件 10 / 卡片 16 / 浮层 18）、
  控件高度（36 / 30）、字号（基准 14）、字重、动效曲线。要换气质优先改这里，
  不要在组件里写死尺寸。层次用 `--shadow-card` 与 `--surface-2`/`--surface-3` 表达；
  深色下 `--shadow-card` 为 `none`，靠亮度差分层。
- 主题写在 `<html data-theme>` 上，由 `src/stores/theme.ts` 维护；偏好存在
  `localStorage['qinglink.theme']`，不经过桌面端接口（管理界面是浏览器页面，服务端不需要
  知道它长什么样）。
- `index.html` 里有一段**内联**脚本在样式表解析前定主题（含默认值），否则首屏会闪一帧
  另一个主题。它读的键名与默认值必须与 `theme.ts` 的 `STORAGE_KEY` / `DEFAULT_MODE` 一致。
- `--color-scheme` 随主题切换，让浏览器原生控件（滚动条、下拉框、日期选择器）跟随。
  **不要在组件里单独写 `color-scheme: dark` 或给图片类控件加 `filter: invert(...)`**：
  两者都会在浅色主题下坏掉。

改主题时至少肉眼过一遍：概览（状态行/统计）、日志页（级别色与两个日期选择器）、
应用列表（行悬停与搜索框清除按钮）、模态框（遮罩）、禁用态按钮。

样式产物必须带内容哈希（`npm run build` 与 `npm run build:fallback` 都是如此）：
后端对 `/assets/**` 发一年 `immutable`，固定名样式表会让改样式后刷新也看不到变化。

### 2.3 服务与移动端

```powershell
cargo build --release -p qinglink-service --bin QingLinkService
cd apps\mobile; flutter pub get; flutter run
```

发布时把服务放到 `apps/desktop/assets/input/QingLinkService.exe`，再以
`QINGLINK_REQUIRE_INPUT_ASSETS=1` 构建桌面端。该文件是 `services/` 的**构建产物，不入库**：
全新 clone 必须先按上面的 `cargo build -p qinglink-service` 加 `Copy-Item` 生成它。
服务建议做 Authenticode 签名，但安装**默认不校验**
签名——本机自建的未签名产物可以直接通过界面「修复服务」安装。要强制签名，在启动桌面端之前设置
`QINGLINK_REQUIRE_SIGNED_INPUT=1`，此时提权助手会拒绝未签名的服务资源或桌面端。无论哪种模式，
内嵌资源的 SHA256 都会核对。

### 2.4 版本号

桌面端与服务端共用一个版本号，形如 `0.1.0+13`：

- `+` 之前是 `Cargo.toml` 的 `[workspace.package] version`，**手工维护**：功能变了改
  minor，只有修 bug 改 patch，破坏性变更改 major。
- `+` 之后是构建号，由 `crates/qinglink-version/build.rs` **自动维护**：被跟踪的源码
  （`crates/*`、`apps/desktop/src`、`apps/desktop/web-ui/src`、`apps/desktop/assets`、
  `services/qinglink-service/src`、`protocol/`，以及根 `Cargo.toml`、桌面端的 `.rc`
  与应用清单）指纹一变就加一，同一份源码重复构建不会变。`assets/input/` 下的服务
  二进制是构建产物，**不计入**指纹——否则刷新服务资源会让桌面端的号无谓地涨。

状态记在 `crates/qinglink-version/build-number.txt`（第一行构建号，第二行上次的源码指纹）。
**它与代码一起提交**：每次改动跑一次构建，这个文件就会变，提交时带上即可。如果它没被提交，
换一台机器构建会从旧号继续，版本号可能出现回退。

**CI 发布构建是例外**：流水线从 Release tag（形如 `desktop-v0.1.0+29`）解析出构建号，
写入环境变量 `QINGLINK_CI_BUILD_NUMBER` 后再构建。此时 `build.rs` 直接用注入的构建号，
既不算源码指纹也不写回状态文件——因为 CI checkout 出来的文件修改时间全部重置，照常算
指纹必然与仓库里存的对不上而白白自增，导致产物版本与 tag 不一致。本地开发不设这个变量，
行为不变。

版本号出现在四个地方，都取自同一份 `qinglink_version::VERSION`：

| 位置 | 字段 |
| --- | --- |
| 桌面端 `GET /api/admin/v1/health`、`/diagnostics` | `version` |
| 桌面端界面（仪表盘「服务端版本」、诊断页「桌面端版本 / 服务端版本」） | 同上 |
| 服务端 `GET /api/v2/health` | `app_version` |
| Windows 文件属性（两个 exe） | `FILEVERSION` / `FileVersion` |

Windows 版本资源不再写死在 `.rc` 里：`apps/desktop/qinglink.rc` 只保留应用清单，版本信息由
`qinglink_version::version_info_rc` 在构建脚本中生成；服务端没有静态 `.rc`，完全由
`services/qinglink-service/build.rs` 生成。`0.1.0+13` 对应 `FILEVERSION 0,1,0,13`。

不要再用 `env!("CARGO_PKG_VERSION")` 展示版本：它只有语义版本，看不出两次改动。

检查更新时对比版本用 `qinglink_version::is_newer(candidate, current)`：它把语义版本三段与
`+` 后的构建号一起比较。构建号虽然语义上是 semver 的构建元数据，但本项目拿它当源码改动
的递增序号，不参与比较就会把 `0.1.0+28`→`0.1.0+29` 误判成同版本。前端与移动端按同一套
语义各自实现比较函数。

### 2.5 发布流水线（GitHub Actions）

两条流水线按 tag 前缀分流，tag 是版本号唯一真相源，发布前先确认 tag 与源码版本一致：

| 流水线 | 触发 | 校验 | 构建与产物 |
| --- | --- | --- | --- |
| `.github/workflows/desktop-release.yml` | `desktop-v<语义版本>+<构建号>` | tag 语义版本 == `Cargo.toml` 的 `workspace.package.version` | Windows runner：服务 → 刷新内嵌资源 → 桌面端；产出 `QingLink.exe` + `SHA256SUMS` |
| `.github/workflows/mobile-release.yml` | `mobile-v<语义版本>+<构建号>` | tag 版本 == `pubspec.yaml` 的 `version` | Ubuntu + Flutter：APK + 同名 `.apk.sha256`；先跑 `flutter analyze` |

桌面流水线把 tag 里的构建号写入 `QINGLINK_CI_BUILD_NUMBER`（见 §2.4），否则 CI checkout
后文件时间全变、指纹自增会让产物版本与 tag 对不上。**新增更新相关代码时**，发布前记得：
版本号递进（`Cargo.toml` / `pubspec.yaml`）、按本机构建更新并提交 `build-number.txt`、
确认两端 release asset 各只含两个文件：程序本体（`QingLink.exe` / APK）与校验清单
（GNU sha256sum 行格式，可用 `sha256sum -c` 校验）。桌面端清单名为 `SHA256SUMS`；
移动端刻意命名为 `<APK 文件名>.sha256`——旧版客户端只会请求「APK 地址 + `.sha256`」，
改名会让它们 404。客户端按精确文件名匹配这两个资产；不再发布独立更新器 exe，
旧客户端无法从新 release 自动更新。

移动端还有一条额外约束：**APK 必须由固定密钥签名，且签名方案 v1/v2/v3 全开**。
Android 只允许同包名 + 同签名的 APK 覆盖安装，而 runner 每次全新、AGP 默认 debug 密钥
每次现生成，等于发一版换一次签名，已安装的客户端会在更新时报签名冲突。密钥与密码刻意
随源码提交（`apps/mobile/android/keystore/qinglink-release.jks` + `key.properties`）。
另外 AGP 在 `minSdk >= 24` 时默认关闭 v1（JAR）签名，产物不含 `META-INF/*.RSA`，
部分安装器会提示「没有签名文件」，所以显式打开 v1/v2/v3。CI 打包后用
`apksigner verify -v --print-certs` 校验证书 SHA-256 并断言 v1、v2 均存在，另外跑
`unzip -t` 与 `zipalign -c 4`，任一不符直接失败。轮换密钥等于让老用户卸载重装，
改指纹时要同步改工作流。详见 `apps/mobile/README.md`。

移动端安装链路的两个约束：

- **只打真机 ABI**（`--target-platform android-arm,android-arm64`）。universal 包默认
  还带 x86_64 引擎（约 20 MB），只有模拟器会用；去掉后包体从 ~58 MB 降到 ~36 MB，
  慢链路下被截断的概率显著下降。
- **应用内安装走 `PackageInstaller`**（`apps/mobile/android/.../InstallChannel.kt`），
  不再用 `open_filex` 的 `ACTION_VIEW`：只有前者能把系统的真实失败原因
  （`INSTALL_PARSE_FAILED_NO_CERTIFICATES` 等）带回来。安装前客户端还会核对
  字节数 → SHA-256 → 包名 → versionCode → 签名者，坏包不会交给安装器。
  「安装包未包含任何证书」的语义是**一个证书都收不到**（文件被截断/不是 APK），
  与「签名不一致」（`INSTALL_FAILED_UPDATE_INCOMPATIBLE`）是两回事，排查见
  `apps/mobile/README.md`。

---

## 3. 运行

```powershell
# 直接运行（数据进 %LOCALAPPDATA%\QingLink）
target\debug\QingLink.exe

# 用独立数据目录，便于测试且不污染真实配置
$env:QINGLINK_DATA_DIR = "$PWD\.tmp-data"
target\debug\QingLink.exe
```

**在 workspace 根目录用 `cargo run --bin QingLink` 时日志会打在该终端。** debug 构建是
**console 子系统**进程（release 才是 GUI 子系统、无控制台窗口），因此它像普通命令行程序
一样继承启动它的终端，`tracing` 的 stderr 层直接可见，默认等级是 `info`：

```powershell
cargo run --bin QingLink              # 终端上直接看到 INFO 及以上的日志
$env:RUST_LOG = 'debug'               # 或 $env:QINGLINK_LOG_LEVEL = 'debug'
cargo run --bin QingLink              # 同一终端里追加 DEBUG 记录
```

裸级别名（`debug`）会被展开成 `QingLink=debug,qinglink=debug`——`EnvFilter` 的目标匹配
区分大小写，只写一个会丢掉另一半日志。不设任何变量时按 `info` 运行；`RUST_LOG` 与
`QINGLINK_LOG_LEVEL` 都只在**非空白**时才算设置，所以 shell 里留下一个空的 `RUST_LOG=`
不会把日志关掉。

> 历史实现曾在**所有**构建里使用 GUI 子系统，再于启动时 `AttachConsole(ATTACH_PARENT_PROCESS)`
> 并把标准句柄指向 `CONOUT$`。实测这条路必然丢日志：重定向到控制台就绕开了 `cargo run`
> 读的那个管道，而在低完整性会话里 `AttachConsole` 与 `CreateFileW("CONOUT$")` 都会失败。
> 已删除，改为按 profile 选择子系统。

启动后：

- 托盘出现 QingLink 图标（Windows 11 可能先放进折叠区，可拖到任务栏固定）；
- 管理界面在 <http://127.0.0.1:3722>；
- 托盘左键单击 = 打开界面，右键 = 菜单；
- 手机发起配对时弹出置顶窗口显示配对码，配对成功 / 超时 / 取消后自动关闭
  （时序见 `docs/architecture.md` 9.4）；窗口没弹出来时，配对码仍在管理界面概览页。

**`Shell_NotifyIcon(NIM_ADD)` 被拒绝时会自动打开浏览器**；拒绝后仍会按定时器重试一段
时间（覆盖"Explorer 刚重启"这类可恢复情况）。若是**重试无法修复**的拒绝——最常见的是
完整性级别不匹配——则不再空耗重试预算，立刻记录一条 `ERROR` 并给出处理办法，同时把原因
写进 `/api/admin/v1/overview` 的 `tray.blocked_reason`。

**完整性级别由启动它的宿主决定。** 终端是 medium 不等于从它启动的子进程是 medium：
子进程继承父进程的令牌，而 low 进程无法创建 medium 子进程。启动日志与
`--self-check` 都会打印本进程与实际通知区域所有者的级别，直接以它为准：

```text
INFO QingLink: QingLink 桌面端启动 ... integrity=low shell_integrity=medium
```

| 端口 | 用途 |
| --- | --- |
| 3722 | 本地管理界面（仅回环） |
| 3721 | 服务对外协议（HTTP + WS） |
| 3723 | UDP 局域网发现 |

**退出会请求服务停止**，这与"重启"不同。验证时若不想影响正在运行的服务，
请强制结束进程（`Stop-Process`）而不是走界面上的"退出"。

**正在运行的实例会锁住 exe。** `cargo run` 在这种情况下的表现是构建阶段报
`error: failed to remove file ...\target\debug\QingLink.exe` / `拒绝访问。 (os error 5)`
然后立刻退出——这**不是**应用启动失败，而是 Windows 不允许覆盖正在运行的镜像：

```powershell
Stop-Process -Name QingLink -Force   # 或者从托盘菜单退出
cargo run --bin QingLink
```

即使 exe 没有被锁（源码未变，cargo 直接复用已有产物），第二次启动也会命中单实例检查：
进程写一条 `已有实例在运行，激活并退出` 后立即退出。

---

## 4. 自检

```powershell
# debug 构建是 console 子系统，直接继承终端，无需重定向
target\debug\QingLink.exe --self-check

# 只探一次服务命名管道，打印完整错误链与原始 OS 错误码
target\debug\QingLink.exe --check-service
```

`--self-check` 不会触碰服务，可以在应用运行期间执行；它对"托盘图标不显示"给出可判读的结论：

| 字段 | 判读 |
| --- | --- |
| `tray_icon.monochrome` | `true` = 图标已退化成单色，检查是否有人重新引入了黑色模板转换 |
| `tray_icon.has_brand_blue` / `has_white_glyph` | 品牌图形是否完整 |
| `tray_icon.icon_handle_ok` | 像素是否成功合成为 `HICON`；`false` 时 `icon_handle_error` 给出原因 |
| `tray_icon.png` | 渲染结果同时写成 PNG，可直接用眼睛确认 |
| `tray_registration.visible` | `Shell_NotifyIcon(NIM_ADD)` 是否被 shell 接受；**`false` = 图标没有注册** |
| `tray_registration.verified` | `Shell_NotifyIconGetRect` 是否确认图标已在通知区域（折叠区也算已注册） |
| `tray_registration.rect` | `GetRect` 返回的真实坐标；`null` 表示图标在折叠区或未注册 |
| `tray_registration.error_code` | 失败时 shell 的 `GetLastError`（`5` = 拒绝访问） |
| `tray_registration.integrity` | **本进程**令牌完整性；与任务栏不一致时图标一定注册不上 |
| `tray_registration.shell_integrity` | 通知区域所有者（通常是 `explorer.exe`）的完整性 |
| `tray_registration.attempts` | 已尝试的重试次数：启动时首次注册就被永久拒绝为 `0`（还没走过重试）；重试路径上为该轮已尝试次数 |
| `tray_registration.blocked_reason` | 非空 = 重试无法修复，字段本身就是处理办法 |
| `log_dir_writable` | 日志目录是否可写 |
| `web_interface.placeholder` | 前端是否缺失 |
| `service.*` | 服务安装/运行/二进制哈希比对结果，以及回环健康协议 `network_ready` |

同样的托盘诊断也出现在 `/api/admin/v1/overview` 的 `tray` 字段里，仪表盘的"安全输入
服务体检"卡片会直接显示——包括"注册被拒绝"的完整原因，所以不装命令行也能看到图标为什么
不见了。

服务**管道**是否可连由 `--check-service` 单独回答（`--self-check` 刻意不触碰服务）。
它的 `probe.outcome` 直接给出结论：

| `probe.ok` | `probe.code` | 含义与处理 |
| --- | --- | --- |
| `true` | 服务端返回的结论码（`capabilities` 为 `OK`） | 管理通道可用，桌面端能完整工作 |
| `false` | `PIPE_UNREACHABLE` | 连不上管道、请求失败，或管道通了但服务协议版本与桌面端不匹配；`probe.error` 含完整错误链与 OS 错误码 |

`probe.outcome` 会按 `install_state` 把 `PIPE_UNREACHABLE` 再分成"服务没装 / 未运行"与
"环境禁止管道"两种处理建议；`probe.code` 本身不区分。

---

## 5. 排障

### 启动后浏览器（Edge）弹出"需要管理员权限"或卡住不响应

这是**桌面端以管理员令牌运行**导致的，不是浏览器坏了。原因链：

1. 桌面端用 `ShellExecute` 打开 <http://127.0.0.1:3722>；
2. 如果当前进程带管理员令牌，浏览器就会以管理员身份启动；
3. Chromium 内核（Edge/Chrome）**拒绝以管理员身份运行**——Edge 会弹出要求管理员权限
   的提示框并停止响应。

诊断（`--self-check` 不含这项，看日志）：

```powershell
# 运行中的桌面端会在打开界面时记录一行，elevated 就是答案
# %LOCALAPPDATA%\QingLink\logs\qinglink.<日期>.log 或终端
#   INFO QingLink: 打开管理界面 url="http://127.0.0.1:3722" elevated=true
```

处理：

- **首选**：不要以管理员身份启动桌面端。它不需要提权——安装/启停服务时会临时拉起
  UAC 辅助进程（`--input-service-control`），自身保持普通用户权限即可。
  从"以管理员身份运行"的终端里 `cargo run` 是最常见的触发方式。
- 已有修复兜底：进程检测到自己以管理员运行时，会改用**资源管理器**的
  `explorer.exe` 将 URL 交给用户 shell（`qinglink_windows::browser`），这样浏览器落在用户
  完整性级别上，不再被拒绝。该路径依赖 explorer.exe 在运行，失败时会回退到直接启动，
  并在日志里留一条 `无法通过资源管理器打开浏览器`。
- 如果提示仍在，说明浏览器实例本身有问题（配置文件损坏、或已有一个管理员实例在跑）：
  先结束所有 `msedge.exe` 再试，或直接手动打开 <http://127.0.0.1:3722>。

### 界面打不开 / 浏览器显示连接被拒绝

按顺序查：

1. 进程还在吗？`Get-Process QingLink`
2. 端口被占了吗？`Get-NetTCPConnection -LocalPort 3722 -State Listen`
3. 看日志：`%LOCALAPPDATA%\QingLink\logs\qinglink.<日期>.log`（也可在界面的日志页下载）

如果进程活着但端口没人监听，看日志里有没有 `Web 管理服务异常退出`——那通常意味着
路由配置断言失败或端口被占用。路由在 worker 线程启动**之前**于调用线程构建，
所以路由写错会在启动期就暴露，而不是留下一个"看着健康"的进程。

### 托盘图标看不见

按顺序查，每一步都有明确判据：

1. **图标是不是被拒绝了？** 跑 `--self-check`，看 `tray_registration.visible`：
   - `false` → 图标没有注册成功（不是折叠区问题）。`error_code` 给出原因，`integrity` /
     `shell_integrity` 给出两边的完整性级别，`blocked_reason` 会直接写出处理办法。
   - 最常见的是**完整性级别不匹配**：进程以管理员运行而任务栏不是，或进程被降级。
     如果 `integrity=low` 而 `shell_integrity=medium`，说明启动宿主把它降级了。
     注意"我的终端是 medium"并不等于"它启动的进程是 medium"——子进程继承父进程的令牌，
     而 low 父进程无法创建 medium 子进程；本会话里 DSH 给的 pwsh 就是 low，从它启动
     `cargo run` 得到的必然是 low。请以应用自己测出的 `integrity` 为准：
     从资源管理器双击 `target\debug\QingLink.exe`，或在普通用户终端里执行
     `target\debug\QingLink.exe --self-check` 复核。应用使用 `asInvoker`，不会请求管理员
     权限；低完整性宿主无法通过应用自身的清单安全地提升到任务栏级别。
     同一次运行里的启动日志也会打出这两个级别：
     `INFO QingLink: QingLink 桌面端启动 ... integrity=low shell_integrity=medium`。
   - `true` 且 `verified=true` → `Shell_NotifyIconGetRect` 已确认图标在通知区域；
     `rect` 是它的真实坐标。
2. **先看 Windows 11 的折叠区**（任务栏的 `^` 或"其他系统托盘图标"），把它拖到任务栏上
   固定。注册被拒绝或重试停止时桌面端会自动打开管理界面。
3. **图标像素对不对？** 确认 `--self-check` 的 `monochrome` 为 `false`、
   `icon_handle_ok` 为 `true`。`monochrome` 为 `true` 说明有人把图标转成了纯黑——这正是
   本项目历史上的真实缺陷，见 [`architecture.md` 第 6 节](architecture.md#6-托盘子系统含历史缺陷)。
4. **图标消失了又出现？** Explorer 重启会让图标消失；托盘窗口监听 `TaskbarCreated`，并在收到通知后重新执行 `NIM_ADD`（同时解除"永久拒绝"状态，因为新的通知区域可能与本进程同级）。启动时 Shell 尚未就绪导致的短暂失败也会定时重试；**重试无法修复的拒绝**（`error_code=5` 且完整性级别不一致）不会空耗预算，会立刻记录 `ERROR` 并尝试打开管理界面。

托盘不再依赖 `tray-icon` 之类的托盘 crate，而是直接用 Win32 外壳 API，原因和实现要点见
[`architecture.md` 第 6.1 / 6.1.1 节](architecture.md)。

### 界面显示"服务未安装/未连接"

`/api/admin/v1/*` 中依赖服务的接口返回 `503 SERVICE_UNAVAILABLE`，这是**预期**
降级，不是崩溃。检查：

```powershell
sc.exe query QingLinkService
```

若未安装，在界面的"安全桌面输入服务"卡片里点"安装/修复"（会弹 UAC）。

### 应用列表、设备列表全部报 SERVICE_UNAVAILABLE

桌面端与服务之间走命名管道。有三个常见原因，`--check-service` 会把它们区分开：

1. 服务确实没运行；
2. **执行环境禁止打开命名管道**。受限沙箱会阻止子进程访问命名管道，此时
   `/api/admin/v1/service` 的 `last_error` 会是 `cannot connect to local pipe`，
   而 `install-state` 却报告服务正在运行——这个组合就是环境限制的典型特征，
   不是代码缺陷。
3. **装了旧版服务**（协议不匹配）。此时管道能连上、服务也在跑，但服务不认识 v2 管理
   协议，应答里没有 `version` / `request_id`。典型特征是 `--check-service` 输出

   ```json
   "probe": { "ok": false, "error": "服务协议版本不匹配（服务 0 / 桌面端 2），请升级或修复服务" },
   "install_state": { "available": true, "service_binary_matches": true, "service_running": true }
   ```

   也就是"体检说可用、链路说未连接"。`GET /api/admin/v1/service` 的 `state` 会是
   `protocol_mismatch`，界面（概览页与「诊断」页）会直接给出「修复服务」动作。
   **这是重装能修好的那一种**：修好的前提是 `apps/desktop/assets/input/QingLinkService.exe`
   本身是新的（见下节）。

最快的确诊方式：

```powershell
target\debug\QingLink.exe --check-service
```

它会打印完整错误链（包含原始 OS 错误码，例如 `CreateFileW failed for local pipe:
Access is denied. (os error 5)`）。**"服务在运行 + 管道被拒"时不要重装服务**，
换个执行环境即可：在普通用户会话（Windows Terminal / VS Code 集成终端）中直接运行
`target\debug\QingLink.exe`，或让受限会话临时获得完全访问权限。

同一原因还会连带一件事，同样是预期降级而非故障：`--self-check` 的
`log_dir_writable` 为 `false`（写不进去数据目录），日志只落到 stderr，文本文件一天也
不会有。

### 手机显示"未检测到显示器"

`get_displays` 由**会话代理**在用户会话里执行。服务本体跑在 Session 0，那里没有交互
桌面，`EnumDisplayDevicesW` 枚举不到任何"连接到桌面"的显示器，所以服务进程自己做枚举
只能是空列表。排查顺序：

1. 确认会话代理存在：

   ```powershell
   Get-Process QingLinkService | Select-Object Id, SessionId
   ```

   应当有两行：服务本体 `SessionId 0`，会话代理 `SessionId 1`。只有一行说明代理没起来，
   在服务日志里搜 `Session agent launched` 与 `Failed to launch session agent`。
2. 代理随服务启动，有几秒延迟；开机后立刻打开控制中心可能失败一次，点右上角刷新即可。
3. 仍失败时看控制中心错误行的文案：`AGENT_REQUEST_TIMEOUT` 表示代理管道 3 秒内没应答；
   `未检测到连接到 Windows 桌面的显示器` 表示代理自己也没枚举到（PC 确实没接显示器，
   或代理被拉到了非交互会话）。

服务升级后必须让**服务与代理一起重启**（界面「诊断」页的「修复服务（升级）」），
只重启桌面端不会换掉正在运行的代理。

### 改了服务端代码，界面却仍显示"服务未连接"

`QingLinkService.exe` **不由桌面端的构建流程编译**，而是作为资源内嵌在
`apps/desktop/assets/input/`（构建产物，不入库）并由"安装 / 修复"释放到
`C:\Program Files\QingLink\Input`。因此只改 `services/` 或 `crates/` 是不会生效的，
"修复服务"也只会重新装回同一个旧二进制。

正确的顺序（顺序不可颠倒）：

```powershell
# 1. 先退出正在运行的桌面端（它占着 target\debug\QingLink.exe，不退出无法重新链接）

# 2. 重建服务并刷新内嵌资源
cargo build -p qinglink-service --bin QingLinkService
Copy-Item .\target\debug\QingLinkService.exe .\apps\desktop\assets\input\QingLinkService.exe -Force

# 3. 重建桌面端（build.rs 会重新内嵌资源；资源比服务源码旧时会打印 cargo:warning）
cargo build -p qinglink-desktop --bin QingLink

# 4. 启动新的桌面端，在「诊断」页点「修复服务（升级）」，确认 UAC

# 5. 验证
target\debug\QingLink.exe --check-service   # 期望 probe.ok = true
```

第 3 步必须**最后**做：`C:\Program Files\QingLink\Input\trusted-desktop.json` 里记录的是
桌面端可执行文件的 SHA256，服务的每次安装都会重新登记它。任何一次 `cargo build` 都会让
哈希失效，此时 `/api/admin/v1/service` 的 `state` 变成 `unauthorized`，界面会提示重新登记；
再点一次"修复服务"即可。

### 退出桌面端后服务也没了

这是设计行为，不是故障：桌面端的**显式退出**会向服务下发 `shutdown`，服务连同它的会话代理
一起停止（`service_lifecycle::shutdown(true)`）；托盘"重启桌面端"只释放租约，不停服务。
下次启动桌面端时 `prepare_service()` 会走免提权的 `start_service_direct()` 把服务拉回来
（服务 ACL 授予交互用户 `SERVICE_START`），所以正常流程**不会**再弹 UAC——`sc.exe query
QingLinkService` 显示 STOPPED 时不必急着"修复服务"。

修复服务时若报 `无法安装 QingLinkService.exe: 另一个程序正在使用此文件 (os error 32)`，
说明停服务之后仍有 `QingLinkService.exe --session-agent` 子进程在退出中、继续映射着服务镜像。
安装已按 500ms 间隔重试最多 15 秒，正常情况下几秒内就会成功；只有超过重试窗口才会报这个错。

### 日志页只有最近 5000 条

这是设计如此：**日志不写数据库**，只在内存里保留最近 5000 条供界面查询与实时推送，
更早的记录在 `%LOCALAPPDATA%\QingLink\logs\qinglink.<日期>.log` 里，用界面上的"下载"
或 `/logs/download` 取。

日志页可以在两个来源之间切换：桌面端（上面的内存缓冲 + 实时推送）与
`QingLinkService`（`%ProgramData%\QingLink\logs\QingLinkService.log.<日期>`，按需解析、
不是实时流）。服务端目录对交互用户是可读的，所以桌面端直接读文件，不走 IPC——这也避开了
管理管道 64 KiB 的帧上限。「导出诊断包」会把诊断报告与两个来源的日志尾部拼成一个纯文本
文件。

日志记录按**写入顺序倒序**返回（实现是 `entries.iter().rev()`，顺序由进程内单调递增的
`id` 决定，并不按 `timestamp_ms` 排序）；`id` 只在**当前进程内**单调递增（每次启动从 1
重新开始），因此跨重启不要用 `id` 比较新旧。

### 权限被拒的日志

```
QingLink cannot write logs to ...: 拒绝访问。 (os error 5); using stderr
```

日志落到 stderr，其余功能正常。常见于受限沙箱或数据目录 ACL 异常。debug 构建在终端里
运行时会看到这行；通过资源管理器双击启动时没有控制台，这行会丢失，此时用
`--self-check` 的 `log_dir_writable` 判断。

### `cargo run` 之后出现 `exit code: 1`

`exit code: 1` 只是 Cargo 对桌面端启动错误的汇总，不是具体原因。当前 debug 构建会在
返回 1 之前打印 `QingLink 启动失败：...`，请看它前面紧邻的那一行。若需要完整保留输出，
在 workspace 根目录执行：

```powershell
$env:QINGLINK_STARTUP_TRACE = '1'
$env:QINGLINK_LOG_LEVEL = 'debug'
cargo run --bin QingLink 2>&1 | Tee-Object .\qinglink-startup.log
```

正常启动不会退出，会持续运行并监听 `http://127.0.0.1:3722`。若仍立即退出，把
`qinglink-startup.log` 中最后一个 `[qinglink-startup]` 面包屑和 `QingLink 启动失败：` 行一并提供。

如果错误是 `Error code 14: Unable to open the database file`，通常表示当前终端的
Windows 完整性级别低于 `%LOCALAPPDATA%\QingLink` 文件，或数据库目录/文件不可写。
请从普通交互式 PowerShell 或 Windows Terminal 启动，不要从低完整性宿主启动；也不要把
桌面端设置为“以管理员身份运行”。可先验证：

```powershell
$env:QINGLINK_STARTUP_TRACE = '1'
.\target\debug\QingLink.exe --self-check
```

自检中的 `log_dir_writable` 必须为 `true`。如果只是开发环境的低完整性限制，可临时使用
workspace 下的可写目录验证程序本身：

```powershell
$env:QINGLINK_DATA_DIR = "$PWD\.qinglink-data"
cargo run --bin QingLink
```

### `cargo run` 之后"什么都没有"

按顺序排除三种完全不同的原因：

1. **终端没有任何输出** —— 检查是不是真的跑的是刚构建出的那个进程；被运行的实例锁住
   exe 时，输出是构建错误（见第 3 节），进程根本没起来。
2. **进程起来了但看不到日志** —— 只可能发生在没有父控制台时（双击启动、或从 GUI 直接
   拉起）。debug 构建是 console 子系统、从一开始就继承父进程的标准句柄，所以在终端里
   `cargo run` 的日志必然可见，`cmd /c QingLink.exe > file` 这类外层重定向也能正常收到；
   要同时看屏幕又落盘就用管道（`cargo run ... | Tee-Object`）。若仍不可见，把
   `--self-check` 的输出附上一起排查。
3. **日志有但托盘没有图标** —— 看第 4 节的 `tray_registration.visible` 判读，以及
   "托盘图标看不见"一节。只有 NIM_ADD 被拒绝时应用才会自动打开浏览器。

若日志有了但**进程停在启动中间**（端口不监听、界面打不开），设
`QINGLINK_STARTUP_TRACE=1` 再跑一次，stderr 会逐阶段打印带耗时的面包屑：

```powershell
$env:QINGLINK_STARTUP_TRACE = '1'
target\debug\QingLink.exe
```

最后一个面包屑就是卡住的位置。正常值守则形如：

```text
[qinglink-startup] +  0.000s main entered
[qinglink-startup] +  0.034s preferences loaded
[qinglink-startup] +  0.048s logs initialised
[qinglink-startup] +  0.068s web listener reporting ready
```

**启动路径上不允许出现无期限的阻塞操作。** 这条规矩来自一次真实的踩坑：日志历史曾经
放在独立的 `logs.db` 里，启动时打开 SQLite 连接池在数据目录被拒绝的环境下会**无限期
挂住**——进程活着、端口不开、日志为空，看起来像"什么都没启动"。现在日志只写文本文件。新增
任何启动期 IO（磁盘、管道、注册表、进程）时都要给它一个期限，或者挪出启动路径。

---

## 6. 校验

```powershell
cargo fmt --all --check
cargo check --workspace --all-targets

# 文件规模（大文件禁令见 AGENTS.md §2）：没有校验脚本，用原生命令现算
Get-ChildItem -Recurse -Include *.rs,*.dart,*.ts,*.vue |
  Where-Object { $_.FullName -notmatch 'node_modules|\\target\\|\\build\\|\\dist\\|\\test\\' } |
  ForEach-Object {
    $limit = if ($_.Extension -eq '.vue') { 400 } else { 500 }
    $lines = (Get-Content $_).Count
    if ($lines -gt $limit) { [pscustomobject]@{ Lines = $lines; Limit = $limit; Path = $_.FullName } }
  } | Sort-Object Lines -Descending

# 前端 / 移动端静态检查
cd apps\desktop\web-ui; npm run typecheck
cd apps\mobile; flutter analyze
```

仓库**不需要新增测试代码，也不运行测试套件**（见 [`AGENTS.md`](../AGENTS.md) §4）；
既有测试文件保留，需要时自行运行。日常验证一律走上面的静态检查，`cargo test` 不在交付
门槛内，因此本文档不再固化通过/失败数字（它会随环境漂移）。

前端：

```powershell
cd apps\desktop\web-ui
npm run typecheck
```

`cargo clippy --workspace --all-targets` 当前是**干净**的（0 告警、0 错误）。工作区把
`unwrap_used` / `dbg_macro` / `todo` 设为 deny，把 `missing_docs`、
`missing_debug_implementations`、`missing_errors_doc`、`missing_panics_doc`、
`redundant_clone`、`needless_pass_by_value` 设为 warn；新增代码请保持零输出。

---

## 7. 改动的约定

改这个项目时请遵守以下几条，它们各自都对应一次真实的踩坑：

1. **不要 `unwrap()`。** 工作区 lint 把 `unwrap_used` 设为 `deny`。
2. **任何修改 `scanner_apps` 的路径都必须调用 `reload_visible_catalog`**，
   否则移动端会继续看到旧目录。
3. **携带敏感字段的事件类型要手写 `Debug`。** `PendingPairing` 刻意不实现 `Debug`，
   为的是 PIN 不可能被打进日志。
4. **不要在 main 线程之外做托盘操作**；保持唤醒的 `SetThreadExecutionState`
   只由服务常驻线程调用，桌面端不应再出现该 API。
5. **不要在主循环里做阻塞 IO**；管道调用最长 3 秒。
6. **Web 处理函数不能直接碰只属于 main 线程的状态**，走 `HostCommand`。
7. **新增日志 sink 时保持"失败即降级"**：日志坏了不能让应用起不来。
8. **改了接口就同步改 `docs/web-api.md`。** 那份文档是契约，不是事后说明。
9. **不要移除 `tray/icon.rs` 里的颜色处理**（见第 5 节）。
10. **不要给管理接口加认证**，也不要把它绑到非回环地址；安全性来自"本机可访问"
    加上服务的管道授权。
11. **代码不得存在大文件。** 单文件硬上限 500 行（`.vue` 400 行，函数 120 行），
    完整规则见 [`AGENTS.md`](../AGENTS.md) §2。既有超限文件列在 §2.3 的表格里，
    **只许减少、不许增加**；新增文件没有豁免。本仓库**没有行数校验脚本**（§7），
    改完用 §2.4 的原生命令自查，并在同一次改动里同步 §2.3 表格的数字。
12. **文档必须跟随项目同步。** 改代码的同一次改动里把对应文档一起改掉，映射表见
    [`AGENTS.md`](../AGENTS.md) §3.1；接口契约类文档未同步按阻断项处理（第 8 条是它在
    管理接口上的一个特例）。
13. **不新增测试代码、不运行测试套件。** 细节与替代的静态检查见
    [`AGENTS.md`](../AGENTS.md) §4；既有测试文件保留。
14. **提交时带上 `crates/qinglink-version/build-number.txt`。** 它记录当前构建号与源码
    指纹；漏提交会让另一台机器从旧号继续，版本号出现回退（见 §2.4）。
