# QingLink 桌面管理 API（Web UI 契约）

本文档是 **QingLink 桌面端本地管理接口** 的唯一权威契约。
它面向两类读者：实现该接口的工程师，以及需要据此编写前端 / 自动化脚本的 LLM 或开发者。

> 约定：本文档描述的所有接口**只监听 `127.0.0.1:3722`**，不对外网暴露，无需认证。
> 移动端使用的对外协议是另一套接口，见 [`protocol/v2.md`](../protocol/v2.md)。

---

## 1. 架构与数据归属（必读）

QingLink 是"一个数据源 + 多个控制端"的结构。

```text
                     ┌─────────────────────────────────────────┐
                     │  QingLinkService.exe  (LocalSystem 服务) │
                     │  ─────────────────────────────────────   │
                     │  · 拥有全部持久化数据 (SQLite)           │
                     │  · 应用目录 / 重命名 / 隐藏 / 删除       │
                     │  · 已配对设备与令牌                      │
                     │  · 局域网协议服务 TCP 3721 / UDP 3723    │
                     └───────────────┬─────────────────────────┘
                                     │ 本机命名管道 \\.\pipe\QingLinkControlV2
                                     │ (DesktopCommand / ServiceResponse)
                     ┌───────────────┴─────────────────────────┐
                     │  QingLink.exe  (桌面端, 普通用户权限)    │
                     │  ─────────────────────────────────────   │
                     │  · 系统托盘图标                          │
                     │  · 本地 Web 服务 127.0.0.1:3722         │
                     │  · 日志采集与查询                        │
                     │  · 不拥有业务数据，只做代理               │
                     └───────────────┬─────────────────────────┘
                                     │ HTTP / WebSocket (本文档)
                     ┌───────────────┴─────────────────────────┐
                     │  浏览器 Web 管理界面 (Vue 3 SPA)         │
                     └─────────────────────────────────────────┘
      移动端 App ────── WebSocket /api/v2/events ──────► QingLinkService
```

### 关键结论

1. **数据跟随 PC，不跟随控制端。** 应用列表、用户重命名、隐藏状态、已配对设备全部存放在服务端
   （`%ProgramData%\QingLink\qinglink.db` 的 `scanner_apps` 表等）。换一台手机连接同一台 PC，
   看到的数据完全一致；换一个浏览器打开 Web 界面，看到的也完全一致。
2. **桌面端不直接读写业务数据库。** 它对 `%ProgramData%\QingLink` 没有权限，所有业务操作都通过
   命名管道转发给服务端。因此 Web 界面的"应用管理"本质是**服务端能力的远程暴露**。
3. **服务未安装 / 未运行时**，管理类接口返回 `SERVICE_UNAVAILABLE`，Web 界面必须显式提示，
   而不是静默失败。

### 三套接口的边界

| 接口 | 端口 | 使用者 | 能否修改业务数据 |
| --- | --- | --- | --- |
| `/api/v2/*` + `WS /api/v2/events` | 3721 | 移动端 App | 是（目录新增 / 重命名 / 删除 / 隐藏 / 常用 / 置顶、图标、自定义扫描目录；另有启动应用、管理进程、键鼠输入与系统动作） |
| `/api/admin/v1/*` + `WS /api/admin/v1/events` | 3722 | 本机 Web 管理界面 | 是（重命名 / 隐藏 / 删除 / 常用 / 置顶 / 新增 / 图标 / 撤销设备） |
| UDP `3723` | 3723 | 移动端发现 | 否 |

两侧写的是**同一份**服务端目录数据（`scanner_apps`），差别只在界面与各自暴露的字段，
不在权限：手机端能改的东西和本机管理界面基本重合，完整清单见
[`protocol/requests.md`](../protocol/requests.md) 的 6、7、14 节。

---

## 2. 通用约定

### 2.1 响应信封

所有 REST 接口统一返回 JSON。**判定成功必须看 `ok` 字段**，但 HTTP 状态码也是
有意义的，便于浏览器 DevTools 与代理观察：

| 情况 | HTTP 状态码 |
| --- | --- |
| 成功 | `200` |
| `BAD_REQUEST` | `400` |
| `UNAUTHORIZED_DESKTOP` | `403` |
| `NOT_FOUND` | `404` |
| `CONFLICT` | `409` |
| `UNSUPPORTED`（服务版本过旧） | `501` |
| `SERVICE_UNAVAILABLE` | `503` |
| `TIMEOUT` | `504` |
| `INTERNAL` / `IO_ERROR` | `500` |

成功：

```json
{ "ok": true, "data": { "...": "..." } }
```

失败（HTTP 状态码见上表）：

```json
{ "ok": false, "error": { "code": "SERVICE_UNAVAILABLE", "message": "服务未连接，请先安装并启动 QingLinkService" } }
```

`data` 在无返回内容时省略或为 `null`。

> **框架层拒绝不走信封。** axum 在进入 handler 之前就可能拒绝请求：方法不匹配
> （`405`，空 body）、缺少 `Content-Type: application/json`（`415`，text/plain）、
> JSON body 缺字段或类型不符（`422`，text/plain）。这些响应**没有** `{ok:false}` 信封，
> 客户端必须按状态码兜底（本项目的 Web 前端把 400/405/415/422 映射为可读的 `BAD_REQUEST`）。

### 2.2 错误码表

| code | 含义 | 界面建议 |
| --- | --- | --- |
| `OK` | 成功 | — |
| `BAD_REQUEST` | 参数缺失或非法 | 提示参数错误 |
| `SERVICE_UNAVAILABLE` | 服务未安装、未运行或管道断开 | 引导"安装 / 修复服务" |
| `UNSUPPORTED` | 服务版本过旧，不支持该命令 | 引导"修复服务（升级）" |
| `UNAUTHORIZED_DESKTOP` | 服务拒绝当前桌面端的身份（可执行文件已重建，可信记录失效） | 引导"修复服务"重新登记 |
| `NOT_FOUND` | 目标不存在 | 刷新列表 |
| `CONFLICT` | 状态冲突（如扫描进行中） | 提示稍后重试 |
| `TIMEOUT` | 管道请求超时 | 建议重试 |
| `IO_ERROR` | 本地文件 / 日志读写失败 | 显示详情 |
| `INTERNAL` | 其他内部错误 | 显示详情 |

### 2.3 时间与分页

- 所有时间戳为 **Unix 秒**（整数），日志条目额外提供 `timestamp_ms`（毫秒）。
- 分页统一使用 `offset` / `limit`，`limit` 上限 100，默认 50；**日志接口除外**
  （`GET /logs` 与 `GET /logs/service` 为 1–500，默认 200，见 4.21）。
- 分页响应统一形状：

```json
{ "items": [], "offset": 0, "limit": 50, "total": 123, "next_offset": 50 }
```

`next_offset` 为 `null` 表示没有更多。

### 2.4 为什么用 POST + JSON body 而不是 RESTful 路径参数

应用条目与设备条目的 `id` **可能包含 Windows 路径**（例如 `C:\Program Files\App\a.exe`），
把这类值放进 URL 路径会引入百分号编码歧义。因此"针对某个对象"的写操作统一是
`POST /api/admin/v1/<资源>/<动作>` + JSON body（`{"id": "...", ...}`）；`GET` 接口的 `id`
走 query string，前端必须用 `encodeURIComponent` 处理。

---

## 3. 接口清单

