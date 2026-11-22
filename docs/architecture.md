# QingLink 架构

本文档描述 QingLink 当前的进程划分、数据归属与模块职责。**如果只读一份文档就
要改这个项目，读这一份。** 接口细节见 [`web-api.md`](web-api.md)，构建与排障见
[`development.md`](development.md)。

---

## 1. 设计原则

这四条原则解释了后面所有的结构决定，改动代码时请先确认没有违反它们。

1. **数据只有一个所有者：服务。**
   应用目录、用户重命名、隐藏状态、已配对设备全部由 `QingLinkService.exe`
   （LocalSystem）读写。桌面端和移动端都是它的客户端。
2. **控制端可以换，数据不跟着走。**
   换一台手机、换一个浏览器、甚至换一台 PC 上的另一个用户会话，只要连的是
   同一台 PC，看到的就是同一份数据。这是"数据跟随 PC"的字面含义。
3. **桌面端是薄壳。**
   它只做五件事：托盘图标、配对提示窗口、本地 Web 服务、日志采集、把管理请求代理给服务。
   它**不**直接读写业务数据——它对业务数据（`qinglink.db`、`config\`）没有权限，
   这正是结构上强制这条原则的原因。唯一例外是 `%ProgramData%\QingLink\logs\`：
   那里 ACL 额外给 `BUILTIN\Users` 授予只读，界面因此能直接读服务日志（见第 3 节）。
4. **日志与界面绝不能让核心功能起不来。**
   日志目录不可写、前端没构建、托盘注册失败，都必须降级而不是让进程退出。

---

## 2. 组件与数据流

```text
┌──────────────────────────────────────────────────────────────┐
│ QingLinkService.exe          LocalSystem，开机自启 Windows 服务│
│                                                              │
│  · SQLite 数据所有者（应用目录、令牌、设备）                    │
│  · 游戏启动模式协调与进程监视；退出后恢复显示模式                │
│  · 局域网协议：TCP 3721（HTTP + WS /api/v2/*）                 │
│  · 局域网发现：UDP 3723                                       │
│  · 命名管道服务端 \\.\pipe\QingLinkControlV2（桌面管理通道）     │
│  · 会话输入代理（安全桌面 / 登录前输入）                        │
└───────────┬──────────────────────────────┬───────────────────┘
            │ 命名管道（授权桌面端）          │ TCP/UDP（已配对手机）
            │ DesktopCommand/ServiceResponse│ /api/v2/*
┌───────────┴──────────────────┐   ┌───────┴───────────────────┐
│ QingLink.exe                 │   │ 移动端 App (Flutter)       │
│ 普通用户权限                  │   │ · 控制端，不持有业务数据     │
│                              │   └───────────────────────────┘
│  · 系统托盘图标               │
│  · 配对提示弹窗（不依赖浏览器） │
│  · 本地 Web 服务 :3722        │
│  · 日志采集/查询/持久化        │
│  · 与服务的租约心跳            │
└───────────┬──────────────────┘
            │ HTTP + WS /api/admin/v1/*
┌───────────┴──────────────────┐
│ 浏览器管理界面 (Vue 3 SPA)     │
│ · 内嵌在 exe 中，无需部署       │
└──────────────────────────────┘
```

### 三套接口的边界

| 接口 | 端口 | 使用者 | 能改业务数据吗 |
| --- | --- | --- | --- |
| `WS /api/v2/events`、`/api/v2/*` | 3721 | 移动端 | 是：重命名、新增、删除、隐藏、常用/置顶、图标；另有启动应用、管理进程、键鼠 |
| `HTTP/WS /api/admin/v1/*` | 3722 | 本机浏览器 | 是：重命名、新增、删除、隐藏、常用/置顶、图标、撤销设备 |
| UDP 发现 | 3723 | 移动端 | 否 |

移动端与桌面管理端读写的是**同一份服务端目录数据**（`scanner_apps`），差别只在
界面与暴露的字段，不在权限；完整命令清单见 5.2 节。

---

## 3. 数据存在哪里

| 数据 | 位置 | 所有者 | 说明 |
| --- | --- | --- | --- |
| 应用目录、重命名、隐藏、删除标记、常用/置顶、游戏启动显示模式 | `%ProgramData%\QingLink\qinglink.db` → `scanner_apps` | 服务 | 全部客户端共享 |
| 游戏运行期间的显示器恢复快照 | 同上 → `settings` 的 `launch_display.snapshot` | 服务 | 服务启动或游戏退出时恢复 |
| 最近启动时间与次数 | 同上 → `launcher_usage` | 服务 | `item_id` → `last_launched_at`、`launch_count`，"最近/最常用"排序用 |
| 每类扫描完成时间 | 同上 → `scanner_scan_state` | 服务 | `kind` → `completed_at`；快照的 `scan` 字段由它而来 |
| Steam 游戏 | 同上 → `steam_games` | 服务 | |
| 已配对设备与令牌 | 同上 → `tokens_v2` | 服务 | |
| PC 身份、配对 PIN | `%ProgramData%\QingLink\config\` | 服务 | |
| 桌面端设置（调试日志） | `%LOCALAPPDATA%\QingLink\qinglink.db` → `settings` | 桌面端 | 每用户 |
| 保持唤醒开关 | `%ProgramData%\QingLink\qinglink.db` → `settings.power.keep_awake` | 服务 | 全局，桌面端与移动端共用 |
| 桌面端日志文本（可读，也是唯一的历史） | `%LOCALAPPDATA%\QingLink\logs\qinglink.<日期>.log` | 桌面端 | 按天轮转，长期保留 |
| 服务端日志文本 | `%ProgramData%\QingLink\logs\QingLinkService.log.<日期>` | 服务 | 桌面端**只读**直读，不经命名管道 |
| 移动端首页快捷键 | 手机 `SharedPreferences` → `home_shortcuts` | 移动端 | 最多保存 12 个组合及加入顺序；方案页可直接固定或移除，触发仍通过服务端 `key_event` |

`%ProgramData%\QingLink` 的 ACL 只给 SYSTEM 与内置管理员完全权限
（`crates/qinglink-storage/src/paths.rs` 里用 `icacls` 强制），业务数据
（`qinglink.db`、`config\`）因此对普通用户权限的桌面端**在结构上不可写**，
它不可能绕过服务直接改业务数据。唯一的例外是 `logs\` 子树：同一个函数额外给
`BUILTIN\Users` 授予 `(OI)(CI)(RX)`，好让管理界面在服务出问题时能直接读到
`QingLinkService.log.<日期>` 说明原因，而不是只报一句"服务失败"。

`QINGLINK_DATA_DIR` 环境变量可以覆盖数据目录，用于测试与便携部署。

### 3.1 `scanner_apps` 的关键字段

```sql
id              TEXT PRIMARY KEY   -- 可能是 exe 路径，所以绝不放进 URL 路径段
name            TEXT NOT NULL      -- 扫描得到的原始名（重命名不写这里）
original_name   TEXT NOT NULL      -- 同上，保留用于恢复
display_name    TEXT               -- 用户重命名；NULL/空 表示"用 name"
exe_path        TEXT NOT NULL
icon_base64     TEXT               -- 懒提取；扫描时不批量提图标。恒为 128×128 PNG：
                                   -- 素材居中、最长边铺满画布（见 docs/web-api.md §4.16）
icon_path       TEXT               -- 图标取自哪个文件；手动指定时就是该路径
icon_source     TEXT               -- manual/steam/shortcut/executable/image/shell/none
icon_updated_at INTEGER            -- 上次解析图标的 Unix 秒，客户端据此破图标缓存
args            TEXT NOT NULL      -- 启动参数 JSON
category        TEXT               -- 忽略大小写等于 'game' 归 game，否则归 app
source          TEXT               -- StartMenu/Desktop/Steam/Custom/User...
is_hidden       INTEGER            -- 隐藏：不出现在移动端，但数据与重命名都保留
is_deleted      INTEGER            -- 软删除：可被后续扫描重新发现
is_user_added   INTEGER
is_favorite     INTEGER            -- 常用（"我的库"）
is_pinned       INTEGER            -- 置顶到手机首页；置顶会自动加入常用
launch_width    INTEGER            -- 可选的游戏启动分辨率宽度
launch_height   INTEGER            -- 可选的游戏启动分辨率高度
launch_refresh_millihz INTEGER     -- 可选的刷新率，单位毫赫兹
created_at      INTEGER NOT NULL
updated_at      INTEGER NOT NULL
```

**读取时永远用 `COALESCE(NULLIF(display_name,''), name)`。**
所以"恢复原名称"的实现就是 `display_name = ''`，不需要额外分支。

`kind` 只有两个值，由 `AppScanner::catalog_kind(category)` 判定：
`category` **忽略大小写**等于 `game` → `game`，否则 `app`。移动端内部还分
steam/software/other 等更细的标签，管理面不暴露这些。

---

## 4. 桌面端进程模型

`apps/desktop` 是**一个进程、多个角色**。理解线程归属是修改它的前提。

```text
main 线程（也是唯一拥有 Windows 消息队列的线程）
├─ 托盘图标：创建、持有、销毁都必须在这个线程
├─ 配对提示弹窗：手机发起配对时显示，配对成功 / 超时 / 取消后自动隐藏
├─ Windows 消息泵：每 50ms 抽一次，托盘与弹窗事件都依赖它
├─ 主循环：处理 HostCommand / TrayAction / PromptEvent / 状态同步
└─ 退出序列（`exit.rs`）：停 IPC worker → 服务租约 → 数据库 → 日志 flush（最后）

qinglink-service-ipc 线程（1 个）
└─ 独占命名管道；所有服务命令串行经过它

qinglink-web 线程（1 个，内含多线程 Tokio runtime）
├─ Axum HTTP + WebSocket 服务
├─ pump_service_events：服务事件 → 浏览器广播
└─ pump_logs：日志合并成 200ms 一帧

服务租约线程（1 个，ServiceLifecycleManager）
├─ 启动时准备服务，之后每 2 秒发一次心跳 / 注册父进程
└─ 失败按指数退避重试（上限 15 秒）；环境受限与协议不匹配各只报一次

临时线程
├─ 提权服务操作（阻塞在 UAC 辅助进程上）
├─ 服务安装状态体检
├─ 配对取消（`qinglink-pairing-cancel`）：主线程不能等的那次管道往返
└─ 文件选择对话框（`qinglink-file-picker`，Shell 选择器要求单线程套间）
```

**这里没有日志写库线程。** 日志不写数据库（见 7.1 节），落盘由
`tracing_appender` 的 non-blocking writer 自己持有的线程完成，桌面端只保留一个
`WorkerGuard` 保证退出时 flush。

推论：

- **不要把托盘操作移出 main 线程**，它要求同线程创建/销毁。保持唤醒的
  `SetThreadExecutionState` 已改由服务进程的常驻线程独占，桌面端一律不得调用。
- **不要在 main 线程上等同步 IO。** 管道调用会阻塞最多 3 秒；配对弹窗的
  「取消配对」因此走 `pairing/cancel.rs` 的专用线程，结论经事件通道回到主循环。
- **不要在主循环里做阻塞 IO。** 管道调用会阻塞最多 3 秒。
- **Web 层不能直接调用只属于 main 线程的东西**，必须走 `HostCommand` 通道。

---

## 5. 桌面端模块职责

```text
apps/desktop/src/
├── main.rs              唯一入口：进程起始时刻 + 启动顺序接线（cli → bootstrap → main_loop → exit）
├── startup.rs           命令行参数（--startup / --restart-parent / --self-check / --check-service）
├── cli.rs               命令行模式分发：提权辅助进程 / 服务探针 / 正常启动的分流
├── bootstrap.rs         启动装配：数据目录 → 日志 → 数据库 → Web → 托盘 → 服务租约
├── main_loop.rs         主循环：HostCommand、托盘事件、配对弹窗、tooltip 同步、Windows 消息泵
├── exit.rs              退出序列：通知浏览器 → 停 IPC worker → 服务租约 → 数据库 → 日志 flush
├── startup_trace.rs     QINGLINK_STARTUP_TRACE 面包屑（`trace_step`）
├── admin_ui.rs          管理界面端口 / URL 与"交给默认浏览器"
├── self_check.rs        --self-check 诊断：托盘图标像素统计 + 注册结果 + PNG
├── preferences.rs       桌面端自己的设置（当前只有 debug_log）
├── service_lifecycle.rs 与服务的租约：注册父进程、心跳、退出时停服务
├── input_install/       安全输入服务的安装/修复/启停（含提权辅助进程）
├── service/             命名管道客户端
│   ├── mod.rs           ServiceError（错误分类是 HTTP 错误码的唯一来源）
│   └── client/          请求-响应、快照轮询与 IPC 线程
│       ├── mod.rs       模块边界：子模块声明与对外重导出
│       ├── handle.rs    交给 Web 层的调用句柄：发命令、等回执
│       ├── inner.rs     worker 线程与调用方共享的运行时内核
│       ├── runtime.rs   worker 线程生命周期：启动、停机、退出时释放队列
│       ├── worker.rs    worker 主循环：排空调用队列、按退避重连、驱动快照刷新
│       ├── transport.rs 阻塞式传输抽象与真实命名管道实现
│       ├── envelope.rs  请求/响应信封：编解码、协议版本校验与能力协商
│       ├── snapshot.rs  周期快照刷新：状态迁移与变更事件派发
│       └── state.rs     ServiceLinkState / 快照视图 / 事件类型
├── logging/             日志子系统
│   ├── entry.rs         LogEntry / LogLevel
│   ├── buffer.rs        内存环形缓冲 + 查询谓词 LogFilter + summary()
│   ├── collector.rs     tracing Layer：写入 buffer/broadcast
│   └── mod.rs           LogSystem 门面 + 级别热切换 + 文件轮转
├── tray/                托盘子系统
│   ├── mod.rs           模块边界：子模块声明与公开 API 重导出
│   ├── controller.rs    TrayController：窗口/菜单/图标的持有与生命周期
│   ├── window.rs        隐藏消息窗口、窗口过程、事件分发与右键菜单弹出
│   ├── shell.rs         Shell_NotifyIcon 调用与失败判读、重试预算
│   ├── report.rs        注册报告/状态模型、blocked_reason 分类、tooltip 文本
│   ├── menu.rs          菜单定义与 TrayAction 映射
│   └── icon.rs          SVG 光栅化（**必须保留颜色**，见第 6 节）
├── pairing/             配对提示弹窗（见 9.4）
│   ├── mod.rs           PairingPrompt 状态机 + PromptEvent
│   ├── window.rs        可见窗口与窗口过程（点击 → 事件）
│   ├── layout.rs        窗口几何、DPI 换算、子控件创建
│   ├── fonts.rs         由系统消息字体派生的两套字体
│   ├── cancel.rs        「取消配对」的专用线程
│   └── text.rs          弹窗文案
└── web/                 本地 Web 服务
    ├── mod.rs           路由 + 静态资源 + 两个事件泵
    ├── state.rs         WebState / HostCommand / ServerEvent / SharedSettings
    ├── error.rs         ApiError → 统一 {ok,error} 信封
    ├── ws.rs            /events 事件套接字（订阅、背压、引导帧）
    ├── assets.rs        内嵌前端产物（由 build.rs 生成）
    └── api/             REST 处理函数，按资源分文件
        └── catalog/     应用目录：门面重导出 + listing/icons/entries/scan/launch/batch/support
```

### 5.1 服务客户端（`service/`）

设计要点，改它之前请先读这段：

- 每个请求带 `oneshot` 回执通道，调用者拿到**自己的**响应，而不是抢一个共享的
  "最新响应"槽。老的实现是后者，这是它无法收敛的根源之一。
- 队列有界（64）。服务不可用时**快速失败**，不排队等几分钟再执行。
- **变更类命令从不重放。** 超时不代表没执行，所以把失败告诉调用者，让下一次快照
  重新给出事实。
- 能力协商：`Capabilities` 的每个字段都带 `#[serde(default)]`，所以新桌面端连旧
  服务时会得到 `catalog_management: false` 而不是解码失败。桌面端据此对不可用
  命令返回 `UNSUPPORTED`。
- **主线程不能 await。** 配对弹窗的「取消配对」走 `ServiceClient::call_blocking`：
  前置检查、`oneshot` 回执与队列都和 `call` 完全一致，只是改在专用线程上轮询回执，
  不依赖 Tokio 运行时；界面更新仍旧只在主线程发生。

### 5.2 服务端管理命令

`crates/qinglink-protocol/src/service_ipc.rs` 的 `DesktopCommand` 是桌面端与服务的
全部管理词汇表：

| 命令 | 作用 | 能力位 |
| --- | --- | --- |
| `Capabilities` | 协商 | — |
| `GetSnapshot { desktop_unlocked }` | 暂停/网络/待配对 | — |
| `SetPaused { paused }` | 暂停对外服务 | `pause` |
| `CancelPairing { pairing_id }` | 取消配对 | `pairing` |
| `ListDevices { offset, limit }` | 设备分页（含总数） | `devices` |
| `RevokeDevice { device_id }` | 撤销并断开连接 | `devices` |
| `GetCatalog { offset, limit }` | 仅可见应用（移动端视图） | `catalog` |
| `ListApps { kind, offset, limit, query, include_hidden, favorite_only }` | 管理视图，含隐藏；`favorite_only` 只要常用 | `catalog_management` |
| `GetApp { id }` | 单条详情 | `catalog_management` |
| `RenameApp { id, name }` | 写 `display_name`；空名恢复原名 | `catalog_management` |
| `SetAppHidden { id, hidden }` | 可见性 | `catalog_management` |
| `SetAppFavorite { id, favorite }` | 加入 / 移出常用（"我的库"） | `catalog_management` |
| `SetAppPinned { id, pinned }` | 置顶 / 取消置顶到手机首页（置顶自动加入常用） | `catalog_management` |
| `DeleteApp { id }` | 软删除 | `catalog_management` |
| `RescanApps { kind }` | 触发后台扫描 | `catalog_management` |
| `GetAppIcon { id }` | base64 图标 | `catalog_management` |
| `RefreshAppIcon { id, icon_path }` | 重解析并落库图标；`icon_path` 为空走自动来源链 | `app_icon_manage` |
| `ListAppIcons { id }` | 该条目可选的图标文件候选 | `app_icon_manage` |
| `GetCatalogStats` | 分类计数 | `catalog_management` |
| `AddApp { path, name, kind, args }` | 新增自定义条目，id 由规范化路径派生 | `app_add` |
| `LaunchApp { id }` | 在本机启动条目（转发到交互会话） | `app_launch` |

`Capabilities` 的全部字段（都带 `#[serde(default)]`，缺字段即 `false`）：

| 能力位 | 覆盖的命令 |
| --- | --- |
| `management` | **所有**命令的前置条件，为 `false` 时一个命令都不发 |
| `pairing` | `CancelPairing` |
| `pause` | `SetPaused` |
| `devices` | `ListDevices`、`RevokeDevice` |
| `catalog` | `GetCatalog` |
| `catalog_management` | `ListApps`、`GetApp`、`RenameApp`、`SetAppHidden`、`SetAppFavorite`、`SetAppPinned`、`DeleteApp`、`RescanApps`、`GetAppIcon`、`GetCatalogStats` |
| `app_add` | `AddApp` |
| `app_launch` | `LaunchApp` |
| `app_icon_manage` | `RefreshAppIcon`、`ListAppIcons` |

`Capabilities` 与 `GetSnapshot` 本身不受能力位约束（`management` 除外）。

新增命令一律通过新的能力位（`catalog_management` / `app_add` / `app_launch` /
`app_icon_manage`）门控：旧服务缺字段即为
`false`，桌面端在发命令之前就会返回 `UNSUPPORTED`，界面据此引导"修复服务（升级）"，
而不是把未知命令当成故障。

### 5.2.1 链路状态分类

链路状态由 `apps/desktop/src/service/client/state.rs` 的 `ServiceLinkState` 分类，并写进
`GET /service` 的 `state`。两端是正常的，中间三种失败互不相同：

| 状态 | 判据 | 正确处置 |
| --- | --- | --- |
| `connected` | 收到过快照且握手成功 | — |
| `connecting` | 还没收到过快照，也还没记录过失败（进程刚启动，或刚调过 `POST /service/reconnect`） | 显示"正在连接"，不要引导重装 |
| `protocol_mismatch` | 应答的 `version` 缺失或不等于 `SERVICE_IPC_VERSION`；或应答 `UNKNOWN_COMMAND` | 重装（修复）服务 |
| `unauthorized` | 应答 `UNAUTHORIZED_DESKTOP`（可信桌面端哈希不匹配） | 重装（修复）服务以重新登记 |
| `unreachable` | 传输层失败（管道缺失/被拒/超时） | 安装、启动，或检查执行环境 |

分类之前必须让旧协议的应答**能被解码**：`ServiceResponse` 的 `version` / `request_id`
带 `#[serde(default)]`，否则缺少这两个字段的旧响应会在传输层抛错，被误报成
"cannot receive pipe response"，把真正的版本问题藏起来。

服务端实现在 `services/qinglink-service/src/windows_service_impl/management/`：分发入口是
`command::dispatch`，`catalog_edit` 承担目录写操作，`apps` / `icons` / `devices` /
`pairing` / `catalog` / `lookup` / `record` / `page` / `snapshot` 各管一摊。
**任何修改 `scanner_apps` 的路径都必须调用 `reload_visible_catalog`**，否则内存里
的可见目录缓存不会更新，移动端会继续看到旧数据。`catalog_edit::publish_catalog_change`
统一做这件事，并为两个 kind 都广播 `LauncherCatalogReady` 事件。

控制面还有四项"读 PC 状态 / 改本机状态"的能力，都**不新增 WS 请求类型**（焦点事件只新增推送，
读取走既有代理管道）：

- `get_foreground` 的响应带可选的 `focus_text`：用 UI Automation 读焦点输入框已有文本。
  只认 `Edit` 控件 + `ValuePattern`，密码框、只读、不提供 Value 模式的控件（组合框、
  只暴露文本树的自绘控件）与占位提示（空白归一后比较）一律不返回——`TextPattern` 读的是
  控件文本树，Chromium 系会把 `placeholder` 混在里面，用它回填等于把提示文字当成正文。
  Chromium 系另外按**结构**判断：焦点 `Edit` 带子元素就是 contenteditable 富文本编辑器，
  同样不回填（它的 `Value` 里可能混着占位提示、UIA 无法区分）；纯 `<input>` / `<textarea>`
  是叶子节点，浏览器地址栏、搜索框与普通表单照旧回填。服务进程在 Session 0 下经会话代理
  读取，普通桌面进程直接读本会话。
- 物理鼠标按键状态由 `service/input_state_monitor.rs` 的后台线程每 250ms 轮询
  `GetAsyncKeyState`，仅在变化时广播 `input_state` 事件，并每 10 秒补发一次让后连上来的
  客户端也能对齐。
- 焦点是否进入输入框走**事件**，不轮询：会话代理里挂 `EVENT_OBJECT_FOCUS`（只收
  `OBJID_CLIENT`），钩子回调只置位一个本进程事件，工作线程被唤醒后读一次
  `has_focus_input()`（只判"焦点元素有没有可写的 `ValuePattern`"，不取文本），
  仅在上升沿（没焦点 → 有焦点）置位跨会话事件 `Global\QingLinkFocusInput`。
  服务进程的 `service/focus_monitor.rs` 阻塞 `WaitForSingleObject` 等这个事件，
  醒来就广播 `focus_input`（无载荷）。空闲时两侧线程都阻塞，不占 CPU，也没有管道流量。
  回调里**不读 UIA**：代理是单线程串行处理管道请求的，一次卡顿会堵住后续所有鼠标 /
  键盘注入，所以真正的读取放在工作线程上。
- **显示枚举与切换统一走 `qinglink_windows::display` 门面**：Session 0 的服务进程没有
  交互桌面，`EnumDisplayDevicesW` 在那里枚举不到任何"连接到桌面"的显示器，
  `ChangeDisplaySettingsExW` 也不会作用到用户桌面。门面按 `running_in_session_zero()`
  决定本地执行还是把 `get_displays` / `set_display_mode` 转发给会话代理；游戏启动分辨率
  的快照、切换与恢复共用这条路径，所以手机上看到的显示器与服务端切换的是同一块屏。
  列表本身裁成常用模式子集（≤4K 的六档分辨率 × `60/90/120/144`Hz）以减少噪音，当前模式
  不在子集里时会原样附上；`set_display_mode` 仍按枚举到的全量模式校验，存量非常用模式不受影响。

上述读状态能力复用既有请求与事件通道，不新增 `requests_ws.rs` 的请求枚举——那个文件
在 500 行硬上限之上，只许变短。保持唤醒是例外：它新增 `get_keep_awake` /
`set_keep_awake` 两个请求类型，同时清掉 `Option` 字段上冗余的 `#[serde(default)]`
（serde 对 `Option` 字段本就允许缺失），使该文件净减 4 行，仍满足 §2.3 的"只减不增"。

单次响应帧上限 64 KiB（命名管道帧限制），因此分页函数会**按序列化后的字节数**
收缩当前页并返回精确的 `next_offset`，绝不静默丢行。

### 5.3 crate 清单与职责

7 个 crate 加两个可执行入口；"内部依赖"一列是各自 `Cargo.toml` 的真实取值，
依赖方向规则见 [`.opencode/rules/architecture.md`](../.opencode/rules/architecture.md) §1。

| crate / 入口 | 职责 | 内部依赖 |
| --- | --- | --- |
| `crates/qinglink-protocol` | 跨语言 wire 类型与 JSON schema；本地管理 IPC 协议 `service_ipc.rs` | 无（叶子） |
| `crates/qinglink-domain` | 纯领域类型与状态机（设备配对状态机、领域错误） | 无（叶子） |
| `crates/qinglink-storage` | SQLite 连接池与迁移、`AppPaths`、数据目录 ACL | 无（叶子） |
| `crates/qinglink-version` | 桌面端与服务端共用的产品版本号：语义版本 + 自增构建号，含 Windows 版本资源生成；`is_newer` 供检查更新比较版本 | 无（叶子） |
| `crates/qinglink-discovery` | 局域网发现（UDP 3723 / mDNS 广播） | `qinglink-protocol` |
| `crates/qinglink-windows` | Windows 平台封装：输入注入、显示/音频、进程与电源、安全桌面、单实例、完整性 | `qinglink-protocol` |
| `crates/qinglink-server` | 局域网 `/api/v2/*` 的 HTTP/WS、认证、扫描与目录服务、`scanner_apps` 读写 | `qinglink-protocol`、`qinglink-domain`、`qinglink-storage`、`qinglink-version`、`qinglink-windows` |
| `services/qinglink-service`（`QingLinkService.exe`） | LocalSystem 服务：业务数据所有者、TCP 3721 / UDP 3723、命名管道管理面、会话输入代理 | `qinglink-protocol`、`qinglink-discovery`、`qinglink-server`、`qinglink-storage`、`qinglink-version`、`qinglink-windows` |
| `apps/desktop`（`QingLink.exe`） | 托盘 + 本地管理 Web + 日志 + 服务租约与命令代理 | `qinglink-protocol`、`qinglink-storage`、`qinglink-version`、`qinglink-windows`（**不依赖** `qinglink-server`） |

### 5.4 三端职责与构建编排

仓库里有三门语言、五个可独立构建的单元。**产物被谁内嵌，就住在谁下面**——这与 Tauri
把 Rust 放进 `src-tauri/` 是同一种安排：

| 单元 | 目录 | 构建器 | 产物 | 约束 |
| --- | --- | --- | --- | --- |
| Rust 库 | `crates/` | Cargo | 无独立产物 | 依赖方向见 §5.3 与 [`.opencode/rules/architecture.md`](../.opencode/rules/architecture.md) §1 |
| Rust 服务 | `services/qinglink-service` | Cargo | `target\*\QingLinkService.exe` | sidecar：签名后进 `apps/desktop/assets/input/`，由桌面端内嵌安装；该副本是构建产物，**不入库** |
| Rust 桌面端 | `apps/desktop` | Cargo + `build.rs` | `target\*\QingLink.exe` | 内嵌 web 产物、服务 exe 与自更新执行器；**不依赖** `qinglink-server` |
| Vue 前端 | `apps/desktop/web-ui` | npm / Vite | `apps/desktop/web-ui/dist` | **没有独立交付物**，被 `build.rs` 内嵌；`dist/` 不入库 |
| Flutter | `apps/mobile` | Flutter | `apps/mobile/build/**` | android / ios 是交付目标；`windows/` 仅本地调试 |

**构建编排的位置。** 跨语言编排全部在 [`apps/desktop/build.rs`](../apps/desktop/build.rs)：
它相当于 Tauri 的 `tauri.conf.json` 加 CLI——`web-ui/dist` 缺失或过旧时自动构建前端并内嵌，
服务二进制按约定从 `apps/desktop/assets/input/` 或 `target/{debug,release}/` 取用（前者是构建
产物，不入库，见上表）；
两者都可能打 `cargo:warning` 提醒内嵌内容已经过期。**因此本项目不设脚手架脚本**，
构建、运行、校验、发布一律用语言原生命令，规则见 [`AGENTS.md`](../AGENTS.md) §7。

---

## 6. 托盘子系统（含历史缺陷）

### 6.1 为什么会"托盘图标不显示"

这是本项目真实踩过的坑，五个原因叠加（第 4 条是历史原因，那种实现已经不在代码里）：

1. **图标被转成了纯黑。**
   早期代码把光栅化结果的每个像素 RGB 强制设为 0，只保留 alpha。品牌图标是
   "蓝色圆角方块 + 白色链接图案"，两个元素因此都变成黑色，最终得到一个
   **纯黑方块**：在深色任务栏上完全隐形，在浅色任务栏上只是一个没有细节的黑块。
   `tray/icon.rs` 现在**刻意保留颜色**，模块注释里写明了不要重新引入模板化转换。
2. **托盘注册必须以 shell 的返回值为准。** 当前实现每次调用
   `Shell_NotifyIconW(NIM_ADD)` 都记录其返回值与 `GetLastError`；启动时的短暂失败会由窗口定时器
   重试，收到 `TaskbarCreated` 广播时会重新注册，但不会调用几何探测来臆测图标是否存在。
3. **注册成功与用户当前是否看见图标是两件事。** `Shell_NotifyIcon(NIM_ADD)` 被接受之后，
   再用 `Shell_NotifyIconGetRect` 问一次"图标在哪"：返回坐标说明它确实在通知区域，
   返回错误说明它可能仍在折叠区。两类信息都进报告（`visible` / `verified` / `rect`），
   而不是靠几何探测猜。
4. **历史原因（`tray-icon` 已移除）：依赖那个库时失败是静默的。** 它创建的是
   **隐藏的顶层窗口**（`WS_OVERLAPPED` + `CW_USEDEFAULT`，尺寸 0），并且在 `NIM_ADD`
   失败时**刻意不报错**（它假设"任务栏可能还没准备好，等 `TaskbarCreated` 再说"）。
   结果是：注册被 shell 拒绝、图标只进了折叠区、图标根本没创建，三种情况在它那里
   长得一模一样。现在托盘直接实现在 Win32 外壳 API 上（见下），每一次
   `Shell_NotifyIcon` 的返回值和 `GetLastError` 都会被记录并进入自检报告。换成任何
   托盘 crate 都不会改变这一点：所有实现最终调用的都是同一个 `Shell_NotifyIcon(NIM_ADD)`。
5. **完整性级别不匹配是唯一无法在应用内绕过的失败。** 通知区域只接受与它同级的进程注册
   图标：低于它（受限令牌、低完整性沙箱）或高于它（管理员）都会得到
   `GetLastError=5`。实测在 low 会话里直接调 `Shell_NotifyIconW`、三种标准写法（含/不含
   `NIF_SHOWTIP`、设置/不设置 `NOTIFYICON_VERSION_4`）**全部**返回 FALSE + 5，所以这是
   shell/OS 层面的判决，换实现、加轮询都无济于事。应用能做的是**立刻说清原因**：
   `blocked_reason()` 把"永久拒绝"与"Explorer 还没就绪"分开，前者不再空耗重试预算。

### 6.1.1 直接实现（不走托盘 crate）

`tray/` 只需要五样东西，全部由自己持有，因此每一步都能报告结果：

| 组成 | 做法 | 为什么不用库 |
| --- | --- | --- |
| 接收消息的窗口 | `CreateWindowExW(WS_EX_NOACTIVATE \| WS_EX_TOOLWINDOW, …)` 隐藏顶层窗口 | 不出现在任务栏/Alt-Tab，不抢焦点，同时能接收 Explorer 广播的 `TaskbarCreated` 消息 |
| 图标注册 | `Shell_NotifyIconW(NIM_ADD)`，检查返回值 + `GetLastError` | 失败必须可见，而不是被吞掉 |
| 注册结果核验 | `Shell_NotifyIconGetRect(NOTIFYICONIDENTIFIER)` | 文档化的"图标到底在不在通知区域"查询；它把"注册被拒 / 已被接受但折叠"分开，而不是靠几何探测臆测 |
| 坐标与右键菜单 | `NIM_SETVERSION(NOTIFYICON_VERSION_4)` + `TrackPopupMenu` | v4 才会在回调里带上图标的屏幕坐标，菜单才能贴着图标弹出；设置失败会带 `GetLastError` 记录为警告 |
| 失败分类 | `blocked_reason()` 纯函数：`GetLastError=5` + 完整性级别不一致 = 重试无法修复 | 可恢复失败（Explorer 刚启动）继续按定时器重试；永久失败立刻报错，不空耗 20×250ms 预算 |

图标句柄也不是直接用位图：`icon::bitmap_from_rgba` 用 `CreateDIBSection`（32bpp、
`biHeight` 为负的 top-down）+ 全零掩码位图，再经 `CreateIconIndirect` 合成真正的
`HICON`。把位图当图标交给 shell 会得到一个不可见或纯黑的方块。

### 6.2 自检

```powershell
QingLink.exe --self-check        # 像素统计 + 注册结果 + 服务安装状态
QingLink.exe --check-service     # 单独探一次服务命名管道
```

`--self-check` 输出 JSON 到 stdout（debug 构建是 console 子系统，直接继承终端），
日志走 stderr，所以 `> 文件` 拿到的仍然是一份干净 JSON。字段含义：

| 字段 | 判读 |
| --- | --- |
| `tray_icon.size_px` / `opaque_pixels` / `transparent_pixels` / `distinct_colours` | 像素统计基数；`distinct_colours` 按每通道高 4 位量化后计数 |
| `tray_icon.monochrome` | `true` 表示图标已经退化成单色 → 检查是否有人重新引入了黑色转换 |
| `tray_icon.has_brand_blue` / `has_white_glyph` | 品牌图形是否完整 |
| `tray_icon.icon_handle_ok` | 像素是否成功合成为 `HICON`；`false` 时 `icon_handle_error` 给出原因 |
| `tray_icon.png` | 同时把渲染结果写成 PNG，可以直接用眼睛看 |
| `tray_registration.visible` | `Shell_NotifyIcon(NIM_ADD)` 是否被 shell 接受；`false` = 图标没有注册 |
| `tray_registration.verified` | `Shell_NotifyIconGetRect` 是否确认图标已在通知区域（折叠区也算已注册） |
| `tray_registration.rect` | `GetRect` 返回的真实坐标；`null` = 在折叠区或未注册 |
| `tray_registration.icon_size` | 光栅化图标的像素边长 |
| `tray_registration.error_code` | 上一条失败时的 `GetLastError`（`5` = 拒绝访问） |
| `tray_registration.integrity` / `.shell_integrity` | 本进程与通知区域所有者各自的完整性；不一致时图标一定注册不上 |
| `tray_registration.attempts` | 启动时首次注册就被永久拒绝为 `0`（还没走过重试）；重试路径上为该轮已尝试的重试次数 |
| `tray_registration.blocked_reason` | 非空 = 重试无法修复，字段内容就是处理办法 |
| `tray_registration.detail` | 人类可读的注册结论；从未尝试注册时为"尚未注册" |
| `log_dir_writable` | 日志目录是否可写 |
| `web_interface.placeholder` | `true` 表示前端没构建，正在提供占位页 |
| `service.*` | 安装/运行/哈希比对与 `network_ready`（回环健康协议） |

**完整性级别是"进程实际跑在哪一级"的唯一权威答案。** 启动日志与这份报告都会打印它；
终端是 medium 不代表它启动的子进程是 medium（子进程继承父令牌，low 进程无法创建 medium
子进程），所以排查时以这两个字段为准，而不是以终端或任务管理器为准。

同一份托盘报告也出现在 `GET /api/admin/v1/overview` 的 `tray` 字段，仪表盘的
"安全输入服务体检"卡片会直接展示它——包括"注册被拒绝"的原因，所以不必装命令行也能
看到图标为什么不见了。

`--check-service` 回答另一个问题：管理管道是否可连。它打印 `probe.ok`、
`probe.code`、完整错误链，以及服务安装状态，因此"服务在运行但管道被环境拒绝"
可以一眼确诊，而不是去重装一个本来正常的服务。

### 6.3 菜单

严格四项，其余管理功能都在 Web 界面里：

| 菜单项 | 行为 |
| --- | --- |
| 打开 QingLink | 用默认浏览器打开 `http://127.0.0.1:3722` |
| 重启服务 | 提权重启 `QingLinkService`，结果通过 install-state 事件回流 |
| 保持唤醒 | 勾选项，经桌面管理 IPC 的 `set_keep_awake` 切换服务进程的开关；勾选状态按管理快照同步 |
| 退出 | 退出桌面端（并请求服务停止，见 6.4） |

托盘/管理界面设置页与移动端控制中心的「保持唤醒」是**同一个开关**：由
`QingLinkService` 进程用一条常驻线程持有 `SetThreadExecutionState`（系统 + 显示器），
状态持久化在服务库 `settings.power.keep_awake` 并在服务启动时恢复。桌面端不再
自己调用该系统 API：设置页与托盘经 `set_keep_awake` 命令写入，按管理快照的
`keep_awake` 字段（推 `ServiceEvent::KeepAwake`）跟随状态变化，因此手机端切换后
PC 端也会同步。旧服务没有该能力位时，桌面端开关判为不支持。

左键单击等同于"打开 QingLink"：左键 `WM_LBUTTONUP` 直接打开界面，右键才
`TrackPopupMenu`（菜单从不挂在左键上）。

菜单句柄由 `TrayMenu` 持有并在 `Drop` 时 `DestroyMenu`；命令 id 从 1 开始，因为
`TrackPopupMenu` 用 0 表示"用户取消了菜单"——用 0 做 id 会让每次取消都像点了第一项。

### 6.4 退出语义

- **退出**：通知浏览器 → 等 400ms 让 HTTP 响应发出 → 请求服务停止。
- **重启**：通知浏览器 → 等 400ms → 拉起新实例（`--restart-parent <pid>`）→ 退出。

先回包再退出是刻意的：退出是由 API 调用触发的，那个响应必须先离开 socket。
等待窗口见 `exit.rs` 的 `RESPONSE_FLUSH`。

重启有个句柄细节：tokio/mio 在 Windows 上用 Winsock 的 `socket()` 建监听套接字，这个句柄是
**可继承**的，而 `CreateProcess`（`--restart-parent` 的替换实例，以及浏览器、提权助手）默认会
复制可继承句柄。替换实例一旦继承旧监听套接字，端口就由它自己继续占着，它自己的 `bind` 会以
`WSAEADDRINUSE` 失败——`netstat` 里那条 `LISTEN` 还挂在**已死**的父进程 PID 上，直到子进程退出
才消失，现象就是「重启桌面端之后应用再也没起来」。`web::bind` 因此在绑定成功后显式清掉
`HANDLE_FLAG_INHERIT`（`make_listener_non_inheritable`），并保留一个约 3 秒的重试窗口兜底。

---

## 7. 日志子系统

### 7.1 两个内存 sink + 一个文件 sink

```text
tracing::info!(..)
      │
      ├─► LogCollector ──┬─► LogBuffer（5000 条，回答"刚刚发生了什么"，无 IO）
      │                   └─► broadcast（WebSocket 实时订阅）
      ├─► fmt layer ────────► 按天轮转的文本文件（历史，长期保留）
      └─► fmt layer ────────► stderr（开发终端；GUI 子系统里写失败也不影响任何东西）
```

console 子系统与 GUI 子系统共用同一套图层：debug 构建是 console 子系统，因此 stderr 直接
落到运行 `cargo run` 的那个终端；release 构建是 GUI 子系统、没有控制台，这一层的写入
不会到达任何地方（无代价）。**等级默认 `info`**；环境变量按 `QINGLINK_LOG_LEVEL`
优先、其次 `RUST_LOG` 的顺序取值，且只在非空白时接管（`logging/mod.rs:618-629`）。
只有轮转文件真正打开成功时才安装文件层：目录不可用时不再"回退到 stderr"，
而是根本不装文件层，避免与控制台层把每条记录写两遍。

**日志不写数据库。** 早期实现把结构化历史放进独立的 `logs.db`，由后台线程批量写入。
那套设计有两个问题：启动时打开 SQLite 池是**无期限的阻塞操作**，数据目录慢、被占用或
被拒绝时应用会卡在启动阶段（表现为"进程活着但端口不开、什么都没有"）；而且用进程内
id 当主键 + `INSERT OR REPLACE` 时，每次重启都会覆盖上一会话的同号记录，历史实际只有
最后一次运行。改成文本文件后，这两类问题都不存在：文件是操作系统保证的追加写，人能直接
读，进程崩了也在。

两个内存 sink 仍可独立失效：

- 订阅者太慢 → `broadcast` 容量 1024，溢出以 `RecvError::Lagged(n)` 精确报告
  丢了 n 条，而不是拖慢发日志的线程。
- buffer 满 → 丢最旧的，界面翻页的 `total` 因此是"内存里还有多少条"。

### 7.2 背压链路

日志是唯一可能无限增长的数据流，有两道闸：

1. **内存侧**：ring buffer 固定 5000 条，最旧的被覆盖；`/logs` 的 `total` 就是它。
2. **推送侧**：`pump_logs` 每 200ms 合并一帧，单帧最多 200 条，超出部分计数后
   通过 `logs/dropped` 事件告知界面。

磁盘侧不设闸：文本文件按天轮转，保留由使用者自己清理（这是刻意的——日志的价值在于
"事情发生之后还能查到"）。

### 7.3 为什么磁盘日志不会拖慢启动

`tracing_appender` 的 `RollingFileAppender` 在目录不可写或文件被占用时**返回错误**，
`file_layer()` 随即只返回一个"没有文件层"的图层（控制台层照常工作）。整个初始化过程
没有数据库连接池、没有重试等待，所以日志系统在数十毫秒内必定完成（见第 9.1 节的启动面包屑）。

### 7.4 查询

`/logs` 只查内存缓冲，按**写入顺序倒序**——实现是 `entries.iter().rev()`，顺序由
进程内单调递增的 `id` 决定，而不是按 `timestamp_ms` 排序（同毫秒内的多条记录因此
仍按写入次序稳定排列）。`level` 过滤用**显式成员判断**而不是字符串比较——按字典序
`"ERROR" < "INFO"`，直接比较会把严重度顺序弄反。更早的记录用 `/logs/download` 取文本文件。

### 7.5 级别热切换

`EnvFilter` 通过 `reload` 句柄在运行时切换，两个位置都由**同一段展开逻辑**生成：
`debug` ↔ `info` 都经 `env_directive()` 变成 `QingLink=<级别>,qinglink=<级别>`，
所以启动默认与界面切换不可能各说各话。裸级别名（如 `QINGLINK_LOG_LEVEL=debug`）必须展开
成大小写两个前缀：`EnvFilter` 的目标匹配区分大小写，而本程序的目标是 `QingLink`、
依赖是 `qinglink_*`，只写一个会把自己所有日志丢掉。

---

## 8. Web 子系统

### 8.1 请求路径

```text
/api/admin/v1/*      REST（见 web-api.md）
/api/admin/v1/events WebSocket 事件
/                    内嵌 SPA 外壳
/assets/*            带内容哈希的内嵌资源（长缓存）
其他非 /api/ 路径     同样返回 SPA 外壳
其他 /api/ 路径      JSON 404（绝不返回 HTML）
```

最后一条很重要：把 HTML 返回给一个拼错的 API 调用，会让客户端的
`response.json()` 抛异常，从而掩盖真正的问题。

### 8.2 只监听回环

服务绑定 `127.0.0.1`。它**有意不做认证**，因为它代理的那条命名管道本身已经由
服务端做了桌面身份授权。绑到可路由地址等于把这份授权交给整个网络。

### 8.3 路由构建时机

`run_on_thread` 在**调用线程**先构建 Router，再启动 worker 线程。原因是踩过的：
路由字符串写错会让 worker 线程 panic，而那时 `ready` 信号可能已经发出，于是
进程看起来健康、端口却没人监听。现在路由配置错误是启动期的硬失败。

（相关坑：axum 0.7 的 catch-all 写法是 `/*name`，`{*name}` 是 0.8 的语法，
在这里会 panic。）

### 8.4 前端如何进入二进制

`build.rs` 扫描 `apps/desktop/web-ui/dist`，生成
`$OUT_DIR/web_assets.rs`（每个文件的 `include_bytes!` + 路径 → (MIME, bytes)
查表）。于是：

- 拷贝单个 exe 就能提供完整界面，不需要附带目录；
- 前端缺失但 Node 可用 → 构建期自动 `npm install && npm run build`；
- 两者都不满足 → 内嵌一个说明性的占位页并打 `cargo:warning`，
  **不让后端构建失败**。

后端**不依赖前端的资源命名约定**：`/assets/**` 走长缓存，其他被内嵌的路径由
fallback 返回并带 1 小时缓存，`index.html` 与未知路径返回外壳。

### 8.5 主题（浅色默认 / 深色 / 跟随系统）

**默认是浅色**：未持久化偏好时不跟随系统，否则系统处于深色模式的机器永远看不到浅色
主题。颜色令牌成对定义在 `styles/light.css` 与 `styles/dark.css`，选择器是
`[data-theme='light']` / `[data-theme='dark']`；几何与排版在 `styles/tokens.css`，
与主题无关的重置与基础排版在 `styles/base.css`。
`--shadow-card` 这类分层令牌在深色下取 `none`：浅色底上白卡片必须靠"边框 + 极浅阴影"
与页面底分开，深色底本身就有足够的亮度差。

浅色令牌有对比度基线（正文与次要文本 ≥4.5:1；语义色落在 soft 衬底上同样 ≥4.5:1；
控件边界只需视觉可辨，键盘可辨识性由焦点光环保证），改色值时要重新核算：
`--text-3` 用在 12px 小字上、`--warning` 会落在自己的 soft 衬底上，这两个最容易不达标。

视觉语言（圆角、控件高度、字号、字重、动效曲线、阴影）集中在 `tokens.css`，
颜色只在两份主题文件里；**组件里既没有颜色字面量，也不写死这些尺寸**，
所以"换一套气质"是改令牌而不是改组件。

主题状态只有一处：`<html data-theme>`。写它的人有两个——`index.html` 里的一段内联脚本
（在样式表解析前执行，避免首屏闪另一套主题）与 `stores/theme.ts`（挂载后接管，并写入
`localStorage['qinglink.theme']`）。两者读同一个键名、也用同一个默认值，改名或改默认值
要同时改。

为什么偏好不进后端设置：管理界面是浏览器页面，主题是它的显示属性，服务端没有理由知道；
而且走接口意味着首帧要等一个请求，闪屏无法避免。

### 8.6 事件泵

- `pump_service_events`：把服务的 `ServiceEvent` 转成 `ServerEvent`。客户端落后
  时直接推送**当前状态**而不是错过的增量。服务连接状态发生跳变时，还会顺带重探一次
  安全输入服务体检结果（SCM 视角只有这里会提醒，且不应该进入 2Hz 的轮询路径）。
- `pump_logs`：见 7.2。

服务的 500ms `GetSnapshot` 轮询不只用于"连接是否还在"：它对比整份快照，把
**暂停 / 网络就绪 / 地址 / 错误**的变化、**有效设备数**的变化、**每类扫描完成时间戳**
的变化分别转成 `service/status`、`devices/changed`、`apps/catalog_changed(scan_finished)`。
管道是严格请求/响应，没有服务→桌面端的推送通道，所以这是让后台扫描完成、设备增删、
服务状态变化到达浏览器的唯一低成本通道——新增"变化才有"的状态时请沿用这个模式，
不要让浏览器去轮询。

`ServerEvent` 与 `ServiceEvent` 都**手工实现 `Debug`**，因为它们含有
`PendingPairing`。那个类型刻意不实现 `Debug`，为的是 PIN 不可能被日志打出来。
新增携带敏感字段的事件类型时请沿用这个做法。

---

## 9. 关键时序

### 9.1 启动

```text
提权辅助进程（--input-service-control）最先处理并退出
→ 参数解析 → --check-service 则探管道并退出（这两步都在任何子系统之前）
（debug 构建是 console 子系统，日志与这些命令的输出直接落在启动它的终端上）
→ 数据目录 resolve
→ 日志系统 → --self-check 则诊断并退出
  （文件层打不开则只保留控制台层，不退出，也不会阻塞）
→ 等待重启父进程（仅 --restart-parent）
→ 单实例互斥体（已存在则激活对方并退出）
→ 会话检查（判定当前会话是否受支持）
→ 数据库迁移 → 注册 qinglink/data
→ Preferences 读取 → 应用调试日志级别（保持唤醒不再走偏好）
→ ServiceRuntime（IPC 线程）
→ 设置镜像（autostart 读注册表；保持唤醒初值 false，等管理快照同步）
→ WebState 装配 + 路由构建 + Web 线程（端口占用则硬失败）
→ 托盘创建（NIM_ADD 被拒后：可恢复则定时重试，完整性级别不匹配则立刻报错并打开浏览器兜底）
→ 配对提示窗口创建（失败只降级为"配对码仅在管理界面显示"，不影响启动）
→ 服务租约线程（准备服务并开始心跳，不提权；管道被环境拒绝时明确说明原因）
→ 主循环
```

这段顺序由 `cli.rs`（前两行）、`bootstrap.rs`（数据目录 → 服务租约）、`main_loop.rs`（主循环）
与 `exit.rs`（退出）实现，`main.rs` 只做接线；退出顺序（IPC → 服务租约 → 数据库 → 日志 flush）
同理见 `exit.rs`。改顺序必须同时改这里和代码。

注意顺序：**日志在数据库之前**（这样后面任何一步失败都还有 stderr 可看）；
**单实例在数据库 / 服务 / Web 之前**（第二次启动只通知持有者并退出，不做任何初始化）；
**会话检查也排在数据库之前**，不受支持的会话不必先打开数据库。

设 `QINGLINK_STARTUP_TRACE=1` 时每个阶段都会往 stderr 打一条带耗时的面包屑，
"进程活着但什么都没发生"因此可以定位到具体一行而不需要调试器。单条格式是
`[qinglink-startup] +{:>7.3}s pid={} {step}`（`startup_trace.rs` 的 `trace_step`），正常启动路径共 17 条；
耗时随机器变化，下面是形状示例：

```text
[qinglink-startup] +  0.000s pid=12345 main entered
[qinglink-startup] +  0.009s pid=12345 startup options parsed
[qinglink-startup] +  0.023s pid=12345 data directory resolved
[qinglink-startup] +  0.034s pid=12345 logs initialised
[qinglink-startup] +  0.041s pid=12345 single instance acquired
[qinglink-startup] +  0.052s pid=12345 session checked
[qinglink-startup] +  0.061s pid=12345 application database migrated
[qinglink-startup] +  0.068s pid=12345 application database registered
[qinglink-startup] +  0.079s pid=12345 preferences loaded
[qinglink-startup] +  0.081s pid=12345 service transport started
[qinglink-startup] +  0.082s pid=12345 install state probing started
[qinglink-startup] +  0.084s pid=12345 web state assembled
[qinglink-startup] +  0.126s pid=12345 web listener reporting ready
[qinglink-startup] +  0.128s pid=12345 tray registration attempted
[qinglink-startup] +  0.128s pid=12345 pairing prompt created
[qinglink-startup] +  0.129s pid=12345 service lease thread started
[qinglink-startup] +  0.129s pid=12345 entering the main loop
```

`--restart-parent`、`--check-service`、`--self-check` 与提权辅助进程会各插入一条
自己的面包屑（`restart parent exited` / `running the service probe` /
`running the self check` / `helper command dispatched`），所以实测条数可能多于 17。

### 9.2 重命名一个应用

```text
浏览器 POST /apps/rename {id, name}
→ api/catalog/entries.rs 校验（≤128 字符、无控制字符；**空名合法**，表示恢复原名）
→ ServiceClient.call(RenameApp) → IPC 线程 → 管道 → 服务
→ 服务写 display_name → reload_visible_catalog → 广播 LauncherCatalogReady
→ 服务返回 {id, name, renamed}
→ 桌面端 publish(CatalogChanged("rename"))
→ 浏览器收到 apps/catalog_changed → 重新拉取当前页
→ 同一台 PC 上的手机下一次拉取目录就看到新名字
```

注意最后两行：**桌面端不做乐观更新**，界面等服务端事实回流。

### 9.3 触发扫描

```text
POST /apps/scan {kind}
→ 服务检查是否正在游戏（是则 CONFLICT）
→ 服务起后台线程扫描（scan_gate 串行化，避免两个扫描抢磁盘）
→ 立即返回 accepted
→ 扫描完成后 reload_visible_catalog + 广播
→ 浏览器收到 catalog_changed(reason=scan_finished) → 刷新列表与统计
```

### 9.4 配对弹窗

```text
手机 POST /api/v2/pairing/requests
→ 服务生成 6 位配对码（crates/qinglink-server/src/server/pairing.rs）
→ 桌面端每 500ms 的快照轮询读到 pending_pairing（含码与 expires_at_unix）
→ 主循环 sync_pairing_prompt：出现新请求 → pairing/window.rs 弹出置顶窗口

手机输入配对码并 confirm，或请求到期（60 秒）
→ 服务快照里 pending_pairing 变 None
→ 主循环的下一次 sync 隐藏窗口（最迟一次轮询 + 一个 50ms 节拍）

窗口上的「取消配对」
→ WM_COMMAND → PromptEvent::CancelRequested
→ 主循环起 qinglink-pairing-cancel 线程 → ServiceClient::call_blocking(CancelPairing)
→ 结论经 PromptEvent::CancelFinished 回到主循环 → Toast + 隐藏窗口
```

要点：

- **窗口不自己判断结局。** 配对成功、超时、被取消、桌面锁屏、服务断开，最终都表现为
  `pending_pairing == None`，所以只有一条隐藏路径；新增结束条件不必再改窗口。
- **取消后不能被迟到快照弹回来。** 取消是异步的，而快照最多滞后 500ms；控制器记下被
  压住的 `request_id`，直到它真正从快照里消失。
- **锁屏不下发 PIN。** 服务在 `desktop_unlocked == false` 时把 `pending_pairing` 清空，
  因此锁屏时既不会弹窗也不会有可用的配对码（与 §10 一致）。
- **PIN 不进日志。** `PendingPairing` 没有 `Debug`，`PairingPrompt` 的手写 `Debug`
  只打印 `request_id`（与 §8.6 同一约定）。
- 窗口创建失败不是启动失败：弹窗为 `None` 时配对码仍可在管理界面里看到。

### 9.5 程序自更新

更新源是 GitHub Releases 的双通道 tag：`desktop-v<语义版本>+<构建号>` 与
`mobile-v<语义版本>+<构建号>`。桌面端只认前者，移动端只认后者，互不干扰。检查时拉取
releases 列表，按通道前缀过滤，用 `qinglink_version::is_newer`（桌面）或
`ParsedVersion`（移动端）比较；`+` 后的构建号参与比较——它是源码改动的递增序号。

两端启动时各自动检查一次；手动检查入口随时可重复执行。发现新版本立即自动下载，
安装始终手动。桌面端流程（Web API 见 `docs/web-api.md` §4.37 – §4.40）：

1. 自动检查或 `POST /update/check`：后台请求 GitHub API，发现新版本后进入 `available`，
   通过 `system/update_state` 事件推送完整状态快照。三个完成分支（发现更新 / 已是最新 /
   失败）都会推送，避免界面卡在 `checking`。
2. 自动触发或 `POST /update/download`：先下载统一清单 `SHA256SUMS` 并解析，再下载
   `QingLink.exe`，按清单里的期望哈希核对后落到 `%LOCALAPPDATA%\QingLink\updates\<版本>\`，
   进入 `ready`。清单缺失、格式无效、缺条目或哈希不符都进入 `error`，绝不使用未校验文件。
   REST 接口主要供校验失败后手动重试。
3. `POST /update/install`：以下载目录里的**新 exe 副本**加 `--self-update` 启动独立进程，
   然后请求主循环退出。新副本等旧进程结束 → 用自身覆盖目标 → **提权重跑"修复服务安装"**
   → 拉起新进程。替换必须由另一个进程完成（Windows 无法覆盖运行中的 exe），执行器逻辑
   已编译进桌面端本体，不再单独发布更新器 exe。提权重登记不可省：服务用
   `trusted-desktop.json` 登记桌面端 SHA256，不重新登记新 exe 会被判定为未授权。UAC
   弹窗由服务重新安装触发，需用户确认；用户取消时仍尝试拉起新进程，界面可再走
   "修复服务"补登记。结果经 `--result` 文件回传：先写结果再拉起，新进程启动时消费该文件，
   失败原因推送到更新状态。

下载用 PowerShell `Invoke-WebRequest` 而非引入 HTTP client crate：本项目是纯 Windows
程序，reqwest 会为 wasm 目标拖进用不到的依赖（web-sys 等），在受限网络下拉不下来；
PowerShell 随系统自带、自动遵循代理与 TLS，配合 `-NoProfile -NonInteractive` 与
`CREATE_NO_WINDOW` 做到无闪窗、无交互卡死。

移动端流程：设置页「检查更新」进入独立页面；Android 下载 APK 并校验后经 `open_filex`
拉起系统安装器（需 `REQUEST_INSTALL_PACKAGES`）；iOS 无法侧载，只提供跳转到 Release
页面的入口。

---

## 10. 安全边界

- 管理面（3722）只监听回环、无认证，安全性由"只有本机可访问"保证。
- 业务数据（`qinglink.db`、`config\`）的 ACL 只给 SYSTEM 与管理员，普通用户进程无法
  绕过服务改写；`logs\` 子树额外给普通用户只读，那只是诊断文本，不含业务状态。
- 管道请求由服务端做桌面身份校验（`authorization::desktop`，核对会话与令牌完整性）。
- 配对 PIN 在桌面已锁屏时不下发（快照会清空 `pending_pairing`）。
- 日志下载接口的文件名走白名单字符集，并再次校验解析后的路径仍在日志目录内。
- IPC 响应帧有 64 KiB 上限，分页函数按字节收缩，防止一条超大记录撑爆帧。

---

## 11. 有意不做的事

写下来是为了避免后来者"顺手补上"：

- **桌面端不直连业务数据库。** 没权限，也不该有。
- **管理接口不做认证。** 回环 + 管道授权已经覆盖威胁模型；加一层会给人错误的
  安全感，却挡不住本机上的其他进程。
- **托盘不放"开机启动""暂停服务"。** 它们在 Web 设置页里；托盘只保留四项高频操作。
- **日志目录不可写时不阻止启动。**