基础路径：`http://127.0.0.1:3722/api/admin/v1`

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/health` | 进程存活与版本 |
| GET | `/overview` | 仪表盘一次性聚合数据 |
| GET | `/settings` | 读取桌面端设置 |
| POST | `/settings` | 修改桌面端设置 |
| GET | `/service` | 服务连接状态与能力 |
| POST | `/service/pause` | 暂停 / 恢复对外服务 |
| POST | `/service/reconnect` | 跳过重连退避，立刻重新握手并重新协商能力 |
| POST | `/service/install` | 安装或修复 Windows 服务（提权） |
| POST | `/service/uninstall` | 卸载 Windows 服务（提权） |
| POST | `/service/start` | 启动 Windows 服务（提权） |
| POST | `/service/stop` | 停止 Windows 服务（提权） |
| POST | `/service/restart` | 重启 Windows 服务（提权） |
| GET | `/service/install-state` | 安全输入服务安装体检结果 |
| GET | `/diagnostics` | 一次性完整诊断报告（只读） |
| GET | `/diagnostics/bundle` | 纯文本诊断包（附件下载） |
| POST | `/desktop/open` | 用默认浏览器打开本界面 |
| POST | `/desktop/restart` | 重启桌面端进程 |
| POST | `/desktop/exit` | 退出桌面端进程 |
| POST | `/dialog/file` | 在 PC 本机弹出原生文件选择窗口，返回绝对路径 |
| GET | `/devices` | 已配对设备分页 |
| POST | `/devices/revoke` | 撤销设备令牌 |
| GET | `/pairing` | 当前待确认配对请求 |
| POST | `/pairing/cancel` | 取消配对请求 |
| GET | `/apps` | 应用 / 游戏目录分页（含隐藏项） |
| GET | `/apps/stats` | 目录统计 |
| GET | `/apps/icon` | 取应用图标（PNG，按需解析并落库，可用 `v=` 破缓存） |
| GET | `/apps/icon/candidates` | 该条目可选的图标文件清单 |
| POST | `/apps/icon/refresh` | 重新解析并落库一个条目的图标 |
| POST | `/apps/icon/set` | 指定图标文件（等价于带 `icon_path` 的 refresh） |
| POST | `/apps/favorite` | 加入 / 移出「常用（我的库）」 |
| POST | `/apps/pinned` | 置顶 / 取消置顶到手机首页 |
| POST | `/apps/rename` | 重命名应用（写入服务端） |
| POST | `/apps/hidden` | 隐藏 / 取消隐藏 |
| POST | `/apps/delete` | 从目录移除（软删除） |
| POST | `/apps/scan` | 触发重新扫描 |
| POST | `/apps/add` | 新增自定义应用 / 游戏 |
| POST | `/apps/launch` | 在 PC 上启动一个条目 |
| POST | `/apps/batch` | 批量隐藏 / 显示 / 删除 / 恢复原名 / 重扫图标 |
| GET | `/logs` | 查询日志 |
| GET | `/logs/stats` | 日志统计 |
| GET | `/logs/files` | 两个来源的磁盘日志文件清单 |
| GET | `/logs/service` | 解析 `QingLinkService` 磁盘日志并分页 |
| DELETE | `/logs` | 清空日志 |
| GET | `/logs/download` | 下载日志文件 |
| GET | `/update` | 当前程序更新状态快照 |
| POST | `/update/check` | 后台检查 GitHub Release 是否有新版本 |
| POST | `/update/download` | 后台下载已发现版本并校验 SHA256 |
| POST | `/update/install` | 启动新 exe 副本自更新安装并退出桌面端 |
| WS | `/events` | 实时事件推送 |

---

## 4. 接口详情

### 4.1 `GET /health`

```json
{ "ok": true, "data": {
  "status": "ok",
  "version": "0.1.0+13",
  "pid": 12345,
  "uptime_secs": 3600,
  "web_port": 3722,
  "log_level": "info"
} }
```

`version` 是桌面端产品版本号 `0.1.0+13`（语义版本 + 随源码改动自增的构建号，见
[`development.md`](development.md) §2.4），与 Windows 文件属性的 `FILEVERSION` 一致。

`uptime_secs` 自**进程启动**计，而不是自 Web 服务启动计：迁移、安全输入探测、
`--restart-parent` 等待与托盘探测都发生在 Web 服务之前，用后者会让一个刚完成慢启动
的健康进程报告出只有几秒的存活时间。

### 4.2 `GET /overview`

仪表盘一次拉取，避免首屏并发请求。

```json
{ "ok": true, "data": {
  "health": { "status": "ok", "version": "0.1.0+13", "pid": 12345, "uptime_secs": 3600, "web_port": 3722, "log_level": "info" },
  "service": {
    "connected": true,
    "paused": false,
    "network_ready": true,
    "address": "0.0.0.0:3721",
    "last_error": null,
    "capabilities": {
      "management": true, "pairing": true, "pause": true,
      "devices": true, "catalog": true, "catalog_management": true,
      "app_add": true, "app_launch": true, "app_icon_manage": true,
      "keep_awake": true
    }
  },
  "settings": { "autostart": true, "keep_awake": false, "debug_log": false },
  "install_state": {
    "package_available": true,
    "service_installed": true,
    "service_running": true,
    "service_binary_matches": true,
    "network_ready": true,
    "service_protocol_ok": true,
    "available": true,
    "detail": "服务已安装并运行",
    "busy": false,
    "result": null
  },
  "tray": {
    "visible": true,
    "verified": true,
    "rect": { "x": 3364, "y": 2064, "width": 64, "height": 96 },
    "icon_size": 32,
    "error_code": null,
    "integrity": "medium",
    "shell_integrity": "medium",
    "attempts": 0,
    "blocked_reason": null,
    "detail": "Shell_NotifyIcon(NIM_ADD) 已接受托盘图标"
  },
  "stats": {
    "apps": 128, "hidden_apps": 3, "devices": 2, "logs": 4521, "errors": 0
  },
  "pairing": null
} }
```

- `health` 是 `GET /health` 的完整对象（含 `status` 与 `web_port`）。
- `service.address` 在服务未就绪时为 `null`；`service.capabilities` 在 `connected=false` 时为 `null`。
- `tray` 在托盘尚未创建时为 `null`（不能假设它总是对象）。

`install_state` 的 `network_ready` / `service_protocol_ok` 语义、三态判定，以及
"两个同名 `network_ready` 含义不同"这一点，**与 4.8 完全一致**（含那张组合表），
这里不再重复；`/overview` 里 `service_protocol_ok` 恒存在，不要当可选字段跳过。

`stats.logs` / `stats.errors` 统计的是**内存缓冲**（最近 5000 条）内的条数，更早的记录在
磁盘文本文件里；依赖服务的计数（`apps`/`hidden_apps`/`devices`）在服务不可用时为 `0`，
此时 `service.connected` 为 `false`，界面应显示引导态而不是"0 个应用"。

`tray` 的判读（`visible` 只反映单次 `Shell_NotifyIcon(NIM_ADD)` 的返回值；`rect` 保留用于
兼容现有响应、目前不参与注册判定；详细排障见
[`architecture.md` 第 6 节](architecture.md#6-托盘子系统含历史缺陷)）：

| 字段 | 含义 |
| --- | --- |
| `visible` | `Shell_NotifyIcon(NIM_ADD)` 是否被 shell 接受；`false` 表示图标没有注册成功 |
| `verified` | `Shell_NotifyIconGetRect` 是否确认图标已在通知区域（折叠区也算已注册） |
| `error_code` | 上一条失败时 shell 的 `GetLastError`（`5` = 拒绝访问） |
| `integrity` / `shell_integrity` | 本进程与通知区域所有者各自的完整性级别；不一致时图标一定注册不上 |
| `rect` | `GetRect` 返回的真实坐标；`null` 表示图标在折叠区或未注册 |
| `attempts` | 已尝试的注册次数；永久拒绝时为 `0`（没有重试） |
| `blocked_reason` | 非空 = 重试无法修复（通常是完整性级别不匹配），界面应直接展示它 |
| `detail` | 面向用户的结论，界面会原样展示 |

`pairing` 为 `null` 或：

```json
{ "request_id": "uuid", "device_id": "mobile-abc", "device_name": "轻联手机",
  "pin": "123456", "expires_at_unix": 1767225600 }
```

仅当桌面处于解锁状态时才返回非 `null`：锁屏时无法确认配对，因此不下发 PIN。

### 4.3 `GET /settings`

```json
{ "ok": true, "data": { "autostart": true, "keep_awake": false, "debug_log": false } }
```

### 4.4 `POST /settings`

请求体字段均可选，只传需要修改的：

```json
{ "autostart": true, "keep_awake": true, "debug_log": false }
```

响应返回修改后的完整设置。`autostart` 写注册表 Run 项；`keep_awake` 经桌面管理管道
下发 `set_keep_awake` 给服务进程，由服务调用 `SetThreadExecutionState`
（`ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED`）阻止系统休眠与息屏；
`debug_log` 切换日志级别。`keep_awake` 持久化在服务库 `settings.power.keep_awake`，
`debug_log` 在桌面端偏好 `logging.debug`。

> 托盘、设置页与移动端控制中心读写服务进程的**同一个**开关：桌面端按服务快照同步
> 显示，不再自己持有请求；服务启动时恢复，桌面端重启/退出不会取消它。服务未连接时
> 写入失败并返回错误，界面保持原值。

### 4.5 `GET /service`

```json
{ "ok": true, "data": {
  "connected": true,
  "state": "connected",
  "expected_version": 2,
  "service_version": 2,
  "service_app_version": "0.1.0+13",
  "paused": false, "network_ready": true,
  "address": "0.0.0.0:3721", "last_error": null,
  "capabilities": { "management": true, "pairing": true, "pause": true,
                    "devices": true, "catalog": true, "catalog_management": true,
                    "app_add": true, "app_launch": true, "app_icon_manage": true,
                    "keep_awake": true }
} }
```

`connected` 是 `state === "connected"` 的兼容别名；界面应优先使用 `state`：

| `state` | 含义 | 界面动作 |
| --- | --- | --- |
| `connecting` | 尚未收到首个快照，也还没有失败 | 显示"正在连接"，不要立刻引导重装 |
| `connected` | 管理链路可用 | 正常渲染 |
| `protocol_mismatch` | 管道已应答，但协议版本与桌面端不一致（服务过旧） | 引导「修复服务（升级）」 |
| `unauthorized` | 服务拒绝当前桌面端的身份（可执行文件已重建，可信记录失效） | 引导「修复服务」重新登记 |
| `unreachable` | 管道不可达（服务未装/未跑，或执行环境拒绝） | 引导安装 / 启动 |

`expected_version` 是桌面端使用的协议版本，`service_version` 是服务实际应答的协议版本
（从未应答过时为 `null`）。`service_app_version` 是**服务端产品版本号**（与服务端
`GET /api/v2/health` 的 `app_version` 同源，形如 `0.1.0+13`），用来确认两端是不是同一次
构建；旧服务不返回该字段时为 `null`，界面应显示占位符而不是空版本。
`state != "connected"` 时 `capabilities` 为 `null`。

### 4.5.1 `POST /service/reconnect`

无请求体。跳过重连退避，立刻重新握手（并丢弃能力缓存重新协商）：

```json
{ "ok": true, "data": { "accepted": true } }
```

权威结果由 `service/status` 事件回流，接口本身不等待。

### 4.6 `POST /service/pause`

```json
{ "paused": true }
```

响应：

```json
{ "ok": true, "data": { "accepted": true, "paused": true } }
```

`paused` 回显的是**请求值**，不是权威状态；真实结果由 `service/status` 事件回流
（服务暂停后移动端无法连接，但服务进程与管道仍在，管理能力保留）。

### 4.7 `POST /service/{install|uninstall|start|stop|restart}`

无请求体。这些是**提权操作**：桌面端会拉起 UAC 辅助进程，接口**立即返回**
`{"ok": true, "data": {"accepted": true}}`，真实结果通过 `/service/install-state`
与 WebSocket `install_state` 事件异步反映。

### 4.8 `GET /service/install-state`

```json
{ "ok": true, "data": {
  "package_available": true, "service_installed": true, "service_running": true,
  "service_binary_matches": true, "network_ready": true, "service_protocol_ok": true,
  "available": true, "busy": false, "result": null, "detail": "服务已安装并运行"
} }
```

`busy` 为 `true` 表示提权操作正在执行。`result` 是上一次操作的结论文本。

`network_ready` 只表示服务的回环健康协议应答成功。它被拆出来是为了让失败可归因：

| 组合 | 含义 |
| --- | --- |
| `service_running=false` | 服务未运行，可执行修复 |
| `service_running=true, network_ready=false` | 服务在跑但健康端口不可达（绑定失败、防火墙，或受限执行环境） |
| `service_protocol_ok=false` | 服务在跑，但管理链路不可用（协议过旧或桌面端身份未登记）；`available` 仍可能为 `true`，因为这几项检查看不到协议层 |
| `available=true` 而 `/service` 的 `connected=false` | 服务健康，但桌面端**命名管道被当前执行环境拒绝**——不是服务故障，见 [`development.md`](development.md) 第 5 节 |

`service_protocol_ok` 由管理链路的实时状态推导：`true` 已握手，`false` 已知不可用，
`null` 未知（未安装、未运行或仍在连接）。它存在的理由就是消除"体检说可用、`/service`
说未连接"的自相矛盾。

### 4.9 `POST /desktop/open` / `POST /desktop/restart` / `POST /desktop/exit`

无请求体。

`/desktop/open` 只有主线程确认已交给 shell 后才返回成功；权限不匹配或 shell 拒绝时返回标准
错误响应，不会伪报成功：

```json
{ "ok": true, "data": { "url": "http://127.0.0.1:3722" } }
```

`/desktop/restart` 与 `/desktop/exit` **先回包再关闭进程**，因此响应只是"已受理"：

```json
{ "ok": true, "data": { "accepted": true } }
```

进程消失与重启进度通过 `system/shutdown` 事件与随后的自动重连体现。

### 4.10 `GET /devices?offset=0&limit=50`

```json
{ "ok": true, "data": {
  "items": [ { "id": "paired:3f2a", "device_name": "轻联手机" } ],
  "offset": 0, "limit": 50, "total": 1, "next_offset": null
} }
```

`id` 前缀 `paired:` 表示含 `device_id` 的新式令牌，`legacy:` 表示旧式令牌
（撤销时按前缀分派）。**`id` 必须原样回传给 `revoke`。**

> 字段名就是 `id`，不是 `device_id`，也没有 `kind`：令牌类型由前缀表达，
> 不需要界面另取一个字段。

### 4.11 `POST /devices/revoke`

请求体字段是 `device_id`（把列表里的 `id` 值原样放进来）：

```json
{ "device_id": "paired:3f2a" }
```

成功返回 `{"ok": true, "data": {"revoked": true}}`，服务端会同时关闭该设备的
控制连接与输入连接。撤销成功后桌面端会向所有浏览器广播 `devices/changed`。

> 注意列表用 `id`、撤销用 `device_id`，两者承载同一个值；这是历史命名差异，
> 客户端只需原样回传，不要自行剥离 `paired:` / `legacy:` 前缀。

### 4.12 `GET /pairing`

```json
{ "ok": true, "data": { "pairing": null } }
```

有请求时 `pairing` 为 4.2 中展示的对象。仅在桌面已解锁且配对可用时返回，
否则恒为 `null`。

### 4.13 `POST /pairing/cancel`

```json
{ "request_id": "uuid" }
```

成功返回 `{"ok": true, "data": {"cancelled": true}}`；请求已完成 / 已过期返回
`NOT_FOUND`。

### 4.14 `GET /apps?kind=app&offset=0&limit=50&query=&include_hidden=true`

| 参数 | 取值 | 默认 | 说明 |
| --- | --- | --- | --- |
| `kind` | `app` \| `game` | `app` | 目录分类 |
| `query` | 任意文本 | 空 | 对 `name`/`original_name`/`category` 做不区分大小写子串匹配 |
| `offset` | 整数 | 0 | 分页偏移 |
| `limit` | 1–100 | 50 | 每页数量；超出范围会被夹到区间内，不报错 |
| `include_hidden` | `true`\|`false` | `true` | 管理界面默认包含隐藏项 |
| `favorite` | `true`\|`false` | `false` | 只返回「常用（我的库）」中的条目 |

`include_hidden` 与 `favorite` 是**两个独立的与条件**（服务端依次
`include_hidden || !hidden`、`!favorite_only || favorite`）：`favorite=true` 不会顺带
带出隐藏条目，`include_hidden=false` 也不会顺带取消"只看常用"，同时给出时只有
"可见且被标记为常用"的条目会返回。`limit` 在桌面端先按 1–100 夹紧，再与服务端的
`MAX_PAGE_SIZE=100` 取 `min`，因此 `limit=1000` 得到的是 100 条而不是报错。

```json
{ "ok": true, "data": {
  "items": [ {
    "id": "C:\\Program Files\\App\\a.exe",
    "name": "我的编辑器",
    "original_name": "Editor",
    "exe_path": "C:\\Program Files\\App\\a.exe",
    "category": "Application",
    "source": "StartMenu",
    "hidden": false,
    "user_added": false,
    "args": [],
    "has_icon": true,
    "renamed": true,
    "icon_source": "image",
    "icon_updated_at": 1770000000,
    "favorite": false,
    "pinned": false
  } ],
  "offset": 0, "limit": 50, "total": 128, "next_offset": 50,
  "kind": "app"
} }
```

- `name` 是**当前生效的显示名**（用户重命名后即为新名）。
- `original_name` 是扫描得到的原始名，界面上可用于显示"原名为 X"。
- `renamed` 是 `name != original_name` 的便捷布尔值。
- `icon_source` 与 `icon_updated_at` 是**可选字段**：两者都可能整个缺失，也可能为
  `null`（从未解析过图标、或旧版服务没有这两个键），客户端必须按
  `string | null | undefined` 处理，不能因为它们不在就认为响应不合法。
- `icon_source` 说明图标是从哪里来的（见 4.33），`shell` / `none` / 缺失都表示
  这个条目的图标很可能不对，客户端应提供单独重扫（更换图标）的入口。
- `icon_updated_at` 是图标最后一次解析的 Unix 秒，用作图标 URL 的 `v=` 参数；
  为 `null` / 缺失时退回不带 `v` 的 URL。
- `has_icon` 只表示服务端**已经缓存**了图标。图标接口本身会按需解析并在成功后落库，
  所以即使 `has_icon=false` 也应当照常请求 `/apps/icon`，失败时再回退到占位符。

### 4.15 `GET /apps/stats`

```json
{ "ok": true, "data": {
  "app": { "total": 128, "hidden": 3, "user_added": 2, "renamed": 5, "favorite": 6 },
  "game": { "total": 42, "hidden": 0, "user_added": 0, "renamed": 1, "favorite": 3 }
} }
```

> 这里**没有**扫描时间字段：目录统计只描述当前目录内容。扫描是否在跑由
> `apps/scan_started` 事件与随后必然到达的 `apps/catalog_changed`
> （`reason="scan_finished"`）表达。

### 4.16 `GET /apps/icon?id=<encoded-id>`

- 命中返回 `200`，`Content-Type` 由载荷的魔术字节嗅探：`image/png` / `image/bmp` /
  `image/jpeg` / `image/gif` / `image/webp`；都识别不出时兜底为
  `application/octet-stream`。`Cache-Control: public, max-age=86400`。
- 未命中返回 `404`，JSON 错误信封。
- `id` 需要 `encodeURIComponent`；服务端会同时接受 exe 路径作为 id。
- 可选的 `v=<icon_updated_at>` 只用于区分缓存；换过图标后 URL 变化，
  浏览器就会重新取图，而不是继续用 24 小时的旧缓存。
- **按需解析的结果会落库**：第一次请求成功解析后，载荷写入 `icon_base64` /
  `icon_source` / `icon_updated_at` 并同步到内存目录，之后同一 id 的请求直接命中缓存。
  解析失败**不落库**，下一次请求会重试；因此同一条目不会在"有图标 / 没图标"之间摇摆，
  `has_icon` 也会在首次成功请求后变为 `true`。

**载荷保证**：返回的 PNG 恒为 `128×128` 正方形，素材居中且最长边铺满画布
（`max(ink_width, ink_height) == 128`）。前端可以直接把它放进固定尺寸的
`contain` 容器，不必再关心原始图标的分辨率或留白——早期版本会把外壳图像列表
返回的"大画布 + 左上角小素材"原样透传，导致部分图标只渲染出几个像素。
base64 长度上限为 128 KiB（解码后 96 KiB）；提取侧会降档重编码以保证不超限，
超限的载荷一律视为不可用而不是截断。

前端可直接 `<img :src="`/api/admin/v1/apps/icon?id=${encodeURIComponent(app.id)}`">`。

### 4.17 `POST /apps/rename`

```json
{ "id": "C:\\Program Files\\App\\a.exe", "name": "我的编辑器" }
```

约束：`name` 去空白后 1–128 字符（按 Unicode 标量计数），不得包含控制字符。
成功后**服务端立即生效**：移动端下一次拉取目录即看到新名字。
响应：`{"ok": true, "data": {"id": "...", "name": "我的编辑器", "renamed": true}}`
（`renamed` 是 `name != original_name` 的便捷布尔值）。

传空字符串表示**恢复原始名**（服务端会清空 `display_name`）。

### 4.18 `POST /apps/hidden`

```json
{ "id": "C:\\Program Files\\App\\a.exe", "hidden": true }
```

隐藏只影响移动端与目录列表的可见性，**不删除数据、不影响已保存的重命名**。
响应：`{"ok": true, "data": {"id": "...", "hidden": true}}`。

### 4.18b `POST /apps/favorite`

```json
{ "id": "C:\\Program Files\\App\\a.exe", "favorite": true }
```

「常用（我的库）」由**服务端唯一拥有**：手机端「库 → 常用」、手机首页置顶项、
这里的 `favorite` 字段读的都是同一份数据。以前它存在手机的 SharedPreferences 里，
换手机就丢、管理端也看不到，所以改成了服务端字段。

响应：`{"ok": true, "data": {"id": "...", "favorite": true}}`。
`favorite=false` 会同时清除置顶（不存在"置顶但不在常用"的状态）。

### 4.18c `POST /apps/pinned`

```json
{ "id": "C:\\Program Files\\App\\a.exe", "pinned": true }
```

置顶项会出现在手机首页的「常用应用」。`pinned=true` 会自动把条目加入常用。
响应：`{"ok": true, "data": {"id": "...", "pinned": true}}`。

### 4.19 `POST /apps/delete`

```json
{ "id": "C:\\Program Files\\App\\a.exe" }
```

软删除（`is_deleted=1`）：条目不再出现在任何列表，但数据保留、可被后续扫描重新发现为
新条目。响应：`{"ok": true, "data": {"deleted": true, "id": "..."}}`。

### 4.20 `POST /apps/scan`

```json
{ "kind": "app" }
```

`kind` 为 `app` 或 `game`。扫描在服务端后台线程执行，接口立即返回：

```json
{ "ok": true, "data": { "accepted": true, "kind": "app" } }
```

扫描进行中重复调用返回 `CONFLICT`。检测到游戏运行时返回 `CONFLICT`（扫描延后）。
完成情况通过 WebSocket `apps` 频道的 `scan_finished` 事件获知。

### 4.21 `GET /logs`

| 参数 | 说明 |
| --- | --- |
| `level` | 最低级别过滤：`trace`\|`debug`\|`info`\|`warn`\|`error`，返回该级别及以上 |
| `target` | 模块路径子串匹配，例如 `qinglink_desktop::tray` |
| `search` | 消息文本不区分大小写子串匹配 |
| `since` / `until` | Unix 秒，闭区间 |
| `limit` | 1–500，默认 200 |
| `offset` | 从最新往回的偏移 |

```json
{ "ok": true, "data": {
  "items": [ {
    "id": 4521,
    "timestamp_ms": 1767225600123,
    "timestamp": 1767225600,
    "level": "INFO",
    "target": "qinglink_desktop::web",
    "message": "Web 管理服务已监听 http://127.0.0.1:3722",
    "thread_name": "main",
    "file": "src/web/server.rs",
    "line": 88
  } ],
  "offset": 0, "limit": 200, "total": 4521, "next_offset": 200
} }
```

`items` 按**写入顺序倒序**（最新在前）返回（实现是 `entries.iter().rev()`，顺序由进程内
单调递增的 `id` 决定，**并不按 `timestamp_ms` 排序**）。`id` 在当前进程内单调递增，可用于
增量拉取；**它每个进程都从 1 重新开始**，因此跨重启比较 `id` 没有意义。

`thread_name` / `file` / `line` 三个键在无值时**整个键缺失**（不是 `null`），
客户端必须按可选字段处理。

`total` 是**内存缓冲里还有多少条**（默认上限 5000）。日志不写数据库，更早的记录在
磁盘文本文件里，通过 4.24 下载。

### 4.22 `GET /logs/stats`

```json
{ "ok": true, "data": {
  "total": 4521,
  "by_level": { "DEBUG": 1200, "INFO": 3100, "WARN": 210, "ERROR": 11 },
  "buffer_capacity": 5000, "buffer_len": 4521,
  "oldest_ms": 1767139200000, "newest_ms": 1767225600123,
  "files": [ { "name": "qinglink.2026-02-01.log", "size_bytes": 1048576, "modified_secs": 1767225600 } ]
} }
```

- `total` 与 `buffer_len` 都只描述内存缓冲；`buffer_capacity` 是它的固定上限。
- `oldest_ms` / `newest_ms` 是缓冲内记录的时间跨度。
- `by_level` **只包含实际出现过的级别**（例如只剩 `DEBUG`/`INFO` 时就没有 `ERROR` 键），
  客户端缺失的级别按 `0` 处理。
- `files` 是日志目录里的文件清单（含轮转日志与历史遗留文件），下载见 4.24。
  轮转文件名约定是 `qinglink.<YYYY-MM-DD>.log`。

### 4.23 `DELETE /logs`

清空内存缓冲（**不删除磁盘日志文件**）。

```json
{ "ok": true, "data": { "cleared": 4521 } }
```

清空成功后桌面端广播 `logs/cleared`，其他标签页据此同步清空本地列表。

### 4.24 `GET /logs/download?file=qinglink.2026-02-01.log&source=desktop`

- `source` 取 `desktop`（默认，桌面端日志目录）或 `service`（`%ProgramData%\QingLink\logs`）。
- 省略 `file` 时服务端在本来源的日志文件里取 `modified_secs` **最大**的那一个
  （`newest_log_name()` 按修改时间而非文件名排序），因此在遗留的
  `qinglink.log.<date>` 与当前的 `qinglink.<date>.log` 共存时不会选到遗留文件。
  服务端已按修改时间选择，**仍然建议界面显式传入 `file`**：从 4.22 的 `files` 里
  挑出最新的一份，下载的就是调用方看到的那一份（确定性），而不是"服务端此刻认为
  最新的那一份"。本项目的 Web 前端就是这么做的。
- 返回 `Content-Type: text/plain; charset=utf-8`，
  `Content-Disposition: attachment; filename="..."`。
- 文件名只允许 `[A-Za-z0-9._-]` 且不得包含 `..`；服务端在解析路径后**再次校验**
  它仍位于所选日志目录内，因此目录穿越不可行。
- 文件不存在返回 `404 NOT_FOUND`。

### 4.25 `GET /logs/files`

两个来源的磁盘日志清单（当前进程可见的"轮转日志"全集）：

```json
{ "ok": true, "data": {
  "desktop": [ { "name": "qinglink.2026-10-03.log", "size_bytes": 4726, "modified_secs": 1759500000 } ],
  "service": [ { "name": "QingLinkService.log.2026-10-03", "size_bytes": 8123, "modified_secs": 1759500000 } ]
} }
```

服务端目录不可读（未来若收紧 ACL）时 `service` 为空数组，而不是报错。

### 4.26 `GET /logs/service?file=&level=&target=&search=&since=&until=&limit=&offset=`

解析 `QingLinkService` 的磁盘日志并按桌面端日志相同的过滤语义返回一页。它是**磁盘快照**，
不会通过事件通道推送，因此响应形状与 4.21 相同（另带当前文件名）：

```json
{ "ok": true, "data": {
  "file": "QingLinkService.log.2026-10-03",
  "items": [ { "id": 1, "timestamp_ms": 1759500000000, "timestamp": 1759500000,
               "level": "INFO", "target": "qinglink_server::server",
               "message": "[remote-control] WebSocket service listening..." } ],
  "offset": 0, "limit": 200, "total": 1, "next_offset": null
} }
```

- 最多读取文件尾部 4 MiB、保留 5000 条记录，因此长跑服务的日志不会把一次请求变成无界读。
- 解析失败的行（多行消息、panic 回溯）会并入上一条的 `message`，不会被丢弃。
- 时间戳前缀无法解析时该条 `timestamp_ms` 为 `0`。
- 文件不存在或名字非法返回 `404 NOT_FOUND`。

### 4.27 `GET /diagnostics`

一次性的完整诊断报告（只读；`pipe_probe` 是服务端真正执行一次 v2 `capabilities` 握手）：

```json
{ "ok": true, "data": {
  "version": "0.1.0+13", "pid": 40248, "uptime_secs": 138,
  "data_dir": "C:\\Users\\me\\AppData\\Local\\QingLink",
  "log_dir": "C:\\Users\\me\\AppData\\Local\\QingLink\\logs",
  "service_log_dir": "C:\\ProgramData\\QingLink\\logs",
  "integrity": "medium", "shell_integrity": "medium",
  "web_interface": { "assets_embedded": 45, "index_bytes": 1450, "placeholder": false },
  "tray": { "visible": true, "detail": "...", "rect": null, "icon_size": 32,
            "integrity": "medium", "shell_integrity": "medium" },
  "install_state": { "...": "见 4.8" },
  "service": { "...": "见 4.5" },
  "pipe_probe": { "reachable": true, "version": 2, "code": "OK",
                  "message": "操作完成", "error": null }
} }
```

`pipe_probe.error` 是失败时的**完整因果链**（`{error:#}`），这是 `--check-service`
与界面诊断页共用的排障依据。握手失败时 `reachable` 为 `false`，且 `code` 换成
`PIPE_UNREACHABLE`、`version` 与 `message` 恒为 `null`：

```json
{ "reachable": false, "version": null, "message": null,
  "code": "PIPE_UNREACHABLE", "error": "连接命名管道失败: ... (os error 2)" }
```

因此界面不能假设 `code` 是服务端应答码、也不能假设 `version` 一定有值：
`reachable=false` 时只应展示 `error` 的因果链，并把 `code` 当作本接口自己的诊断码。

### 4.28 `GET /diagnostics/bundle`

返回 `Content-Type: text/plain; charset=utf-8` 的纯文本诊断包，附件名
`qinglink-diagnostics-<YYYYMMDD-HHMMSS>.txt`，内容依次为：诊断报告 JSON、桌面端日志尾部、
`QingLinkService` 日志尾部。两个日志各最多 512 KiB，整包上限 2 MiB（超出会截断并在包内注明）。
依赖为零：不生成压缩包，也不引入新的构建依赖。

### 4.29 `POST /apps/add`

```json
{ "path": "C:\\Program Files\\Example\\example.exe", "name": "", "kind": "app", "args": ["--fullscreen"] }
```

- `path` 必须是本机**已存在**的文件（服务端校验），否则 `404 NOT_FOUND`。
- `name` 留空时使用文件名；`kind` 归一化为 `app` / `game`。
- 服务端用规范化路径派生稳定 id（`user:<hash>`），因此同一路径重复添加返回 `409 CONFLICT`
  而不是产生重复行。
- 成功后广播 `apps/catalog_changed`（reason: `add`）。

```json
{ "ok": true, "data": { "id": "user:9f86d081884c7d65", "name": "example", "kind": "app" } }
```

### 4.30 `POST /apps/launch`

```json
{ "id": "user:9f86d081884c7d65" }
```

在 PC 上启动该条目。服务运行在 Session 0，但启动会转发到交互会话，因此用户能看到窗口。
条目不存在返回 `404 NOT_FOUND`。

```json
{ "ok": true, "data": { "launched": true, "id": "user:9f86d081884c7d65" } }
```

### 4.31 `POST /apps/batch`

```json
{ "action": "hide", "ids": ["C:\\a.exe", "C:\\b.exe"] }
```

`action` 取 `hide` / `show` / `delete` / `restore_name` / `refresh_icon`；`ids` 最多 100 个。
`restore_name` 等价于用空名调用 4.17（清空显示名覆盖，恢复扫描名）。
`refresh_icon` 对每个条目执行一次 4.33 的自动重新解析，因此单独限制为最多 50 个
（每条都要做一次 shell 取图，再多的批量会超出本地传输超时）。

逐条执行并逐条汇报，部分失败不会中断整批：

```json
{ "ok": true, "data": {
  "results": [ { "id": "C:\\a.exe", "ok": true },
               { "id": "C:\\b.exe", "ok": false, "code": "NOT_FOUND", "message": "目录中不存在该条目" } ],
  "succeeded": 1, "failed": 1
} }
```

---

### 4.32 `GET /apps/icon/candidates?id=<encoded-id>`

列出这一个条目在 PC 上可用的图标文件，供用户直接挑选。目录里已经有
`icon.png` / 同名 `.ico` 的程序（Electron、Unity、便携版游戏）就是靠这一步修好的。

```json
{ "ok": true, "data": {
  "id": "steam:570",
  "candidates": [
    { "path": "D:\\Steam\\steamapps\\common\\dota 2 beta\\game\\bin\\win64\\dota2.exe",
      "kind": "executable", "size_bytes": 98304000 },
    { "path": "D:\\Steam\\appcache\\librarycache\\570_icon.jpg",
      "kind": "image", "size_bytes": 4821 }
  ]
} }
```

`kind` 取 `executable` / `shortcut` / `shortcut_icon` / `image`；顺序即推荐顺序。
列表最长 40 项，不存在的文件不会出现。

---

### 4.33 `POST /apps/icon/refresh` 与 `POST /apps/icon/set`

```json
{ "id": "C:\\Program Files\\Example\\Example.exe" }
```

`icon_path` 可省略。**省略时按下面的顺序自动重新解析**并写回服务端：

1. Steam 条目优先用 Steam 已经下载好的封面（`source` 为 `steam`）：扫描猜出的主程序
   经常是错的，而封面是这些条目唯一可靠的图源，所以它排在整条链的**最前**。图源是
   `appcache/librarycache`，三种排布都认：`<appid>_icon.jpg`（旧版平铺）、
   `<appid>/header.jpg` 这类按 appid 分目录的具名资源，以及新版内容哈希布局
   （`<appid>/<sha1>/header.jpg`，方形图标为 `<appid>/<sha1>.jpg`）。
   选择顺序是**方形图标 → 横版头图 → 竖版胶囊 → logo**，都找不到才退回第 2 步；
2. 快捷方式（`.lnk`）用 `IShellLink::GetIconLocation` 指定的外部图标与其
   **图标序号**（`some.dll,3` 这类快捷方式以前会取到错的图）；
3. 快捷方式目标的程序内嵌资源；
4. 程序同目录（以及上一级）的 `icon.png` / `icon.ico` / `logo.png` /
   同名 `.png`、`.ico`；`.ico` 优先交给外壳按尺寸取最佳条目，因此没有 32bpp
   条目的图标不再整条解析失败；
5. 系统默认图标（`source` 为 `shell`，通常就是"图标不对"的那一类）。

指向**目录**或不存在路径的条目不会走第 5 步（系统默认图标那一步）：Steam 里没找到
主程序的条目 `exe_path` 就是安装目录，以前会因此展示成"文档"图标，现在返回
`source=none` 并由客户端显示占位符。

Steam 条目落库时的 `exe_path` 是**猜**出来的（清单里没有这项），规则是：在安装目录下
最多下挖 3 层（覆盖 Unreal 的 `<项目>/Binaries/Win64/`）收集候选，排除崩溃上报器、
前置运行库安装器、卸载器，再按"名字与安装目录名 / 游戏名一致 > 互相包含 >
层级更浅、体积更大"排序取第一名；一个都挑不出来时 `exe_path` 才会退化成安装目录。
启动本身走 `steam://`，与猜出的 `exe_path` 无关。

带上 `icon_path` 时使用该文件，并把它记为条目的首选图标；再次调用且不带
`icon_path` 会清除这个偏好并回到自动解析。

```json
{ "ok": true, "data": {
  "id": "C:\\Program Files\\Example\\Example.exe",
  "source": "image",
  "icon_path": "C:\\Program Files\\Example\\icon.png",
  "icon_base64": "iVBORw0KGgo..."
} }
```

`source` 取值：`manual` / `steam` / `shortcut` / `executable` / `image` /
`shell` / `none`。`icon_base64` 可能为 `null`（没有可用图标），并且总是符合
§4.16 的 `128×128` 载荷保证。

> 图标管线变更后，服务端启动时会自动把已落库的旧图标重新解析一遍
> （`settings` 表里的 `icon.pipeline` 记版本，每个数据库只跑一次；
> 用户手动指定的 `manual` 图标只按原文件重新编码，不会被自动结果顶替）。
> 因此升级后不需要用户逐个点「重新扫描图标」。
>
> 客户端也可以直接提交一张位图作为图标（v2 协议的 `update_scanner_app` 带 `icon_base64`）：
> 服务端把它规范化为同样的 128×128 PNG，并把 `icon_source` 记成 `manual`；这种图标没有
> 对应的磁盘文件，同一次写入会清空 `icon_path`，图标回填时按"没有原文件就不动"处理。

`GET /apps` 的每一项**可选**带 `icon_source` 与 `icon_updated_at`（语义与 `v=` 缓存用法见 4.14）。

---

### 4.34 `POST /dialog/file`

浏览器看不到本机文件系统（`<input type="file">` 只回传不含路径的 blob），所以"在 PC 上
选一个文件"只能由桌面端进程弹出原生选择窗口代为完成，并把**绝对路径**回给界面。

```json
{ "kind": "image", "start_in": "C:\\Program Files\\Example" }
```

- `kind`：`image`（图片与可执行程序）或 `executable`（只列可执行项）；其它取值返回
  `400 BAD_REQUEST`。
- `start_in`：可选，选择窗口的初始目录；空白字符串按未提供处理。

```json
{ "ok": true, "data": { "path": "C:\\Program Files\\Example\\icon.png" } }
```

用户取消时 `path` 为 `null`——这是**成功**响应，不是错误。

约束：

- **同一时刻只允许一个选择窗口**：已有窗口在等待时再次调用返回 `409 CONFLICT`
  （`已经有一个文件选择窗口在等待`）；窗口关闭或调用结束后该标记立即释放。
- 选择窗口跑在独立线程上（shell 组件是单线程单元，不能复用可能已被图标提取初始化为
  MTA 的线程池线程），因此请求会一直阻塞到用户作答；前端超时设为 5 分钟
  （`FILE_DIALOG_TIMEOUT_MS`）。该线程无法启动或异常退出时返回 `500 INTERNAL`。
- 返回的路径可直接作为 4.29 的 `exe_path` 或 4.33 的 `icon_path` 使用。

### 4.35 移动端游戏启动显示模式

该能力**只走移动端 `WS /api/v2/events`，不新增桌面管理 HTTP 接口**：`launch_mode`
（`width` / `height` / `refresh_millihz`）是游戏目录摘要上的可选字段，由 `update_scanner_app`
保存、`clear_launch_mode: true` 清除。候选模式集合、`INVALID_DISPLAY_MODE` 的判定、启动时的
切换与恢复策略（含 PID 差异识别、180 秒兜底、快照位置）见
[`protocol/v2.md`](../protocol/v2.md)「显示设置」与「游戏启动显示模式」。

### 4.36 移动端保持唤醒

同样**不新增桌面管理 HTTP 接口**：手机用 `get_keep_awake` / `set_keep_awake` 读写开关，
桌面端托盘与设置页经管理 IPC 的 `set_keep_awake` 命令操作**同一个**开关（`keep_awake`
能力位为 `false` 时判为不支持）。per-thread 语义、`KEEP_AWAKE_FAILED` 与持久化位置见
[`protocol/v2.md`](../protocol/v2.md)「保持唤醒」；`POST /settings` 的 `keep_awake` 字段
形状不变（`apps/desktop/web-ui/src/api/types.ts`）。

### 4.37 `GET /update`

读取当前程序更新状态快照，供「更新」页首次进入时取初值。字段与 §5 的
`system/update_state` 事件数据完全一致：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `phase` | string | 阶段：`idle` / `checking` / `up_to_date` / `available` / `downloading` / `ready` / `installing` / `error` |
| `current_version` | string | 本地版本（`qinglink_version::VERSION`，如 `0.1.0+28`） |
| `latest_version` | string \| null | 发现的最新版本，未发现时 null |
| `release_url` | string \| null | GitHub release 页面地址 |
| `notes` | string | 更新说明（release body） |
| `progress` | number | 已下载文件数 / 总文件数换算的百分比 0-100 |
| `downloaded_bytes` | number | 保留字段，当前为 0 |
| `total_bytes` | number | 保留字段，当前为 0 |
| `error` | string \| null | 最近一次错误，`phase=error` 时有值 |
| `busy` | boolean | 是否有后台任务在跑，前端据此禁用按钮 |
| `last_check_secs` | number \| null | 距上次成功检查的秒数 |

### 4.38 `POST /update/check`

后台请求 GitHub Releases API，筛选 `desktop-v` 通道 tag，与本地版本比较。无请求体。
返回 `{ "accepted": true }`；真实结果通过 `system/update_state` 事件回流。已在忙态时
直接返回、不改变状态。

桌面端进程启动时会自动调用一次同样的检查（不经 REST）。**检查发现新版本后自动进入
§4.39 的后台下载**，因此 `available` 通常只是瞬时阶段；用户只需要决定是否安装。
检查的三个完成分支（发现更新 / 已是最新 / 失败）都会各推送一次完整状态快照。

### 4.39 `POST /update/download`

后台下载已发现的版本：先取统一清单 `SHA256SUMS`（GNU sha256sum 行格式）并解析，
再下载 `QingLink.exe`，按清单里的期望哈希核对。清单缺失/格式无效/缺条目或哈希不符
都进入 `error` 阶段，绝不使用未通过校验的文件。无请求体，返回 `{ "accepted": true }`；
进度与结果经事件回流。下载保存在 `%LOCALAPPDATA%\QingLink\updates\<版本>\`。

这条接口主要用于**下载失败后的手动重试**：正常情况下 §4.38 发现更新时已经自动触发。
非 `ready` 也没有已发现版本时调用会返回 `accepted` 但不产生动作。

### 4.40 `POST /update/install`

仅在 `phase=ready` 时执行：以下载目录里的**新 exe 副本**加 `--self-update` 启动独立进程，
然后请求主循环退出。新副本等待旧进程退出后用自身覆盖目标 exe、提权重新登记服务信任
（服务校验桌面端 SHA256）、再拉起新进程；结果经 `--result` 文件回传，新进程启动时消费，
失败原因推送到更新状态。无请求体，成功返回 `{ "accepted": true }`。非 `ready` 阶段返回
`409 CONFLICT`（码 `CONFLICT`）。UAC 弹窗由服务重新登记触发，需用户确认。

---

## 5. WebSocket 事件

### 5.1 连接

```text
ws://127.0.0.1:3722/api/admin/v1/events
```

连上后服务端立即推送一帧 `system/hello`，紧接着推送一批**状态帧**，让新打开的页面
无需等待下一次变化就是正确的：`service/status`、`service/install_state`、
`pairing/pending`、`apps/catalog_changed`（`reason="resync"`），以及（已收到过服务快照时）
`devices/changed`。这些帧同样遵守订阅集合：未订阅的频道不会收到。

### 5.2 客户端 → 服务端

```json
{ "action": "subscribe", "channels": ["service", "devices", "pairing", "apps", "logs", "system", "notice"] }
{ "action": "unsubscribe", "channels": ["logs"] }
{ "action": "ping" }
```

默认订阅除 `logs` 外的全部频道（`system`、`service`、`devices`、`pairing`、`apps`、
`notice`）——日志量可能很大，必须显式订阅。

- `subscribe` 里**未知或非法的 channel 被静默忽略**：服务端只把在可订阅集合
  （`system` / `service` / `devices` / `pairing` / `apps` / `notice` / `logs`）里的名字
  加入订阅，其余不报错、不入集合，客户端不会收到任何提示帧。
- **客户端消息 JSON 解析失败会被直接丢弃**：不是合法 JSON、或 `action` 不在
  `subscribe` / `unsubscribe` / `ping` 之内时，服务端只记一条 debug 日志并继续读下一条
  消息，既不回错误帧也不断连。因此拼错 action 表现为"连接还在，但订阅没生效"，
  客户端需要自己按 `hello.channels` 与后续帧判断订阅是否成功。

### 5.3 服务端 → 客户端

统一信封：

```json
{ "channel": "service", "event": "status", "data": { } }
```

> **`data` 是该事件的对象本身，不含 REST 的 `ok` / `data` 信封。** REST 接口的响应是
> `ok()` 包出的 `{"ok":true,"data":{...}}`，WS 帧则把对象直接放进 `data`：例如
> `service/status` 的 `data` 就是 4.5 里 `data` 里的那个对象，取字段要写
> `frame.data.connected`，而不是 `frame.data.data.connected`。事件表里所有标注为
> "某节的 `data` 对象本身"的行都是这个意思。

| channel | event | 触发时机 | data |
| --- | --- | --- | --- |
| `system` | `hello` | 连接建立 | `{"server_time_unix":..., "channels":[...]}` |
| `system` | `pong` | 收到 `ping` | `{"server_time_unix":...}` |
| `system` | `shutdown` | 桌面端即将退出 / 重启 | `{"reason":"restart"\|"exit"}` |
| `system` | `settings` | 桌面端设置变化（接口写入或托盘菜单） | 4.3 的 `data` 对象本身 |
| `system` | `update_state` | 更新检查/下载/安装各阶段与进度变化 | 4.37 的状态快照对象本身 |
| `service` | `status` | 连接状态、暂停、网络就绪、地址或错误变化 | 4.5 的 `data` 对象本身 |
| `service` | `install_state` | 提权操作状态变化 | 4.8 的 `data` 对象本身 |
| `devices` | `changed` | 有效设备令牌数量变化（撤销、重新配对、过期） | `{"total": 2}` |
| `pairing` | `pending` | 出现 / 更新 / 消失待确认配对 | `{"pairing": {...}}` 或 `{"pairing": null}` |
| `apps` | `catalog_changed` | 重命名 / 隐藏 / 删除 / 常用 / 图标重扫 / 新增 / 扫描完成 / 丢帧重同步 | `{"reason":"rename"\|"hidden"\|"delete"\|"favorite"\|"icon"\|"add"\|"scan_finished"\|"resync"}` |
| `apps` | `scan_started` | 扫描开始 | `{"kind":"app"}` |
| `logs` | `entries` | 批量日志（合并推送，见下） | `{"items":[...]}`，元素形状同 4.21 |
| `logs` | `dropped` | 因速率限制丢弃 | `{"count": 12}` |
| `logs` | `cleared` | 日志缓冲被清空（任意标签页触发） | `{"cleared": 4521}` |
| `notice` | `toast` | 需要提示用户的异步结果 | `{"level":"info"\|"warn"\|"error","message":"..."}` |

`reason` 的取值与产生处一一对应：`rename`（4.17 与 `batch` 的 `restore_name`）、
`hidden`（4.18 与 `batch` 的 `hide`/`show`）、`delete`（4.19 与 `batch` 的 `delete`）、
`favorite`（4.18b，以及 4.18c 的置顶写入——置顶会连带改常用）、`icon`（4.33 的
`icon/refresh`、`icon/set` 与 `batch` 的 `refresh_icon`）、`add`（4.29）、
`scan_finished`（服务端扫描完成，广播机制见本节末）、`resync`（连接握手与丢帧重同步）。
未知 `reason` 必须按"目录可能变了"处理，不要因为没见过就忽略整帧。

除上表外，`POST /devices/revoke`（4.11）还会额外推送一条 `notice/toast`
（`level="info"`，消息为"设备已撤销，其对这台电脑的访问已断开"）；这是撤销的即时
用户反馈，不订阅 `notice` 的客户端只会收到 `devices/changed`。

`resync` 是服务端**主动的自愈帧**：能派发到浏览器的事件都是"变化才有"的，一旦某个
订阅者落后（慢标签页、日志洪峰），漏掉的帧无法靠增量补齐，服务端会重发上面的状态帧并
同时补一条 `logs/dropped`。客户端遇到 `resync` 只需当作"可能什么都变了"重新拉取。

`scan_finished` 的到达方式与众不同：扫描跑在服务端进程里，而命名管道是严格请求/响应、
没有服务→桌面端的推送通道，所以**桌面端**在它本来就有的 500 ms `GetSnapshot` 轮询里带上
每类的扫描完成时间戳，发现变化才向所有浏览器广播。浏览器侧不需要轮询，长扫描也不会漏掉
完成事件。

### 5.4 日志推送的合并与背压

`logs` 频道**不逐条推送**：服务端最多每 200 ms 合并一帧，单帧最多 200 条。
若在 200 ms 窗口内产生的日志超过上限，多余条目被丢弃并累计，下一帧附带
`{"channel":"logs","event":"dropped","data":{"count":N}}`。

界面在收到 `entries` 时应当**追加到顶部**（列表是最新在前），并在用户已滚动到顶部
时才自动滚动，否则保持位置并显示"有新日志"提示。

---

## 6. 典型调用流程

### 6.1 首屏加载

```text
1. GET  /api/admin/v1/overview          → 渲染状态卡片、统计、配对提示
2. WS   /api/admin/v1/events            → 订阅增量更新
3. GET  /api/admin/v1/logs?limit=200    → 仅当日志页可见时才请求
```

其余动作都是同一个形状：**一次写 + 一次事件驱动的重拉**。写接口见 §4.17（重命名）与
§4.20（触发扫描），随后到达的事件（`catalog_changed` 的 `reason`、`scan_started`）见 §5.3。

---

## 7. 前端实现约束

1. **`ok` 判定优先**，不要用 HTTP 状态码判断业务成功（但状态码是有意义的，见 2.1）；
   收到非信封响应（405/415/422）时按状态码兜底，不要报"响应不是合法 JSON"。
2. **不要轮询**：状态变化统一由 WebSocket 驱动；只在收到事件后重新拉取需要的分页。
3. **日志列表虚拟化**：单页最多 500 条，禁止一次性渲染数千条 DOM。
4. **重连策略**：WebSocket 断开后指数退避重连（1s→2s→4s→8s，上限 15s），
   重连成功后重新拉取 `/overview` 以校正状态；心跳长时间收不到任何帧时视为半开连接，
   主动断开并进入同一条重连路径。
5. **`SERVICE_UNAVAILABLE` 不要当作错误弹窗**，应呈现为"服务未安装/未启动"的引导态。
6. 所有写操作成功后**不要乐观更新**：写接口返回后重新读取，或等服务端事件回流。
7. **每次连接建立只补拉一次状态**。事件客户端的连接快照会随握手时间、延迟等字段变化
   反复通知，用它当"重连信号"会导致重复拉取；应按"连接代次"（generation）判断。
8. **`/overview` 是聚合接口**，同一窗口内到达的多个事件应合并成一次刷新。
9. **长文本不进列表行**。`exe_path` 与令牌 `id` 动辄上百字符，行里只显示末两段
   （`…\Folder\App.exe`）或"前缀 + 尾部"，完整值放进 `title` 与详情面板，并提供复制按钮。
   目录行的详情入口是 `AppDetailDrawer`（右侧抽屉，保留列表上下文），行本身也可点击打开。
10. **低频动作收进行内「⋯」菜单**（`RowMenu`）。行上只保留启动、常用、详情这类高频动作，
    否则每行六七个按钮会把界面撑成一片按钮墙。
11. **诊断页用 4.27 的两个只读接口**：`GET /diagnostics` 走普通 JSON 信封；`GET
    /diagnostics/bundle`（4.28）是 `text/plain` 的**附件下载**，必须按 Blob 取回并触发
    下载，不能对它调 `response.json()`，也不要把它当普通 JSON 接口去断言 `ok`。

### 7.1 当前实现采用的路由模式

内嵌的界面使用 **hash 路由**（`#/logs` 之类），因此深链接不会到达服务端。
服务端仍然实现了 history 模式的回退（未知非 `/api/` 路径返回 SPA 外壳），
所以将来切换到 history 路由不需要改后端。

当前 6 个目标页：`#/`（概览，含已配对设备）、`#/apps`（目录）、`#/logs`（桌面端与
服务端日志）、`#/diagnostics`（服务诊断：4.27 的链路状态 / 协议版本 / 自检报告与
4.28 的诊断包导出）、`#/settings`、`#/update`（更新：4.37 的状态快照与 4.38–4.40 的
检查 / 下载 / 安装动作）；`#/devices` 保留为重定向到 `#/`。

### 7.2 静态资源

- `/assets/**` 返回长缓存（`immutable`），因此**产物文件名必须带内容哈希**——包括
  沙箱内使用的无子进程回退构建（它同样给样式表加哈希）。固定名样式表 + 一年
  `immutable` 会让重建后的界面永远拿不到新样式。
- `index.html` 与未知非 API 路径返回 SPA 外壳，带 `no-cache`。
- `/api/**` 下的未知路径返回 **JSON 404**，绝不返回 HTML——否则客户端的
  `response.json()` 会抛异常并掩盖真正的问题。
- 后端**不依赖前端的资源命名约定**：被内嵌的其他文件由回退分支返回并带 1 小时缓存。

### 7.3 托盘诊断

`/overview` 的 `tray` 字段可用于在界面上解释"为什么看不到托盘图标"。
排障时也可运行 `QingLink.exe --self-check`，它会输出同样的结论并把渲染后的图标
写成 PNG。见 [`development.md` 第 4 节](development.md#4-自检)。

---

## 8. 相关文档

- [`architecture.md`](architecture.md)：模块划分、数据流、托盘与日志子系统设计。
- [`development.md`](development.md)：构建、调试、常见问题。
- [`protocol/v2.md`](../protocol/v2.md)：移动端对外协议。
