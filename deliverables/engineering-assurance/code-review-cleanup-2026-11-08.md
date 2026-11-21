# QingLink 代码质量审查与清理方案

**日期**：2026-11-08
**工作流**：工作流 1（综合代码审查）
**参与成员**：Cody（代码审查师）、Archi（系统架构师）、Tessa（测试与质量专家）、Docu（技术文档师）
**审查范围**：全仓库 —— Rust 233 个 `.rs`（`crates/` 204 + `services/` 29）、Dart 156 个、TS/Vue 165 个
**审查方式**：只读审查，未修改任何文件，未运行任何测试

---

## 📌 TL;DR（执行摘要）

- **整体结论**：底盘比预期干净得多——生产代码裸 `unwrap` **0 处**、`cargo check --workspace --all-targets` 通过且**无代码警告**、crates 侧 `#[allow(dead_code)]` **0 处**（是真干净，不是被 allow 掩盖的假干净）。所谓"代码脏"的表象，实际由三类结构性问题造成：**静默吞错、契约写两遍、分发层重复列举**。
- **严重度分布**：🔴 严重 4 项 / 🟠 高 20 项 / 🟡 中 18 项 / 🟢 低 12 项（跨四位成员的汇总口径；单个成员按其自身 lane 的结论会得到更少的 🔴，差异来自职责范围而非判断冲突）
- **定级判据（本报告统一口径）**：🔴 = 产生**用户可感知且与事实相反**的结果，或无任何可回溯痕迹；🟠 = 失败难以区分/难诊断，但结果不直接与事实相反；不要求"攻击者可触发"——那属于安全评审的另一条轴。
- **阻塞 / 非阻塞**：**没有阻塞发布的单项缺陷**，但有 1 条"一次 panic 即永久降级且全链无痕迹"的失败放大器，属必须先拆的高杠杆点。
- **可删/可消重的量**：重复与死代码合计约 **−1200 行**（去重 14 项约 −850 + 纯死代码约 −365），且无任何一条会让 537 行的棘轮文件 `requests_ws.rs` 变长，也不需要新建任何 crate 依赖边。
- **最反直觉的发现**：仓库存量最大的"脏"其实是**规则文件本身在说谎**——`AGENTS.md` §2.3 声称只有 1 个文件超硬上限，实际是 3 个。

---

## 🎯 核心结论卡片

| 项目 | 内容 |
|------|------|
| 整体评级 | 🟡 **有条件通过**（可作为清理依据；建议按批次执行，不要一次性大改） |
| 阻塞项数量 | 0 |
| 关键行动项 | 14 条（见行动清单），其中 4 条属"改 1 行/1 处"的极高性价比（清单 #1/#2/#3/#11） |
| 建议下一步 | 先做批次 1（纯删除 + 零行为变更，约 1000 行级 diff 但零逻辑风险），确认后再动批次 2/3 |

---

## 🔍 审查发现（按严重度排序）

### 4 条 🔴 严重：失败静默且不可诊断

| # | 严重度 | 类别 | 文件:行 | 问题描述 | 建议修复 | 来源 |
|---|--------|------|---------|---------|---------|------|
| 1 | 🔴 | 稳定性 | `server/ws_request_handler.rs:117` | **全局输入锁跨越整个请求生命周期**。`control.input_gate` 在**路由之前**获取，guard 活到请求返回；而 `RefreshScannerApps`（`ws_handler_legacy_scanner.rs:40` 的 `app_scanner.scan()`，无超时、无取消、无进度）与 `RefreshSteamGames`（`ws_handler_steam.rs:29-31`）都在锁内执行。同一把锁正是鼠标注入闸门（`state/input_gate.rs:27`）。表现：一次目录扫描期间**所有连接的**远程鼠标彻底失灵；更糟的是锁获取处（`:117`）把中毒当作永久 `SERVICE_UNAVAILABLE` 且**不打任何日志**，一次 panic 后所有远程指令永久失败，WebSocket 仍能握手、桌面页仍能打开，只能重启进程 | ①`input_gate` 下移到真正做注入的 handler 内，长任务（扫描/刷新）不持锁；②`:117` 改用仓库自带 `crate::lock_unpoisoned`（`lib.rs:25`，已用 86 处，唯独最关键这条没用）；③`else` 分支补 `tracing::error!`；④扫描加超时并把 `elapsed_ms` 打进日志 | Cody #37 / Archi #1 / Tessa #1·#27 |
| 2 | 🔴 | 错误处理 | `server/auth.rs:174-189` | `TokenStore::validate` 用 `.ok()` 把"取连接失败/查询失败"与"令牌不存在"同化为 `false`，并**无条件写入 60 秒校验缓存**。数据库瞬时抖动会让**全部在线设备在 60 秒内被判令牌失效**、被心跳踢线，日志无任何 DB 错误痕迹 | `validate` 改 `anyhow::Result<bool>`；`Err` 分支记 warn 且**不写缓存**；只有"查到但过期"与"查无此行"才缓存 false | Cody #1 |
| 3 | 🔴 | 错误处理 | `services/qinglink-service/src/windows_service_impl/helpers.rs:21/49/78/100/141`、`platform_windows_foreground_audio.rs:434` | 5 个 `helper_*` 与媒体键全部 `let _ = SendInput(...)`。输入注入失败（UIPI 拦截、会话错配、桌面未附着）被静默丢弃，调用方与日志都看不到——**"鼠标点了没反应"无法定位的根因**。媒体键函数文档还写着"失败被忽略，函数总是返回 `Ok(())`" | 返回 `anyhow::Result<()>`，用 `GetLastError()` 补原因；删掉"总是返回 Ok"的假 Result 文档 | Cody #3·#22 |
| 25 | 🔴 | **安全/正确性** | `server/auth.rs:225-241`；`server/ws_handler_auth.rs:43-46`；`apps/mobile/lib/src/providers/session_connector.dart:108-119`；`management/devices.rs:49-53` | **撤销失败被完整地呈现为成功，且事后不可证伪**。① 服务端：**5 种失败合流成一个返回合同**——`connection()` 失败、`unchecked_transaction()` 失败、`execute()` 失败、`affected == 0`（唯一诚实的那个）、`commit()` 失败；其中 :225-227 的 `.unwrap_or(0)` 把 **DELETE 执行错误洗成 `affected == 0`**，即"存储错误伪装成'这个令牌本来就不存在'"。4 种反向断言里最危险的是 `commit()` 失败：**事务回滚、令牌仍在库里且仍有效、`remove_cached` 未执行、校验缓存仍判 valid**。② 移动端：`revokeCurrentToken()` 走 map 通道（`remote_session_commands.dart:66-70`），`remote_command_bus.dart:36-42` 其实**正确**检出了错误，但 `session_connector.dart:111-115` 把异常 `catch` 掉、只在 `if (kDebugMode)` 里 `debugPrint`，然后**无条件继续**执行 `disconnect()` + 重置 `AuthState()` + **`clearToken()`** → 用户看到"已退出登录"，而 PC 端该令牌依然有效。**所以定级理由不是"文案写错"，是这条链让"撤销失败"这个事实在客户端不可见、在服务端无日志。** 另有 `let _ = DELETE FROM paired_devices` 吞错后仍 commit（令牌删了、设备记录残留）。同结构体 `cleanup()`（`:304-310`）**是**打了 `warn!` 的——属局部漏做 | 前三层（数据层三态）：`commit().is_err()` 单独返回 `Err` → `INTERNAL_ERROR` + `tracing::error!`；只有 `affected == 0` 才回 `NOT_FOUND`；两处 DELETE 改 `?` 并让 `revoke_hash` 返回 `Result`。**第四层（可见性，别做错方向）**：`catch-and-continue` 是**可辩护的 UX**（网络断了也不该阻止登出），**不要改成抛出去阻断登出**；应改可见性——把失败落到会话状态，界面能说"已退出本机，但 PC 端未确认撤销"；日志不能只在 `kDebugMode`（release 版零痕迹）；`clearToken()` 后可留一条"待确认撤销"记录。审计日志记 device_id | Tessa #12·#28 / Cody #12·#27 |

### 20 条 🟠 高

| # | 严重度 | 类别 | 文件:行 | 问题描述 | 建议修复 | 来源 |
|---|--------|------|---------|---------|---------|------|
| 4 | 🟠 | 错误处理 | `server/pairing.rs:132-141` | `.map_err(\|_\| error(500, "TOKEN_CREATE_FAILED", ...))` 把原始 `anyhow::Error` **整个丢弃**且无日志。磁盘满、库被锁、SQL 冲突在服务端呈现为同一句话 | `map_err` 内加 `tracing::error!(%e, ...)`；保留 source 链 | Tessa #3 |
| 5 | 🟠 | 错误处理 | `server/state.rs:109-147` | 启动期读 DB 静默失败：`paused` 读失败当 `false` → **安全语义反转**（用户点了暂停，重启后自动恢复对外服务）；目录缓存读失败当空库 | 失败即 `error!` 并保留标志位；暂停状态读不到时 **fail-closed**；启动日志显式打印 `paused=` / `cached_apps=` | Tessa #2 |
| 6 | 🟠 | 错误处理 | `ws_handler_legacy_scanner.rs:55-58` | 一次扫描的四个写入结果（`sync_scanned_kind` ×2、`mark_scan_completed` ×2）全丢弃，随后返回成功。界面刷新了但 DB 没写，重启回退；标记缺失导致每次启动重复全量扫描 | 逐条 `if let Err` + warn，把失败数进响应体；全失败返回错误码。同仓库 `catalog.rs:236-240` 对同样四个调用用的是 `?` | Cody #13 / Tessa #4 |
| 7 | 🟠 | 错误处理 | `service/app_db/database.rs:120-160` | `ensure_tables` 用 **11 个** `let _ = connection.execute("ALTER TABLE ...")` 补列，全部吞结果。迁移失败时初始化仍返回 Ok，后续以"no such column"在无关功能处失败 | 先查 `PRAGMA table_info` 再逐列 ALTER 且 `?` 传播；抽 `migrate_columns()`，顺带让 112 行的 `ensure_tables` 回到 80 行内 | Cody #2·#18 / Tessa #16 |
| 8 | 🟠 | 错误处理 | `custom_dir/persistence.rs:54,62-68` | `load_config` 用 `.ok()?` 把 IO 错误与 JSON 解析错误都变成 `None`，再 `unwrap_or_default()` 得空配置且零日志；随后用户新增一个目录即 `save()` 把**只剩新目录**的配置写回 → **静默数据丢失** | 返回 `Result`；解析失败 `error!` 并把坏文件重命名为 `*.corrupt-<ts>` 保留现场；`save()` 前若曾解析失败则拒绝写入 | Tessa #6 |
| 9 | 🟠 | 错误处理 | `apps/desktop/src/service/client/worker.rs:35` | **"点了没反应"的主干道只写 `debug!`**，而桌面端默认过滤级别是 `info`（`logging/subscriber.rs:21`）→ 日志文件里一个字都没有 | 升 `warn!`（连通性）/`error!`（命令失败），并打印 `ServiceError` 变体 | Tessa #7 |
| 10 | 🟠 | 错误处理 | `services/qinglink-service/src/logging.rs:9-15` | 日志目录建不起来时**整个函数静默返回**：Windows 服务无控制台、不写事件日志 → 全机零日志且无任何迹象。级别硬编码 `"qinglink=info"`，现场无法开 debug | 失败兜底到事件日志；级别读 `QINGLINK_LOG_LEVEL`，与桌面端共用同一变量名 | Tessa #8 |
| 11 | 🟠 | 稳定性 | 全仓库 | `std::panic::set_hook` **0 处**。未设 `panic = "abort"`，进程不死，但 panic 只走默认 hook → stderr；桌面端 GUI + 服务端均无控制台 → **字节流向虚空** | 在 `logging::install` 与 `init_service_logging` 各加一次 `set_hook`：先调原 hook，再 `tracing::error!(location, payload)`。**本报告投入产出比最高的一条** | Tessa #9 |
| 12 | 🟠 | 稳定性 | `server/ws_handler.rs:282` | `spawn_blocking(...).await.unwrap_or_else(\|_\| ...)` 把 `JoinError` 整个丢弃——panic 载荷（含 `unreachable!` 的 message 与位置）不留任何日志，客户端只收到一句 `INTERNAL_ERROR` | `Err(join_error)` 分支补 `tracing::error!(%join_error, "request task panicked")`。`request` 已在 `:270` clone 进闭包，直接可用 | Tessa #26 / Cody #38 |
| 13 | 🟠 | 可观测性 | `apps/desktop/web-ui/src`（58 处 `catch`） | 全部只 `notifications.error(...)` 或塞进 `ref`，**无一处上报**；日志页只能读 Rust 侧，没有前端写日志的通道 | 加客户端日志上报通道；`utils/errors.ts` 的 `toAppError` 是所有 catch 的唯一汇聚点，改一处覆盖 58 处 | Tessa #10 |
| 14 | 🟠 | 重复代码 | `protocol/responses_system.rs:33-121`、`responses_process.rs:191-235`、`responses_apps.rs:5-10` vs `crates/qinglink-protocol/src/platform.rs` | **11 个 DTO 逐字定义两遍**，且**已漂移 1 处**：`ForegroundResponse.focus_text` 在 server 侧有 `skip_serializing_if`（字段缺失），protocol 侧没有（发 `null`）。协议层的"单一事实来源"名存实亡 | 让 `WsServerResponse` 直接引用 `qinglink_protocol::*`，`responses_*.rs` 里的 11 份复本改成 `pub use`。约 −140 行 | Archi #3 |
| 15 | 🟠 | **正确性**（重复 + 语义分叉） | `server/ws_handler_scanner_management.rs`(247 行) vs `services/.../management/catalog_edit.rs`(100) + `apps.rs`(109) + `lookup.rs` | 目录写操作在**同一进程的两条传输路径**上各实现一遍，且**已核实 6 处语义分叉**，其中 4 处是 WS 入口（手机端唯一路径）完全缺失输入校验：① id 存在性（IPC 走 `find_entry`→`NOT_FOUND`，WS 直接 `db.delete_app(id)`，0 行也回成功）；② id 解析宽容度（IPC 认精确 id 或规范化 exe_path，WS 仅精确 id）；③ 显示名校验（控制字符/长度/trim）WS 无；④ exe 路径校验（空/控制字符/超长/`is_file()`）WS 无；⑤ args 清洗 WS 无；⑥ **重复添加**：IPC 用稳定 id `user:<sha256>` + `CONFLICT`，WS 用随机 UUID → 同一 exe 加两次得两行（`lookup.rs:101-104` 的注释明确记载了"重复添加应变成冲突"的设计意图，**该意图在手机端从未生效**）。用户可见症状：删了还在、加两次两条、且 5 个操作全部静默回 `true`（`launcher_controller.dart:168` 的 `_requestOk` 只判 `response['type'] != 'error'`，服务端回成功信封即恒为 true，删/隐藏/收藏/置顶/更新全部走这一句）。**最反常的一点**：`lookup.rs:3-4` 的模块文档写着"本模块是请求数据被允许变成目录数据的**唯一去处**"——这句话描述的只是一个**只对一半入口成立**的事实 | **根因不是"写了两遍"，是校验器被 `pub(super)` 钉死在 service crate 内**：`lookup.rs` 的 5 个函数（`find_entry`/`validate_display_name`/`validate_app_path`/`sanitize_args`/`app_id_for_path`）作用域封在 `management/` 内，而依赖方向是 service → server（`services/qinglink-service/Cargo.toml:18`），**`qinglink-server` 想复用也编译不过**，WS 入口是结构上无法校验而非忘记校验。修法：① 5 个纯函数下沉到 `qinglink-server` 的 catalog 服务层并改 `pub`；② 错误统一为 `CatalogError { code, message }`（两边形状本就相同：`record.rs:11-31` 的 `CommandError` 与 `WsServerResponse::error`），一个类型 + 两个渲染器；③ 5 个写操作收口为 `AppState` 上的 `pub` 方法，内部"校验 → 落库 → **判影响行数** → publish"；④ **db 层必须同改**：`entries.rs:218/250/267` 的 `rename_app`/`delete_app`/`set_hidden` 现返回 `Result<(), String>` 且不读受影响行数，应改返回 `usize`，否则只改 handler，`rename_app` 的静默无效仍留在 db 层。约 −150 行（**价值在消除分叉，不在行数**）。**实施顺序（唯一硬顺序是"步骤 0 合并 → 步骤 1 建索引"；步骤 ②/④ 之间无耦合）**：**步骤 0 必须先做**——一次性合并同 `exe_path` 的重复行：`is_favorite`/`is_pinned` 取逻辑或、`display_name`/`launch_mode` 取非空、其余软删；**收尾复验 `is_pinned = 1 AND is_favorite = 0` 计数为 0**，若不为 0 只能用 `UPDATE ... SET is_favorite = 1 WHERE is_pinned = 1` 修（方向反了会静默拆掉用户置顶，因 `usage.rs:60/:82` 两个 setter 是双向联动）。**存量重复未合并就建索引会直接失败。** ① WS 入口改调 `app_id_for_path`（`lookup.rs:105-108` 的 `user:<sha256前16位>`），并为 `exe_path` 建**部分唯一索引**：`CREATE UNIQUE INDEX ... ON scanner_apps(exe_path) WHERE is_deleted = 0 AND exe_path <> ''`——**不能是普通 UNIQUE**，否则两处必回归：`delete_app`（`entries.rs:250-261`）只置 `is_deleted = 1`、整行连 `exe_path` 都保留，而新增走 `INSERT OR REPLACE`（`entries.rs:116-118`）→ "删掉一个程序再加同一个"**不会报错，而是物理抹掉墓碑行**，历史标记静默丢失；WS 的 `handle_add_app` 无任何校验（`:76` 原样写 `exe_path`）→ 存量里两行空路径就让索引建不起来。**因此"WS 缺 exe_path 校验"不是附带项，而是一致性修复的前置条件**——两个缺陷互为因果（校验缺失 → 允许空/重复 `exe_path` 入库 → 唯一索引建不起来 → "合并重复行"的唯一手段被挡住，属本次审查里少见的"缺陷 A 让缺陷 B 不可修"）。**⚠ 步骤 1 自身还会引入一个更隐蔽的缺陷，必须同批解决**：改用稳定 id 后，同 exe 二次添加会命中同一主键，而新增语句是 `INSERT OR REPLACE`，其中 `display_name` 硬编码 `NULL`、`is_hidden` 硬编码 `0`、`is_favorite`/`is_pinned` 取调用方值（WS 传 `false`）→ **`REPLACE` 是"删了再插"，用户的收藏/置顶/隐藏/自定义名被静默清空**，且无需唯一索引参与、靠主键就会发生。IPC 侧不踩，是因为它落库前查存在性并返回 `CONFLICT`（`apps.rs:28-35`，注释原话"A stable id makes a repeated add an explicit conflict instead of a second row"），**WS 侧没有这一步**——所以本修复实际是把"产生重复行"换成了"抹除用户标记"，后者更难发现、更难恢复。故 **WS 新增必须同步补存在性检查 → `CONFLICT`**（仓内已有正确实现：`apps.rs:23-35` 先 `app_id_for_path`、再查 id 命中则 `CONFLICT`，注释原文"A stable id makes a repeated add an explicit conflict instead of a second row"；三条路径里两条有保护，没保护的恰好是手机端最常用那条）。**步骤 1 验收锚点两条**：① 同 exe 二次新增返回 `CONFLICT`；② 断言既有行的 `is_favorite`/`is_pinned`/`display_name`/`is_hidden`/**`created_at`** 与操作前逐字段相等——**`created_at` 是唯一能证明 `OR REPLACE` 真的没跑过的字段**（其余四个在"二次添加恰好带相同参数"时会假通过），而 `entries.rs:116-118` 恰把 `created_at` 与 `updated_at` 都绑成 `now`。**另有一条硬证据可直接压掉"这是设计取舍"的辩解**：同一个 `impl AppDatabase` 块内、`rename_app` 上方约 130 行的 `set_launch_mode`（`entries.rs:67-94`）**已经写对了**——`if updated == 0 { return Err("游戏条目不存在") }`；而 `rename_app:218`/`delete_app:250`/`set_hidden:267` 读到 `updated` 却不用（`rename_app:237-242` 甚至把它打进日志）。即**不是"没意识到"，是"写对过一次、没有推广"**——修复范式仓内已存在，不需要设计决策。**但它只能当"做法已存在"的证据，不能当实现模板**：`set_launch_mode` 的 `WHERE` 带 `lower(category) = 'game'`，所以"0 行"有三种原因（不存在 / 非游戏 / 已软删），文案却只有一句"游戏条目不存在"——照抄会把这种"三因一文案"一起带进来。实施时应写 `WHERE id = ?1 AND is_deleted = 0` → `NOT_FOUND`。同理，`catalog_edit.rs:35` 不要改成 `db.get_app`：`get_app:97-100` 转调 `get_apps(false)` 也是全表扫描，只是把 2 次全扫降成 1 次；正解是新写 `SELECT ... WHERE id = ?1`（理由不是性能——全仓仅 1 个调用点、低频——而是**注释承诺单行读、实现不是**）；② db 层改返回受影响行数 `usize`，0 行 → `NOT_FOUND`；③ 校验器收敛到 `qinglink-server`；④ **删除 exe_path 兜底**，两个入口统一"精确 id"——是"删"不是"推广"：`lookup.rs:22-24` 的注释称该兜底是为"exe 移动导致 id 变化"而设，但 `lookup.rs:29-38` 的 `wanted` 其实就是 id 参数、兜底做的是"接受把路径当 id 传"，而 id 由路径派生、本身稳定，故第 ① 步后它已无存在理由。其风险只有一种**窄回归**：仓外客户端"传路径当 id"的习惯会从成功变成 `NOT_FOUND`（仓内已查无此用法：`endpoints.ts` 6 个端点均传 id、`contracts/catalog.ts` 无 path→id 转换）。另需注意两处风险：**（a）兜底的"第一个匹配"排序键是显示名**（`entries.rs:34-35` 的 `ORDER BY COALESCE(NULLIF(display_name,''), name)`），一旦有重复行，命中的是"随重命名漂移的那一行"，比"任意一行"更难复现；**（b）去重迁移必须"合并"而非"择一"**——`is_favorite`/`is_pinned`/`display_name`/`launch_mode` 可能分散在不同重复行上，若按"保留 updated_at 最新的一行、其余软删"会静默丢标记，等于制造与被修问题同类的缺陷 | Cody #41 / Archi #4（上调） |
| 16 | 🟠 | 重复代码 | `server/ws_request_handler.rs:158-308` + 14 个 `ws_handler_*.rs` | **95 个请求变体被列举 212 次**（平均 2.2 遍）；四张路由表 151 行与各 handler 的 match 各列一遍；**21 处** `_ => unreachable!("request routed to the wrong handler")` + 1 处 `unwrap_or_else(\|\| unreachable!(...))`。**已脚本核实：95 个变体当前零遗漏，不是现网缺陷**，但下次加变体漏配即 panic，且两处 `match` 都带 `_` 兜底，编译器永远不会报错 | 各 handler 改返回 `Option<WsServerMessage>`，按序 `or_else` 串联；删四张路由表；链尾兜底返回 `UNSUPPORTED_REQUEST` 并 `tracing::error!`。净删约 146 行。**三个坑**：认领顺序会变成新的静默失败面（需逐文件核对认领集合）、链尾必须打日志、新错误码必须同步文档 | Cody #5 / Archi #5 |
| 17 | 🟠 | 不必要代码 | `services/qinglink-service/src/main.rs:22-46` | 5 个未文档化的调试开关 `--inject-move/-button/-button-state/-wheel/-key`，直接调 `SendInput`，**绕过 `input_gate`、`is_paused` 与认证**，无审计日志。全仓库除该文件外**零引用**（已验证：无文档、无脚本、无调用方）→ 约 170 行死代码 + 一条未审计输入通道 | 删 5 个分支 + `helpers.rs` + `windows_service_impl.rs:24` 的 `pub use`。**定性收紧**：需本地主动带参数启动该二进制才可达，不是远程可利用 | Cody #4 |
| 18 | 🟠 | 不必要代码 | `ws_handler_legacy_scanner.rs:10` | `GetScannerApps` / `RefreshScannerApps` 两个"旧版接口"变体**全仓库零消费者**：mobile、desktop web-ui、`docs/web-api.md`、protocol schema 均无引用，只剩 enum 与 handler 分支 | 删两个变体 + 实现（约 60 行）。**必须同批**更新 `protocol/v2.md`、`qinglink-v2-ws.schema.json`、`protocol/fixtures/*`；`requests_ws.rs` 顺带减 4 行 | Cody #25 |
| 19 | 🟠 | 错误处理 | `apps/mobile/lib/src/protocol/remote_protocol.dart:76`、`:110-114` | 服务端 WS 错误帧是 `{"type":"error","data":{code,message}}`（`responses_ws.rs:14` 的 `#[serde(tag="type", content="data")]` + `:140-145`），此文件却读**顶层** `json['error']` → 恒为 null → 固定返回「服务端返回了错误响应」。**成因可精确到一句：把 HTTP 契约的字段名套在了 WS 帧上**。当前零影响（`requestTyped` 无特性调用，54 处特性调用全走正确的 `remote_command_bus.dart:36-42`），但该通道被注释明确计划推广，迁移即 100% 丢码 | **先改文档（这才是根因）**：`protocol/v2.md:17` 原文写"响应使用 `{type, request_id, data\|error}`"，而实现只有一个 `data`（`responses_ws.rs:14` 的 `tag/content`）——`:76` 的 `json['error']` **不是笔误，是照规范写的**。只改代码不改文档，这个缺陷会重生。**最小修法是 1 行三元式——且必须把"从哪读"与"什么条件下读"写在同一行**：`final errorPayload = json['type'] == 'error' ? json['data'] : null;`。**这里有一个已踩过的坑，实施者必须知道**：只写成 `final rawError = json['data'];` 会**打挂整条 typed 路径**——`RemoteError.fromJson`（`:101-104`）对任何输入都返回非 null（`?? 'REMOTE_COMMAND_FAILED'`），而成功帧的 `data` 几乎必然是 Map → `error` 恒非空 → `isError`（`:72`）恒真 → `remote_session_commands.dart:50` 的 `if (response.isError) throw` 会对**每一次成功调用**抛异常。判定依据是**解析器行为**，不是字段类型。`RemoteError` 与 `RemoteResponse.error` 字段**保留**，`failureFromResponse:111`、`isError:72` 均不动，测试只改 fixture 1 行、三条断言不动；把 `code` 从 message 字符串提成结构化字段属另一件事（见架构总论第 6 项：仅降级、不销毁依据，🟢）。**零新增类型，但不要直接复用既有工厂**（本轮最容易被"看起来免费"误导的一处）：① 已有 `ServerFailure(code, statusCode)`（`core/errors/failure.dart:22-33`）**就是** WS 错误该用的类型，无需在 sealed 层级新增成员——它没被用上纯粹因为类注释把适用范围锁死为 HTTP，改一句注释 + `statusCode` 改 `int?` 即可，**全仓 `ServerFailure(` 构造点只有 1 处**（`http_transport.dart:109`），sealed 集合仍是 7 个；② `failureFromResponse` 改为"判 `response.type == 'error'` → 取 `json['data']` → 构造 `ServerFailure`"；③ **不要直接换调 `RemoteCommandException.fromResponse`**——该工厂（`core/network/remote_command_exception.dart:4-11`）**不校验 `type == 'error'`**，`data` 缺失时静默兜底成 `REMOTE_COMMAND_FAILED`，而 `RemoteResponse.isError`（`:72`）的 `error != null` 分支在 WS 上恒假、判据实际只剩 `type == 'error'` → 直接换调会把**成功帧变成幽灵错误**；若要用它，必须先让工厂自己判 `type` 并返回可空，守卫不许留在调用点；④ **严禁按 grep `['error']` 全局统一改**——`qinglink_pairing.dart:63-70` 读 `['error']` 对 HTTP 契约语义正确（虽然该分支是死代码，见收敛清单 #13），统一改会打断配对；按通道分：WS 取 `data`、HTTP 取 `error`，不做"两种都兼容"的抹平写法。**落地是"1 行判断 + 1 行构造隔离"，不要合并**：判断用 `type == 'error'`（**不能用 `isError`**——它含 `error != null`，那正是 WS 恒假的部分，用等于把"误判成成功"的隐患留着），且这是 lint `avoid_...release_mode_assertions`/`check` 的合规要求；构造再隔离成 `statusCode` 可空、供两种 envelope 各自构造。⑤ **`apps/mobile/test/remote_protocol_test.dart:21-32` 用 `{'type':'error','error':{...}}` 构造 fixture——它把错误的线格式写进了断言、且当前通过**。改 `:76` 后这条测试必红，实施者极易"为了让测试变绿而回退实现"，**必须同步把 fixture 改成 `data`**（纯改键名，净零行） **三源同改**：客户端（`remote_protocol.dart`）+ schema（`qinglink-v2-ws.schema.json` 的 `$defs.response.error`）+ 文档（`protocol/v2.md:17` 的 `{data\|error}`）必须**同批**改完——只改客户端等于第三次复制这个错误。 | Tessa #25 / Archi #2（收敛口径下调） |
| 20 | 🟠 | 规则一致性 | `AGENTS.md` §2.3 | 声称"当前仍有 1 个文件超出上限"，**实际 3 个**：`scan_folder_screen.dart` 659、`pairing_screen.dart` 650、`requests_ws.rs` 537。前两个是未登记的新增超限文件，按 §2.3"不设登记新债通道"已属违规，且棘轮对它们不生效 | 补登记两个 Dart 文件（§2.3 表格 1→3、文末摘要同步），并按 §2.2 拆分：`scan_folder_screen` → 抽 `_CandidateRow`/`_SelectBox`/`_FilterSegment` 到 widgets，`_CandidateIcon`(37 行) 换用已有 `widgets/app_icon.dart` 的 `AppIcon`，目标 ≈440 行；`pairing_screen` → 抽 `pairing_pin_card.dart`（约 220 行的 PIN 组件）与 `pairing_panels.dart`，目标 ≈270 行 | Archi #6 |
| — | — | — | — | **编号 #25 已上调为 🔴，见上方严重表**（裁决依据见「严重度裁决记录」） | — | — |
| 34 | 🟠 | **规则有效性** | `AGENTS.md` §2.4 | **自查命令自称可用、实际不可用**：正则 `\\test\\`（PowerShell 转义后匹配 `test` 前后各一字符）**不匹配 `tests.rs`**，且不排除三类非源码路径——Rust 目标目录的临时散列目录、`ephemeral_deliverables`、`.plugin_symlinks`。按该命令实跑输出 **11 条，其中 8 条是误报**（`requirements-57d9cc28` 4 条、`ephemeral_deliverables` 3 条、`.plugin_symlinks` 1 条）。**这正是 §2.3 棘轮清单漏登记 2 个文件的直接原因**：规则唯一的机器判据本身不可信 | 正则改 `[\\\\/]tests[\\\\/]`；排除集补 `ephemeral_deliverables`、`.plugin_symlinks`、`target`；更彻底的做法是改用 `git ls-files` 枚举（仓库已明确不设行数校验脚本，命令必须可靠才有意义） | Docu (A.2) |
| 35 | 🟠 | 文档契约 | `AGENTS.md` §3.1；`apps/desktop/web-ui/src/api/types.ts` | 两处会把人引向错误的文档入口：① §3.1 把 `crates/qinglink-server/src/server/**` 映射到 `docs/web-api.md`，而 `server/` 目录名在该文档 210 次 "server" 中出现**全部是"服务端进程"语义、不是路径**，实际契约在 `protocol/v2.md`——按表办事会改错文档，而这正是 §3.1 存在的目的；② §3.1 要求同步的 `types.ts` 只有 40 行、实际是 `export * from './contracts/...'` 桶文件，**端点级类型全在 `api/contracts/*.ts`**，按表改 `types.ts` 会把被 `contracts/apps.ts` 刻意消掉的 `unknown[]` 带回来 | §3.1 该行改成 `protocol/v2.md` + `api/contracts/*.ts`；`types.ts` 若保留桶角色就在文件头写明"此处不放类型定义" | Docu (A.4/A.5) |
| 36 | 🟠 | 协议文档 | `protocol/v2.md`；`protocol/README.md` | ① **v2.md 只覆盖 21 个下行响应类型，95 个上行请求类型一个都没列出** → 74 个未记录，占 78%（这解释了 #18 那两个旧接口为何长期无人发现）；② `GAME_ACTIVE` 错误码**未进 v2.md**，且移动端 7 个错误码分支里没有它 → 有正常文案的错误落进默认分支，被显示成通用失败；③ `protocol/README.md:16-19` 声称 CI 会校验 schema 与 fixture，但**仓库没有任何 CI 配置文件**，同时有 2 个无对应消息的孤儿 fixture | 为 95 个请求类型建"类型→载荷→响应"表（按功能域拆多份以避开 1200 行上限）；`GAME_ACTIVE` 进 v2.md 错误码表 + 移动端补分支（属新契约，需 `docs/web-api.md` 与 `api/contracts/*.ts` 同批）；README 的 CI 描述要么删除、要么补真 CI | Docu (B.1/B.2/B.3) |

### 18 条 🟡 中

| # | 严重度 | 类别 | 文件:行 | 问题描述 | 建议修复 | 来源 |
|---|--------|------|---------|---------|---------|------|
| 21 | 🟡 | 稳定性 | `crates/qinglink-windows/**` | **215 个 `unsafe {}` 只有 26 个 SAFETY 注释**；风险最高的六个文件全是 0：`foreground_audio`(19)、`focus_text`(18)、`secure_desktop/context`(13)、`launch_session`(10)、`mouse_input`(9)、`sas`(7)。`foreground_audio.rs:7` 把含纯 Rust 字符串处理的 57 行整个包进一个 unsafe 块 | `unsafe` 收缩到单个 FFI 调用粒度，逐块补前置条件说明。注意 `ipc/` 子模块的 SAFETY 注释质量很高，团队知道标准，只是没铺开 | Cody #8 |
| 22 | 🟡 | 稳定性 | `server/auth.rs:56-65` | 库 crate 内 `panic!`（rust-coding §4.5 禁止）；同函数 `ensure_schema` 失败却只 `tracing::error!` 后继续 → 令牌表建不起来时服务照常启动，之后**所有鉴权静默失败** | `TokenStore::new` 返回 `anyhow::Result`，`ensure_schema` 失败向上传播到启动流程 | Cody #9 / Tessa P4 |
| 23 | 🟡 | 稳定性 | `server/ws_handler.rs:307`、`input_ws_handler.rs:191/238` | `token_store.validate()` 是**同步 SQLite 查询**，直接跑在 tokio 工作线程；缓存未命中（60s TTL）时在异步运行时做阻塞 IO | 改走 `spawn_blocking`（与 `dispatch_client_request` 一致） | Cody #10 |
| 24 | 🟡 | 重复代码 | `service/app_db/`（54 处） | `map_err(\|e\| e.to_string())` 高度集中：entries 16 / usage 10 / steam 9 / scan_sync 8 / scan_state 7 / database 4。`rusqlite` 结构化错误被拍平成 String，无法区分 busy/constraint/not-found，也没法重试 | app_db 内部定义 `thiserror` 的 `AppDbError`（`#[from] rusqlite::Error`），只在协议/HTTP 边界转 String | Cody #11 |
| — | — | — | — | **编号 #25 已上调为 🟠，移至上方高严重度表**（两位成员对其定级有分歧，裁决依据见「严重度裁决记录」） | — | — |
| 26 | 🟡 | 错误处理 | `catalog.rs:98`、`launch_display.rs:162/169/192`、`icon_service.rs:220`、`steam_cache.rs:20`、`start_menu.rs:12/17` | 一组"写标记/写图标/读环境变量"吞错：图标流水线标记没写成 → 每次启动重跑全量重建；显示模式快照清空失败 → 下次按过期快照"恢复"到错误分辨率；`APPDATA` 缺失得空路径 → 开始菜单来源整体产出 0 条且与"真的没装"不可区分 | 统一 `warn!` + 保留现场；快照三处逻辑复制三份，抽 `restore_and_clear()` | Cody #14·#15·#16 / Tessa #5·#15 |
| 27 | 🟡 | 不必要代码 | `crates/qinglink-domain/src/capabilities.rs`(75 行)、`crates/qinglink-windows/src/lib.rs:32-98` | 领域层"能力契约"整层零调用（`PowerControl`/`ProcessControl`/`CapabilityError` 无构造点、无方法调用），两个 Windows 实现零引用；它还是 `qinglink-windows → qinglink-domain` 依赖边的**唯一理由** | 删两处 + `qinglink-windows/Cargo.toml:13` 依赖行，约 −135 行。**破坏性 API 变更，需人确认后单独一次改动**，并同步 `docs/architecture.md` §1 依赖表。注意 `qinglink-protocol/src/service_ipc/capabilities.rs` 名字接近但无关，不要一起删 | Archi #7 / Cody #39·#40 |
| 28 | ⏸ **暂不执行** | 重复代码 | `apps/mobile/lib/src/services/ws_client.dart:261-286` | 两份指数退避重连，其中一份**生产不可达**（`remote_session_types.dart:49` 传 `autoReconnect: false`）——判定成立。**但执行时发现本报告漏了一个依赖**：至少两处既有测试直接构造 `WsClient` 并依赖 `autoReconnect` 默认 `true` 与 `WsConnectionState.reconnecting`，删掉能力等于连同测试一起删。**在不能运行测试的前提下删有测试覆盖的能力，超出了"纯删除"的授权边界**，已回退、代码保持原样 | 需要连测试一起改：先确认这两处测试覆盖的是"生产不可达的行为"，再决定是删测试还是给 `WsClient` 补一个真实使用方。**建议连 `architecture.md` §5.1 那张指向死代码的表一起改** | Archi #9（**执行受阻，非结论错误**） |
| 29 | 🟡 | 可维护性 | `server/ws_request_handler.rs:67-108` | `mod auth { include!("ws_handler_auth.rs"); }` 把 **14 个** handler 塞进内联模块，12 个文件靠 `use super::*` 拿上下文 → IDE 跳转/重命名/依赖分析失效，等于 crate 内部可见性失效。与 §2.2-5 立法本意相悖 | **已核实为零风险机械替换**：改 `#[path = "ws_handler_auth.rs"] mod auth;`，`super` 仍解析为 `ws_request_handler`，父模块私有 helper（`is_supported_remote_key`:8、`scanning_is_paused`:39、`mouse_input_error`:47）对子模块本就可见。14 行对 14 行，零行为变更，应排在任何分发层重构之前 | Cody #7 / Archi #8（改判） |
| 30 | 🟡 | 稳定性 | `server/ws_handler.rs:97-101` | `Ok(event) = events.recv()` 模式下 `Err(Lagged)` 不匹配任何分支 → 事件静默丢失，慢客户端永远不知道漏了什么（`broadcast::channel(256)`） | 统计 lag 次数并在日志页暴露 `event_lag_count` | Tessa §2.3 |
| 31 | 🟡 | 可维护性 | 31 个 `.rs` 文件 | 内联 `#[cfg(test)]` 占生产文件行数预算，最高占比 49%（`service/mod.rs` 194/396）、48%（`ipc/codec.rs`）、44%（`icon_normalize.rs`）。§2.1 按物理行数判 500 上限，导致"生产 360 + 测试 87"与"生产 447"被判同档，**棘轮内外的 500 行含义不同** | 判据不改（保守执行）；在 `agent.md` 审查清单加一项：含内联测试的文件必须同时报告"生产段/测试段"行数。迁移到 `*_test.rs` 技术上零风险（`requests.rs:17-21` 已有先例与 `#[path]` 接线），但**字面踩 AGENTS.md §4 禁令，需人类授权** | Cody #35·#36 |
| 32 | 🟡 | 错误处理 | `server/auth.rs:333-346`；`services/.../management/snapshot.rs:45` | `count_active()` 把"取连接失败"（`:334` 的 `let Ok(connection) = ... else { return 0 }`）与"查询失败"（`:345` 的 `.unwrap_or(0)`）都变成 `0`，唯一调用方包成 `device_count: Some(0)` 发给桌面端 → **数据库抖动时仪表盘显示"已配对设备 0 台"**，用户会以为所有设备被解绑甚至被入侵清空。与 #2/#25 同一根因：把"失败"与"空/零"合并成一个值 | 改 `-> Option<usize>`，两条错误路径返回 `None`，`snapshot.rs:45` 直接透传。**零协议变更**：`qinglink-protocol/src/service_ipc/snapshot.rs:26` 已是 `pub device_count: Option<usize>`，`:36` 文档写明"旧服务不返回时是 `None`（版本未知）"——类型早就留好了"未知"态，只是被 `count_active` 用掉了 | Cody #42 |
| 33 | 🟡 | 可维护性 | `apps/mobile/lib/src/core/errors/failure.dart`(sealed，7 变体) vs `core/network/remote_command_exception.dart`(扁平 2 字段) | **手机端并存两套并行错误类型层级**，同一条错误跨两套转换必然丢字段：`ServerFailure` 有 `code` + `statusCode`（`failure.dart:31-32`），而 `RemoteCommandException` 只有 `code`/`message`，`statusCode` 无处安放；反向更糟——`ProtocolFailure`（`:46`）连 `code` 都没有，`failureFromResponse` 把 code 拼进 message 字符串（输出 `INVALID_TOKEN: …`）。这不是 #19 的补丁点，而是**#19 能"和平共存这么久"的环境前提**：正因为两套类型互不校验，一份解析写错了也没有任何机制会告警 | 收敛到一套（建议以 `QingLinkFailure` 为唯一，`RemoteCommandException` 降级为其适配器），或至少在 `docs/architecture.md` 写明"哪一层用哪个、边界在哪"。**不建议本轮动手**——它触及所有控制器的错误处理路径，属独立立项 | Archi（新增） |
| 40 | 🟡 | 错误处理 | `apps/mobile/lib/src/core/network/http_transport.dart:66-88`、`:45-47`；`services/qinglink_pairing.dart:63-70` | **HTTP 错误信封解码在同一层里有三种不一致**：① `delete()` **完全不解码**错误信封（只判状态码、body 被 `drain`），而同一文件的 `postJson`（`:41-44`）解码 → 取消配对失败只剩 `PAIRING_CANCEL_FAILED: 删除请求失败：<status>`，服务端结构化 code 被丢弃；② 非 2xx 且非信封（axum 的 400/405/415/422 纯文本拒绝）落到 `:45-47` 抛 `NetworkFailure`，再被 `qinglink_pairing.dart:78-80` 映射成 `PAIRING_NETWORK_ERROR` → **请求格式错误被报成网络故障**，用户被导向去查网络/防火墙；对照桌面端 `api/client.ts` 有专门的 `frameworkFailure` 处理这一类，**同一现象两端诊断结论相反**；③ `qinglink_pairing.dart:63-70` 手工读 `decoded['error']`，但它只在"2xx 且 `ok == false`"时可达，而 `:100` 对 2xx 直接返回 null → 不可达 | 抽 `_decodeEnvelope` 供 `delete` 复用；非信封的非 2xx 改抛 `ProtocolFailure`（与 web-ui 的 `frameworkFailure` 语义对齐）；删 `:63-70`。**净减代码** | Tessa #29 |
| 41 | 🟡 | **测试可信度** | `apps/mobile/test/remote_command_error_test.dart:6-15`（断言 `data`）与 `apps/mobile/test/remote_protocol_test.dart:21-31`（断言 `error`） | **两份互相矛盾的测试同时通过**。判据：*当两份测试对同一线格式断言不同、却同时为绿，测试已经不能充当契约守门人——必须先裁决哪份对，再做任何"修一处"的改动。* 这条的解释力高于任何单条症状：它正是"#19 这种错误能长期存活"的机制——任何人想修 `:76`，都会先撞上一份绿测试 | 先裁决（正确态是 `data`），再同批改 `remote_protocol_test.dart:25` 的 fixture（`'error'` → `'data'`，**断言无需改动**）。§4 合规措辞：这是**修正既有测试的输入 fixture**，不是新增测试 | Tessa（新增） |
| 42 | 🟡 | **契约维护** | `service/app_db/lookup.rs:22-24`、`:101-104`；`entries.rs:96` | **catalog 领域的注释被当作设计意图写了、但从未被当作契约维护**，三处合并看不是零散笔误：① `lookup.rs:22-24` 承诺"陈旧 id 可按路径解析"，实现是"接受把路径当 id 传"；② `lookup.rs:101-104` 承诺"稳定 id 把重复添加变成冲突"，而它**是纯函数、不产生任何冲突**，且 WS 路径根本不走它——**这条最严重，因为按注释推断会认为"重复添加是安全的"，而实际既不冲突也不安全**（注释把 hazard 写成了 guarantee，它承诺的 "same row" 在 `INSERT OR REPLACE` 语义下恰是数据被静默覆盖的**成因**）；③ `entries.rs:96` 的 `Load **one** game record`，实现是 `get_apps(false)` 全表扫描 + 线性查找 | 三处注释按实际行为重写（②还要顺带说明 WS 侧真实行为）；②的"重复添加即冲突"要等 #15 步骤 1 真正落地后才成立，注释不得先于实现改。属**文档与代码同批**改动 | Archi / Cody（新增） |
| 37 | 🟡 | 文档容量 | `docs/web-api.md`(1195)、`docs/architecture.md`(833) | web-api.md 距 1200 硬上限**仅剩 5 行**（`AGENTS.md` 仍写 1188，已过期 7 行），任何补充即触发拆分；architecture.md 已 833 行、刚过 800 建议值 | web-api.md **本轮不适用"按主题拆分"**（§4.x 编号被 `api/*.ts` 注释引用，拆分已推迟）：正解是先删被淘汰端点的整节（如旧版 WS 配对），**删比拆省事且立刻降行**；architecture.md 按 §1/§3/§5 分级拆附件 | Docu (A.3) |
| 38 | 🟡 | 文档一致性 | `docs/development.md` §7 | 称 "`todo = deny`"，而 `Cargo.toml` 实际是 `todo = "warn"`——唯一一处会让人误信"编译期必拦 todo"的描述（全仓 `todo!` 实为 0，所以不影响现状，只影响可信度） | 改文档为 `warn`，或把 `Cargo.toml` 改成 `deny`（更符合原文意图，且已核实全仓无 `todo!`，改成 deny 是安全的） | Docu (B.4) |
| 39 | ❌ **撤回** | —— | `docs/architecture.md` §5.3 | **本条为假阳性，已撤回**。实测 `crates/` 只有 **7 个**目录（protocol / domain / storage / version / discovery / windows / server），§5.3 的 crate 表 7 行已含 `qinglink-version`；报告原先引用的 `:634/:651/:658` 在 833 行的文件里是 §8.4/§8.5 的无关内容（行号取自更早的文件版本）。`.opencode/rules/architecture.md` §1 同样含 `qinglink-version`。**两条工作流都没漏，是审查时把"7 行"误读成"7 个 crate 里没有 version"。** 教训已并入「待完善」第 9 条 | 无需改动（`docs/architecture.md:370` 的"7 个 crate"是正确值，本次一度被我改成"8 个"，已回滚） | Docu (B.5) 误报 |

### 🟢 低（12 项，合并列出）

| 类别 | 位置 | 内容 |
|------|------|------|
| 可维护性 | `services/.../dispatch.rs:242` | `_ => unreachable!()` 无消息，违反 rust-coding §3.2 |
| 可维护性 | `secure_desktop/{mouse,key,text}.rs`（6 处） | `let _ = super::attach_input_desktop()?;` —— `?` 已传播错误，`let _ =` 只丢弃 `()`，纯噪声且误导 |
| 可维护性 | `catalog.rs`、`ipc.rs`、`launch_display.rs:137` | 公开 API 文档整篇英文，违反 AGENTS.md §1；`expect` 未说明不变量 |
| 稳定性 | `ws_handler_system.rs:231`、`requests_ws.rs:97` | `256` 硬编码在校验、文案、注释三处靠人工同步 |
| 稳定性 | `platform/window.rs:16-50` | 三处重复 session-zero 守卫（刻意降级，非缺陷），缺的是文档 |
| 可维护性 | `platform/stub.rs:1-3` vs `:179-180` | 文件级文档称"每个函数都返回不支持错误"，`enum_windows` 实际返回 `Ok(vec![])` |
| 不必要代码 | `qinglink-discovery/src/lib.rs:20` | `pub fn default_port()` 全仓库零引用（唯一确认零引用的公开 API），可直删 |
| 不必要代码 | `crates/qinglink-server/src/{scanner,view,assets}/` | 三个空目录残留（未进 git），本地删除即可 |
| 死代码 | `web-ui/src/stores/logs.ts`、`apps.ts` | 只写不读成员约 45 行（`logs.ts` 的 `reloadFirstPage`/`newestLogFile`/`download`、`apps.ts` 的 `catalogVersion`/`lastChangeReason`/`currentKindStats`），另有 `apps.ts:277` 注释声称消费者存在但实际无 |
| 死代码 | `mobile/services/remote_session_client.dart:207` | `sendTyped` 只有定义、零调用 |
| 死代码 | `mobile/services/qinglink_pairing.dart:63-70` | HTTP 非 2xx 已在 `http_transport.dart:41-44` 抛异常，成功路径也不会回 `{ok:false}`，`if (decoded['ok']==false)` 及后续错误分支永不可达（−8 行，**删除前置条件见收敛清单 #13 注**） |
| 稳定性 | 7 处 `OffsetDateTime::now_utc`（`usage.rs:122`、`entries.rs:303/335` 等）vs 5 处 `SystemTime::now`（`entries.rs:116/185/206`、`usage.rs:86/102`），共 12 处 | **时钟混用**：两种时钟生成 `updated_at` 写同一列 → `ORDER BY updated_at` 语义不统一。混用本身即"无统一约定"的证据，不只是风格问题 |

---

### 发现项的跨层分布（避免误读为"纯 Rust 后端问题"）

**42 条主项**按实际代码归属（每条按"改哪里才能修好"归一次，不重复计数）：**Rust 后端（crates/ + services/）27 条**、**移动端（Flutter）5 条**、**桌面端（Rust host + web-ui）2 条**、**规则与文档（`AGENTS.md` / `protocol/` / `docs/`）7 条**、**跨端 1 条**（#25，Rust 服务端 + Flutter 移动端各有一半）。

其中 🔴 的 #1/#2/#3 三条全部在 Rust 后端，#25 是唯一跨端项；42 条里有 **9 条属"同一语义写多遍或写分叉"**（#14/#15/#16/#24/#26/#28/#33/#40/#41），**移动端占其中 4 条**——这说明**重复代码是本仓库跨端共性的问题，不只是后端问题**，收敛时应按"契约"而非按"语言"组织。

**但影响面最大的 🟠 不在重复代码，而在正确性**：#15 的 6 处语义分叉全部落在手机端唯一的写入口上，症状是"删了还在、加两次两条"，而桌面端同一操作是正确的。这是本报告里唯一"同一功能在两个入口行为不同"的类别，排期上应与错误处理批次同批，不应等到结构性去重。

### 严重度裁决记录（成员分歧的最终定性）

四位成员对四项的定级存在分歧，以下是主理人的最终裁决与依据。**报告内只保留一个编号一个级别**，分歧过程留档于此。

| 条目 | 分歧 | 最终定级 | 依据 |
|------|------|---------|------|
| #25 撤销失败与"设备不存在"不可区分 | Tessa 主张 🔴；Cody 主张 🟠，理由是"今天那句假文案到不了用户眼前" | **🔴（但强制与 #19 同批）** | 采用 Tessa 的方向，但把理由换成**可验证的事实**，而不是文案措辞：`session_connector.dart:110-113` 把 `revokeCurrentToken()` 的失败整个 `catch` 成一句 `debugPrint` → **用户看不到任何提示，只会以为自己已经登出/已撤销**，而服务端令牌因事务回滚仍然有效——这是"与事实相反的结果"，且只有 debug 级痕迹。Cody 的约束一度被认为成立，**后被其本人撤回**：`revoke_token` 走 `remote_command_bus.dart:36-43`（已正确读 `data`），#19 只影响 typed 路径，两者**无耦合**。真正让定级成立的最后一环是**失败不可恢复**：`:118` 的 `clearToken()` 删掉了本地令牌，用户连重试撤销的机会都没有，而重新配对**不会让旧令牌失效**——威胁模型正好命中"丢手机后的唯一动作就是撤销"。 |
| #15 目录写操作 6 处分叉 | Archi 初判"去重项"；Cody 补出 6 处证据后 Archi 自认应上调 | **🟠（正确性）** | 症状用户可见、且是手机端唯一写入口，排期进批次 2 而非批次 3 |
| #19 typed 通道错误码丢失 | Archi 初判"现网全坏 🔴"；Tessa 修正为"当前零使用、迁移即触发" | **🟠** | 主理人复核 `remote_command_bus.dart:35-42`（读 `data`，正确）与 `RemoteRequest` 使用点（特性层零调用），采纳 Tessa 的收窄版本 |
| #5 WS 分发层 | Archi 初判"必然而非偶然的现网缺陷" | **🟠（潜伏风险）** | 三方脚本核实 95 变体 × 4 路由表 × 14 handler 零遗漏，降为结构性风险，排期从"止血"降为"改进" |

---

## 🏗️ 架构影响评估

### 结论：依赖方向合规，被侵蚀的是"单一事实来源"

- **依赖方向：合规。** 逐个核对 `Cargo.toml` 的实际依赖与 `.opencode/rules/architecture.md` §1 的表**完全一致**，无反向依赖、无循环；四个叶子 crate（protocol/domain/storage/version）零内部依赖。这块**不需要动**。
- **被侵蚀的点是契约双写**，不是 crate 边界：11 个 DTO 在 protocol 与 server 各定义一遍（#14），且已经漂移。
- **`include!` 装配**：全仓库共 31 处（14 个包在 `mod {}` 内 + 17 个平铺）。**只有那 14 个值得改**（机械替换、零风险）；另 17 个（`protocol/requests.rs` 6、`responses.rs` 8、`windows/platform_windows.rs` 3）改了要连带 `pub use` 兜路径，收益远小于成本，记为已知问题。

### 六个大文件：逐个明确结论

| 文件 | 行数 | 结论 | 理由 |
|------|------|------|------|
| `scan_folder_screen.dart` | 659 | **必须拆（硬违规）** | 一个 `ConsumerState` 承担扫描/选择/改名/提交 + 4 个私有 Widget + 手写 base64 图标解码 |
| `pairing_screen.dart` | 650 | **必须拆（硬违规）** | 1 个 state 类 + **7 个**私有 Widget 类，PIN 输入组件自成一体的 220 行 |
| `requests_ws.rs` | 537 | **不拆，只许变短** | 95 个变体必须同文件（Rust 穷尽性无法跨文件）；537 行里约 480 行是字段级文档注释，删注释属 §2.2.5 明令禁止的规避手段。**正确动作是删命令（#18）而不是拆文件**，且必须单独一次改动 + 同步协议文档 |
| `platform_windows_foreground_audio.rs` | 439 | **应拆** | 文件名自己承认：1-143 行是前台窗口+WinEvent 钩子，144-439 行是全部音频/媒体（12 个函数），两组无共享状态 |
| `stores/logs.ts` | 442 | **应拆** | 同一 store 有两套**几乎逐字相同**的分页状态机（desktop 源 vs service 源），另混入文件清单、下载、诊断包导出、过滤匹配、流式缓冲。抽 `logsStream.ts` + `logsFiles.ts` + `createPagedSource()` 工厂，约 −85 行 |
| `stores/apps.ts` | 415 | **不动，只删 3 个死成员** | 40+ 成员全围绕同一资源域（目录条目），是资源 store 的正常形态；与 `logs.ts` 的区别在于后者有"两套同构状态机并存" |
| `platform_windows_display_modes.rs` | 447 | **不动** | 13 个函数全部服务于"枚举→筛选→排序→切换显示模式"同一件事，447 > 400 只是超过**建议值** |

### 跨端重复：明确接受，但手机端内部必须收敛

web-ui 与 Flutter 的"协议客户端"服务的是**完全不同的信任域与传输**（本机回环 vs 配对 token），且 `apps/desktop` 不依赖 `qinglink-server` 是刻意的架构决策，强行共享会破坏这条边界。**接受双端独立实现。** 但手机端内部两份退避重连（#28）是真重复，必须删一份。

### 稳定性（架构级）

- **锁顺序无倒置**，这一点核过：`input_gate` 是唯一跨模块持有的锁，其下游只再取 `held_input`/`app_db`/`window_monitor`，这三个从不反向取 `input_gate`。但 `AppState` 里全是 `std::sync::Mutex`，"只允许在 `spawn_blocking` 内持有"目前**只靠人守，无类型级保障**，建议写进 `docs/architecture.md`。
- **supervisor 设计是本次评估里质量最高的部分**：`windows_service_impl/network.rs:17-112` 具备指数退避 1s→15s、`input_generation` 代际作废旧在途工作、监听器与发现同生命周期、优雅关停 + 1s 超时 abort、清理失败下轮重试。**唯一缺口**：端口一直绑不上时 15s 一次静默重试到天荒地老，没有"连续 N 次失败升级告警"的路径。
- **合规缺口**：`architecture.md` §2.4「每个 WS handler 必须有超时」**完全未落实**——`server/` 下 `grep "timeout("` 零命中（只有 `auth_timeout` 这个 sleep）。目前靠 `spawn_blocking` + 客户端超时兜底。

---

### 总论：跨四层的"乐观归一化"（本报告最重要的一条结构性结论）

这不是第 7 条缺陷，而是解释为什么下面 4 类问题会同时出现在 4 个互不相干的层里。四轮独立审查共核实 **6 处"失败或差异被乐观地归一化"**：

| # | 位置 | 归一化方式 | 销毁了什么 | 定级 |
|---|------|-----------|-----------|------|
| 1 | `state/input_gate.rs:21-35` | 输入帧被 `watch` 合并丢弃，无上报 | "曾收到输入" | 🟠（#1 的一部分） |
| 2 | `app_db/entries.rs:218/250/267` | 0 行受影响 → `Ok(())` | "目标不存在" | 🟠（#15） |
| 3 | `launcher_controller.dart:168` | `type != 'error'` 即视为成功 | "服务端拒绝了" | 🟠（#15 的症状面） |
| 4 | `ws_handler.rs:282` | `JoinError` 被 `\|_` 吞掉，无日志 | "处理线程崩了" | 🟠（#12） |
| 5 | `providers/session_connector.dart:108-119` | 撤销失败仍 `clearToken()` | "撤销未完成"，且**不可恢复** | 🔴（#25） |
| 6 | `remote_protocol.dart:113` | 错误码压进 message 字符串 | 结构化 code（**仅降级，不销毁依据**） | 🟢 |

**跨 4 层（平台 / 存储 / 协议 / UI）、无共享代码、无共同作者，形状完全一致。** 每一条单看都局部合理：`watch` 只保最新值是为了不堆积、`Ok(())` 是为了不泄漏 SQL 细节、`type != 'error'` 是为了宽容、`|_|` 是为了不 panic、`clearToken()` 是为了用户体验、拼字符串是为了日志可读。**合起来的效果是：任何一级的失败都到不了用户眼前。** 单点修复消灭不了这类问题——它不是某处写错，是每一层的默认选择同向。

**可复用的判据（建议进规范；已按第二轮意见收敛为三档）**——不只看"是否销毁依据"，还要看**用户能否从结果上发现**：

- **销毁依据 + 用户可发现**（`clearToken()`、丢弃输入帧）→ 🔴 或 🟠 前列，必须修；
- **销毁依据 + 用户不可发现**（0 行 → `Ok(())`、吞 `JoinError`）→ 🟠，必须修；
- **仅降级、不销毁**（错误码压进 message、时间戳差 4 行）→ 🟢，可延后。

**关键补正**：第 1 档与第 2 档的差别不在"技术严重性"，而在于**是否只有"客服投诉"一个入口**。Tessa 对扫描落库四连吞错的修正正好说明这点：它的形态被视觉上归进"错误处理"，但**实质是"删除/编辑看起来成功、实际没做"的可见症状链**，危害高于同表里若干 🟡 项。分档要按"用户能否发现"，不是按"代码在哪一层"。

**幂等操作例外**（同一成员的第二条补正）：上表 6 处全部是对**状态有持久效应**的操作，判据适用；`maximize(already maximized)`、`close(closing)` 这类幂等操作返回 `Ok` 是**正确**的，不要照此判据去"修"。

这条判据同时解释了为什么表内第 1-5 项必须修、第 6 项可延后——避免 6 条平铺时读者分不出轻重。

**附带一条从审查过程本身得到的教训**（值得写进 `docs/development.md`）：本轮 4 位成员共出现 6 次统计或判断偏差，其中 **3 次是同一类错误**——*假设"类型对了语义就对了"*（`awk` 在整文件测试模块处提前退出、`grep` 的排除条件自我过滤、把 `data` 直接喂给永不返回 null 的解析器）。共同点是**验证手段本身没有校验自己**。建议任何"批量计数"或"批量替换"结论在写入报告前，必须用一个具体反例反向验证一次。

## 🧪 故障可见性与错误处理评估（成熟度 3/5）

本仓库不新增测试、不跑测试套件（AGENTS.md §4），因此本节评估的是**"故障发生时，谁能发现、谁能定位、能不能自愈"**。

### 先校正统计口径（这是本轮最容易踩的坑）

我最初的初筛报了 `unwrap` 101 处"风险"，其中一半以上落在测试代码里。三方用三种方法交叉验证后一致：

| 构造 | 全仓 | 生产代码 | 差额去向 |
|------|------|---------|---------|
| `.unwrap()` | 101 | **0** | 94 在独立测试文件（`tests/`、`*_test.rs`、`tests.rs`）+ 7 在内联 `#[cfg(test)]` 段 |
| `.expect(` | — | **2** | `launch_display.rs:137`（守卫在 117 行，当前不可达）、`input_install.rs:287`（30 次循环内局部可证不可达，不在请求路径）；其余在测试与 3 个 `build.rs` |
| `panic!` | — | **1** | `auth.rs:57`，启动装配期，不在请求路径 |
| `unreachable!()` | 25 | **25** | 21 在 14 个 `ws_handler_*.rs`（`ws_handler_system.rs` 独占 8）+ 2 window_catalog + 1 dispatch + 1 `ws_request_handler.rs:154` |
| `let _ =` | 241 | **152** | 64 处不可改（`#[cfg]` 占位 10、Win32 句柄释放 54）、39 处 channel send 无订阅者属正常；**约 20 处真吞写操作**，已逐条定位 |

> `expect` 的全仓总数各方因 grep 模式不同有 90/93/105 三个值，本节不引用该总数，只引用三方无争议的生产段结论。

### 结论 A：当前不存在任何可被远程客户端触发的 panic 点

依据不是"统计数字小"，而是两条**正向验证**：
1. `WsClientRequest` 的 95 个变体 × 4 张路由表（`ws_request_handler.rs:158-308`）× 14 个 handler 逐条比对，**零遗漏、零错配**。
2. 攻击面已核实：帧长上限 128 KiB（`ws_handler.rs:58-59`）/ 64 KiB（`input_ws_handler.rs:42-43`）；`input_protocol.rs:47` 先校验长度再索引；`mouse_input.rs:21-25` 对位移与坐标做 `ensure!`/`clamp`。**这块做得扎实。**

### 结论 B：唯一真正的稳定性缺陷是"失败放大器"

未设 `panic = "abort"`，所以 panic 只杀任务不杀进程——这本来是好事。问题在于 `input_gate` 把"单次任务级故障"放大成"进程级、静默、需人工介入"：

```
一次 panic（哪怕来自未来某个 bug）
  → input_gate 中毒（不可复位，全程唯一的 ControlState 实例）
  → 此后每条远程指令返回"服务状态不可用"
  → WebSocket 依然握手成功、桌面管理页依然可用 → 看起来"服务是活的"
  → 只能重启进程
```

而且这条链上**四个环节全部无痕迹**：

| 断点 | 位置 | 丢失的信息 |
|------|------|-----------|
| 1 | 全仓无 `set_hook`，GUI/服务无控制台 | panic 消息、位置、栈 → 流向虚空 |
| 2 | `ws_handler.rs:282` 的 `\|_\|` | `JoinError` 里的 panic payload 与位置 |
| 3 | 客户端只收到 `INTERNAL_ERROR`，而手机端 typed 通道连这句都取不到（#19） | 命令名、request_id |
| 4 | `input_gate` 已毒化 | 后续每条指令都失败，且无日志 |

### 21 处 `unreachable!` 的准确定性

**是"事实成立、但只由人的纪律保证"的不变量断言；编译器完全不参与。** 证据：路由表与 handler 是两个**各自带 `_` 兜底臂**的独立 `match`，调用点传的是整个 `&WsClientRequest`（`ws_request_handler.rs:166`），没有任何类型收窄。系统域更极端——同一个变体被枚举 **3 次、跨 2 个文件**（`:167-184` 路由 → `ws_handler_system.rs:17-42` 中间层 → 8 个叶子函数各自再 match）。所以：**今天无害，下一次新增协议变体漏配路由即是生产上的远程可触发 panic + 永久故障。** 定级 🟠（潜伏风险），不是现网缺陷。

### 错误分层：不存在"所有错误都显示成一句话"的系统性问题

三端都有成型的分层：Rust 侧 `anyhow` + `thiserror` 分工清楚（`input_install.rs:206-251` 的 `?` + `context` 是全仓库标杆）；桌面端有 `ApiError{code,status}` + `frameworkFailure`；移动端有完整的 `sealed class QingLinkFailure`（Network/Server/Authentication/Protocol/Storage/Native/Unknown）。

真正的问题是**错误在传输途中被逐层压平**，且信息不是没有，是在三个具体位置被主动丢弃：`pairing.rs:135`（`map_err(|_)`）、`ws_handler_system.rs:159`（从文案里 `split_once(':')` 反推错误码）、`favorites_controller.dart:151`（`return false`，原因已拿到却写进 `state.error` 后 UI 不读，固定显示"操作失败"）。

---

## 🧹 重复代码收敛清单

| # | 重复实现 | 位置 | 预计删除 | 会让棘轮文件变长？ |
|---|---------|------|---------|------------------|
| 1 | 11 个 DTO 双份定义 | `responses_system.rs:33-121` 等 | −140 行 | 否 |
| 2 | 为同构 DTO 逐字段搬运 | `platform/{secure_desktop,volume_media,display,foreground}.rs` | −80 行 | 否 |
| 3 | 目录写操作两套实现 | `ws_handler_scanner_management.rs` vs `catalog_edit.rs`+`apps.rs` | −150 行 | 否（新建 `catalog_write` 约 180 行，未超上限；**不要塞进 `ws_request_handler.rs`**，会破 500） |
| 4 | `handle_set_hidden/favorite/pinned` 三对同形函数 | 同上 | −60 行 | 否 |
| 5 | WS 分发层四张路由表 | `ws_request_handler.rs:158-308` | −146 行 | 否（375 → 约 230） |
| 6 | 令牌认证两遍 | `ws_handler_auth.rs:12-27` vs `input_ws_handler.rs:187-214` | −25 行 | 否 |
| 7 | `mouse_input_error`/`key_input_error` 同构 | `ws_request_handler.rs:47-65` vs `ws_handler_display_input.rs:3-18` | −20 行 | 否（375 → 约 355） |
| 8 | 手机端两份退避重连 | `ws_client.dart:35-40,105-110,261-286,340-358` | −55 行 | 否 |
| 9 | 手机端图标解码/渲染 4 份 | `scan_folder_screen.dart:623-659` 等 | −40 行 | 否（659 → 约 440，**同时解决硬违规**） |
| 10 | 三份"解析服务端错误帧" | `remote_command_bus.dart:36-42`（对）、`remote_session_commands.dart:87-92`（对）、`remote_protocol.dart:74-85`（**写错**） | −12 行（**零新增**：`RemoteCommandException.fromResponse` 已存在且正确，只需删错的那份、改为委托它） | 否 |
| 11 | `logs.ts` 两套并行分页 + 死成员 | `logs.ts:110-153` vs `343-379` | −85 行 | 否（442 → 约 210） |
| 12 | `enum_windows` 三处 session-zero 守卫 | `platform/window.rs:16-50` | −12 行 | 否 |
| 13 | 手机端 HTTP 错误分支为死代码 | `qinglink_pairing.dart:63-70`：`postFrame` 对非 2xx 直接抛 `ApiException`（`http_transport.dart:41-44`），成功路径不返回 `{ok:false}`（服务端 `pairing.rs:202-214` 失败一律 4xx/5xx），故 `if (decoded['ok']==false)` 及 65-76 行永远是 false 分支 | −8 行 | 否（**但删除有前置条件，见下方注**） |
| 14 | 两种时钟生成同一列（12 处调用点） | 见上表 🟢 项 | −20 行 | 否 |
| — | **合计** | — | **约 −850 行**（14 行逐项相加为 853；**第 3、4 行可能部分重叠**——#4 的三对同形函数本就在 #3 的收口范围内，故按"≤ 853"理解，不要当精确值）。**纯死代码另计 −365 行**（#17 `--inject-*` 170 + #18 旧扫描变体 60 + #27 domain capabilities 135），不与本表相加重复 | **无一条会让 `requests_ws.rs` 变长，也不需要新建任何 crate 依赖边** |

> **第 13 行的删除前置条件**：`qinglink_pairing.dart:63-70` 目前是**唯一一处**按 HTTP 契约读 `error` 字段的代码。它本身是死分支，删掉不影响行为；但删之前必须**先把契约写进文字**（否则 #19 的修法会失去"HTTP 用 `error`、WS 用 `data`"的现场参照，后来者按 WS 统一改就会把配对改坏）。建议在同一次改动里：删死分支 + 在 `docs/web-api.md` 错误响应章节明确写出信封字段名。

### 不建议动的部分（同样是结论）

1. **`requests_ws.rs` 不拆**，理由见上；正确动作是删命令（#18），且必须单独一次改动。
2. **17 个平铺 `include!` 不动**（只有 14 个包裹在 `mod {}` 里的值得改）。
3. **`platform/stub.rs`（264 行）必须保留**——它是 workspace 在非 Windows 目标上 `cargo check` 能过的唯一保障，删了会让 CI 的非 Windows runner 崩。
4. **`apps/admin/`（1088 行）保留**——不参与构建，但 `apps/mobile/README.md:224` 引用它作为管理端设计稿；且 `.css` 不在 §2.1 行数覆盖范围内，不违规。
5. **`PairingRequest` 同名双定义**（domain 运行态 vs protocol 线格式）职责不同、都正确，改名波及 mobile + docs，收益仅可读性，不动。

---

## 📄 文档一致性核查

1. **`AGENTS.md` 的数据已失准**（本项目最高优先的文档问题，见 #20）：§2.3 表格与文末「当前清单摘要」都写"1 个超限文件"，实际 3 个。三处需要同步修改：§2.3 表格（增 2 行）、§2.3 上文"当前仍有 **1 个**"、文末摘要"§2.3 清单内的超限文件 | 1 → 3 / 清单内最大文件 → `scan_folder_screen.dart`（659 行）"。
2. **协议文档是"按主题组织"，不是完整消息类型注册表**：`protocol/v2.md` 344 行、13 个主题小节（配对/会话关闭/前台窗口/事件推送/图标/保持唤醒/显示设置/…），没有逐一列出 95 个变体。因此**无法用文档反查"某个变体是否已登记"**——这既是 #18 那两个旧接口长期无人发现的原因，也是下次加变体的风险点。
3. **`GetScannerApps` / `RefreshScannerApps` 是"代码有、文档也没有"**：Rust 侧有 enum 与 handler，而 `protocol/v2.md`、两个 schema、`docs/web-api.md`、mobile、web-ui 全部无引用。属纯遗留，删除不改变任何对外契约。
4. **`protocol/qinglink-v2-ws.schema.json` 只约束信封**（`type: string` + `additionalProperties: true`，仅对少数结构做具体约束），这是**刻意取舍而非遗漏**：95 个变体的逐字段 schema 一定会漂移，信封级不会。
5. **本次改动若执行，必须同步的文档**（依 AGENTS.md §3.1）：
   - 删除两个旧接口变体 → `protocol/v2.md`、`qinglink-v2-ws.schema.json`、`protocol/fixtures/*`、`requests_ws.rs`
   - 新增错误码 `UNSUPPORTED_REQUEST` → `docs/web-api.md` 错误码表、`apps/desktop/web-ui/src/api/types.ts`、`protocol/v2.md` §3.4
   - `requests_ws.rs` 行数下降 → `AGENTS.md` §2.3 表数字
   - 拆分两个 Dart 文件 → `AGENTS.md` §2.3 表格与文末摘要
   - 删除 domain capabilities → `docs/architecture.md` §1 依赖表
   - `include!` → `#[path] mod`（纯内部装配，不改线格式）→ **需同步三处规则的措辞**：`AGENTS.md:55`、`.opencode/rules/architecture.md:445`、`.opencode/rules/rust-coding.md:569` 现在一律禁止 `include!`，按"不禁止正当用法"改为"禁止用 `include!` 规避行数统计/隐藏代码"，否则同一条规则会被继续同时违反与遵守
   - **`protocol/v2.md:17` 的 `{type, request_id, data|error}` 需改成只有 `data`** → 这是 #19 的根因文档，不改则缺陷必然重生；改后 `protocol/README.md` 若也提到 `error` 需一并改
   - `protocol/qinglink-v2-ws.schema.json` 的 `$defs.response` 仍声明 `error` 属性并 `$ref` 它，而代码从不产生该键 → 去掉该定义并同步 `protocol/v2.md:17`（**schema + 文档 + 代码三处必须同改，只改其一就是这份报告里反复出现的那种"两份契约"**）
   - `requests_ws.rs` 若因删变体降到 ≤500 → §2.3 表格**整行删除**（不是改数字，因为该表只列超限文件）
6. **注释也是契约，本仓库存在"注释按设计意图写、但不按契约维护"的成片现象**（见 #42）：三处合并看不是零散笔误——`lookup.rs:22-24` 承诺的兜底实际换了语义、`lookup.rs:101-104` 承诺"重复添加变冲突"而它是纯函数、`entries.rs:96` 承诺"读一条"实际全表扫描。其中最危险的是第二条：**把 hazard 写成了 guarantee**，按注释推断会认为"重复添加是安全的"，而它承诺的 "same row" 在 `INSERT OR REPLACE` 语义下恰是数据被静默覆盖的成因。**建议 §3.1 补一行**：改函数行为时必须同批核对它的文档注释是否仍成立。
7. **HTTP 错误语义需按路由族写清**：`protocol/v2.md` 目前只说"非 2xx 即失败"，但 `/api/v2/health` 有 5 种非 2xx（`:168` 还写明"body 不保证是 JSON"）、admin 面有 axum 产生的无信封拒绝（`api/client.ts` 的 `frameworkFailure` 正是为此存在）。三处不补齐，客户端与文档就是三个不同假设：移动端归成 `NetworkFailure`、桌面端走 `frameworkFailure`、schema 却声明有 `error`。

---

## ✅ 行动清单

| # | 行动 | 负责角色 | 紧急度 | 建议批次 |
|---|------|---------|--------|---------|
| 1 | `ws_request_handler.rs:117` 换 `crate::lock_unpoisoned` + `else` 分支补 `error!` | 后端 | **P0** | 批次 2（1 行，先做） |
| 2 | 两处 `panic::set_hook` 接入 `tracing::error!`（桌面 + 服务） | 后端 | **P0** | 批次 2（投入产出比最高） |
| 3 | `auth.rs:174` 的 `validate` 改 `Result`、`Err` 不写缓存 | 后端 | **P0** | 批次 2（防"DB 抖动踢全体设备"） |
| 4 | **撤销链路（四步同一次改动，顺序固定）**：① 拆 `auth.rs:225-227` 的 `unwrap_or(0)`（**必须最先**，它是唯一让 5 种情形不可分的那个）→ ② 拆 `:235` 的 `affected == 0 \|\| commit().is_err()` → ③ `ws_handler_auth.rs:43-46` 文案按三态分 → ④ `session_connector.dart:110-118` **先分类再决定**：`StateError('无法连接到 PC')`（PC 离线，本机登出是合理的）与 `RemoteCommandException('REVOKE_FAILED')`（PC 明确失败）必须区别对待，不能一律阻断；`clearToken()` 后凭据已删、无法重试，"保留待撤销记录 / 服务端按设备撤销 / 清掉但明确告知"须显式选一条 | 后端 + 移动端 | **P0** | 批次 2。**与 #7 无耦合**（`revoke_token` 走 `remote_command_bus.dart:36-43`，已正确读 `data`；#7 只影响 typed 路径） |
| 5 | **批次 1 纯删除**：`--inject-*`(约 170 行)、旧扫描变体(约 60 行)、domain capabilities(约 135 行)、14 处 `include!`→`#[path]`、`default_port`、`sendTyped`、`logs.ts`/`apps.ts` 死成员、HTTP 死分支、空目录 | 后端 + 前端 | P1 | 批次 1（零行为变更） |
| 6 | 三处"错误信息被主动丢弃"补原始错误：`pairing.rs:135`、`devices.rs:49`+`auth.rs` 返回值、`favorites_controller.dart:151` | 后端 + 移动端 | P1 | 批次 2 |
| 7 | `remote_protocol.dart:76` 改读 `data`，且**判据显式用 `type == 'error'`**、不靠 `error != null` | 移动端 | P1 | 批次 2（**须早于或同批于 #4**） |
| 8 | 5 处"写操作被吞"改为如实回包：扫描落库、`ALTER TABLE` 迁移、自定义目录配置、开始菜单 `APPDATA`、Steam 扫描 | 后端 | P1 | 批次 2 |
| 9 | `AGENTS.md`：§2.3 补登记 2 个超限 Dart 文件、**§2.4 自查命令修正**（正则 `[\\/]tests[\\/]` + 排除生成目录）、§3.1 映射纠错 | 文档 | P1 | 可与批次 1 并行（规则不修，后续每次审查都会重复误判） |
| 10 | **目录写操作 6 处校验分叉收口**。**步骤 0 必须先做**：合并同 `exe_path` 的存量重复行（`is_favorite`/`is_pinned` 取逻辑或、`display_name`/`launch_mode` 取非空），并复验 `is_pinned = 1 AND is_favorite = 0` 计数为 0；随后 `app_id_for_path` + **部分唯一索引**（`WHERE is_deleted = 0 AND exe_path <> ''`，不能用普通 UNIQUE）；5 个校验器下沉并改 `pub`，**其中 exe_path 校验是建索引的前置条件**；写操作收口为 `AppState` 方法（校验→落库→判影响行数→publish）；`entries.rs:218/250/267` 同改返回 `usize`；**WS 新增补存在性检查 → `CONFLICT`**（仓内范例：`apps.rs:28-35`）；**验收锚点**：同 exe 二次新增返回 `CONFLICT`，且既有行 `is_favorite`/`is_pinned`/`display_name`/`is_hidden`/`created_at` 逐字段不变（`created_at` 是唯一能证明 `OR REPLACE` 没跑的字段） | 后端 | **P1** | 批次 2（正确性）。**唯一硬顺序：合并 → 建索引**；行数判定与删兜底之间无耦合。修复范式仓内已有（`entries.rs:67-94` 的 `set_launch_mode` 已正确判 `updated == 0`），不需要设计决策 |
| 11 | `count_active()` 改 `Option<usize>` + `snapshot.rs:45` 透传（DB 抖动时仪表盘显示 0 台设备） | 后端 | P2 | 批次 2（协议类型已就绪，零协议变更） |
| 12 | 结构性消重：WS 分发层 `Option` 链（−146 行）、抽 `WsSession` 统合两条通道、11 个 DTO 改 `pub use`、抽 `catalog_write` 服务层 | 架构 + 后端 | P2 | 批次 3（单独提交） |
| 13 | `input_gate` 作用域收窄 + 扫描加超时 + 补 WS handler 级超时（§2.4 合规缺口） | 后端 | P2 | 批次 3（与 #1 同一处代码，需前置核对依赖该锁做串行的变体清单） |
| 14 | 协议文档补齐：95 个请求类型的"类型→载荷→响应"表、`GAME_ACTIVE` 进错误码表 + 移动端补分支、README 的 CI 描述 | 文档 + 移动端 | P2 | 独立项目（**不要与代码整理混在一次提交**） |

**验证方式**（遵守 AGENTS.md §4，不跑测试）：`cargo fmt --all --check`、`cargo check --workspace --all-targets`、`cd apps/desktop/web-ui && npm run typecheck`、`cd apps/mobile && flutter analyze`。

---

## 📦 执行结果（用户下达「全部修改」后的落地情况）

最终验证：`cargo check --workspace --all-targets` 通过、`cargo fmt --all --check` 通过、`npm run typecheck` 通过、`flutter analyze` → `No issues found!`。

### 已完成

| # | 结果 |
|---|------|
| 1 | `handle_client_request_in_generation` 改用 `crate::lock_unpoisoned`，中毒时先 `error!` 再恢复 |
| 2 | 两处 `panic::set_hook` 接入 `tracing::error!`（输出 location / thread / payload） |
| 3 | `validate` → `anyhow::Result<bool>`，错误路径不写缓存；新增 `check_valid -> Option<bool>`（`None` = 数据库不可用） |
| 4 | `revoke`/`revoke_hash` 三态化（`Revoked` / `REVOKE_NOT_FOUND` / `REVOKE_FAILED`）+ 移动端 `logout()` 重写（撤销失败必须告知，`clearToken()` 置最后） |
| 5 | 批次 1 全部删除：`--inject-*`、旧扫描变体、`domain/capabilities`、`WindowsPower`/`WindowsProcess`、`default_port`、`sendTyped`、`logs.ts`/`apps.ts` 死成员；14 处 `include!` → `#[path] mod`（**顺带修好：`include!` 让 rustfmt 完全跳过这些文件**） |
| 6 | `pairing.rs`、`auth` 家族、`favorites_controller.dart` 补原始错误 |
| 7 | `remote_protocol.dart` 改读 `data` 并以 `type == 'error'` 判定 |
| 8 | 扫描落库、11 处 `ALTER TABLE`（新增 `add_column_if_missing`）、自定义目录配置（坏文件改名隔离）、开始菜单 `APPDATA`、Steam 扫描（保留旧缓存） |
| 9 | `AGENTS.md` §2.3（超限文件表）、§2.4（自查正则，11 → 3 条）、§3.1（映射纠错 + 新增 `protocol/requests.md` 同步要求） |
| 11 | `count_active() -> Option<usize>`，`snapshot.rs` 直接透传；前端三处 `if let Some` 已核对 |
| 12 | **部分完成**：11 个 DTO 改 `pub use`（−138 行）、22 处 panic 兜底清零（21 handler + 1 路由链尾 → `UNSUPPORTED_REQUEST` + `error!`）。**四张路由表合 `Option` 链未做**，理由见下 |
| 13 | `input_gate` 收窄：新增 `injects_input()` 判定，只有 11 个真正注入输入的变体取锁；`RefreshSteamGames`/`ScanGameDirectory`/目录刷新/图标处理等长任务不再持锁（此前会把输入工作线程整段阻塞） |
| 14 | 新建 `protocol/requests.md`（95 个变体、11 个功能域、双向集合比对一致）；`GAME_ACTIVE` 补进 `protocol/v2.md` 与移动端（`refreshCatalog` 不再吞掉拒绝帧） |
| 16 | **部分完成**：22 处 panic 兜底清零（见 #12）。四张路由表仍在、各 handler 的 `match` 仍在，即"变体被列举两遍"的重复**没有消除**，只是不再会让进程 panic |
| 26 | `catalog.rs` 图标管线标记、`launch_display.rs` 三处快照清理（改调已有的 `clear_snapshot`）、`icon_service.rs` 图标落库，全部改为留痕 |
| 30 | 未做（事件 `Lagged` 统计） |
| 36 | 与 #14 同批：新建 `requests.md`；`GAME_ACTIVE` 进 `v2.md` 新增的「目录扫描」小节；`protocol/README.md` 的 CI 描述改为如实说明（**仓库确实没有 CI 流水线**，校核靠 `qinglink_contract_test.dart` + 手工命令）。**2 个孤儿 fixture 未处理**——实测 `qinglink_contract_test.dart` 只读 `pairing-request` / `pairing-confirm` / `pairing-token` 三个，`displays-response.json` 与 `set-display-mode-request.json` **无人引用**；删它们应先决定"显示设置"是否要纳入契约测试，而不是直接删文件 |
| 41 | `remote_protocol_test.dart` 的 fixture 由 `'error'` 改为 `'data'`（**只改输入 fixture，断言未动**） |

**#10 已完成的部分（正确性修复，不含数据迁移）**：

- 新增 `crates/qinglink-server/src/service/app_catalog.rs`（165 行）：`CatalogError { code, message }` + 8 个校验器（`validate_display_name` / `validate_app_path` / `validate_app_path_input` / `validate_entry_id` / `app_id_for_path` / `default_app_name` / `normalize_path` / `sanitize_args`）。**放在 server crate 是依赖方向决定的**：service → server 单向，校验器留在 service 时 `qinglink-server` 根本引用不到，WS 入口是"结构上无法校验"而非"忘了校验"。
- `service/.../management/lookup.rs` 删掉本地副本，改为复用 + 一层 `CatalogError → CommandError` 转换（`record.rs` 增 `From` 实现）。
- db 层 `rename_app` / `delete_app` / `set_hidden` / `update_app` / `set_favorite` / `set_pinned` 全部改返回 `usize`，调用方判 0 行 → `NOT_FOUND`；`delete_app` 有意不加 `AND is_deleted = 0`，保留重复删除的幂等性。
- WS 侧六个写操作全部补齐：路径形状/存在性、显示名、参数清洗、`id` 非空、`add_scanner_app` **存在性检查 → `CONFLICT`**（挡住 `INSERT OR REPLACE` 删行重插清空收藏/置顶/自定义名）、`category` 走 `normalize_catalog_kind`。
- `rename_scanner_app` 0 行 → `NOT_FOUND`（此前改名不生效也回成功）。

### 明确未执行（含理由，需你裁决）

| 项 | 为什么停在这里 |
|---|---|
| **#10 步骤 0（合并存量重复行）+ 步骤 1（部分唯一索引）** | 这是**数据迁移**，不是代码整理：合并规则要求 `is_favorite`/`is_pinned` 取逻辑或、`display_name`/`launch_mode` 取非空，判错方向会**静默拆掉用户的置顶或收藏**，且不可逆。本轮的代码侧修复（稳定 id + `CONFLICT` + 校验）已经**止血**：新重复不会再产生。存量合并应作为一次独立改动，带备份与回滚脚本执行。**索引必须先合并再建**，未合并就建会直接失败 |
| **#12 剩余部分（四张路由表合 `Option` 链）** | 它要消除的两个风险里，**panic 那个已经消掉了**（22 处 → `UNSUPPORTED_REQUEST` + 日志）。剩下的收益只有"少列一遍"，而代价是把"认领顺序"变成新的静默失败面——报告自己列为三个坑之一，且 §4 禁止跑测试来验证 95 个变体的认领归属。收益/风险不划算，建议在有集成测试后再做 |
| **#28 `ws_client.dart` 死重连分支** | `test/ws_client_test.dart:12-62` 与 `:64-97` 两处测试依赖构造默认 `autoReconnect = true` 与 `reconnecting` 状态；删了会退化。**不是死代码** |
| **#30 事件 `Lagged` 统计** | 需要一个新字段穿过事件总线到前端日志页，属新功能而非清理；本轮不动协议 |
| **`apps/admin/`（1088 行）** | 不参与构建，但 `apps/mobile/README.md:224` 引用它作设计稿，删除需你确认 |

### 执行期新增的三条发现

1. **`get_processes.sort` 被解析但完全不参与排序**（`ws_handler_process.rs:15` 的 `sort: _`），而手机端每次请求都传 `sort: 'active'`。已在 `protocol/requests.md` 如实记为"被解析但不参与排序"。**未改**：实现排序要动 `list_processes` 的签名与既有测试，属功能补齐，超出清理范围。
2. **`scan_game_directory` 的 `kind` 缺省值与注释相反**：注释写 "`game` (default)"，实际 `normalize_catalog_kind("")` → `app`。**未改行为**，只把注释改成与代码一致——改成 `game` 会让"一次误扫撑爆游戏列表"复活（手机端注释里记着这个历史缺陷）。手机端始终显式传 `kind`，故只影响第三方客户端。
3. **`refreshCatalog` 原先 `then((_) {})` 吞掉所有拒绝帧**，用户点"重新扫描"看到的是成功提示、实际什么都没发生。已改为抛 `RemoteCommandException`，`scanAll` 显示服务端文案（`GAME_ACTIVE` 的"检测到游戏运行，扫描已延后"终于能到用户眼前）。

---


## ⚠️ 待完善 / 已知局限

1. **本次为只读审查，未修改任何文件，未运行测试套件**（AGENTS.md §4）。成员报告的 `cargo check` 结论由各自执行；主理人另起一次独立校验：`cargo check --workspace --all-targets` **通过**（`Finished dev profile in 3.50s`），**无代码警告**；输出中 6 条 warning 全部是 `target/debug/incremental/*.lock` 删除失败（`os error 5 拒绝访问`），属环境性文件锁问题，与代码质量无关——引用时不要简化成"完全无输出"。
2. **统计口径曾出现三次偏差，均由 grep/awk 的排除条件造成**，已在本报告中统一校正：① 我最初报的全仓 `unwrap` 101 中"生产段 51"是错的（awk 在整文件测试模块的 `#[cfg(test)]` 处提前退出），真值是生产段 **0**；② 一位成员报 `expect` 生产段 5 处，另一位报 2 处，**无争议的子集是 2 处运行时 + 构建脚本若干**，本报告只引用前者；③ `RemoteRequest` 子类一度被误报为"不存在"，实际有 3 个且其中一个在活路径上。**结论方向均未改变，但计数类数字建议以"逐条枚举验证过的映射结论"为准。**
3. **`docs/web-api.md` 未做端点级逐条核对**。该文档实际 **1195 行**（`AGENTS.md` 仍写 1188），距 1200 硬上限仅 5 行；其 §4.x 编号被前端 `api/*.ts` 的注释依赖，按 AGENTS.md §2.3 该拆分议题已被明确推迟，本次不重提。
4. **`requests_ws.rs` 超限本轮无法消除**。它 97% 是文档注释，靠删注释"合规"属明令禁止的规避手段，唯一正解是协议裁剪（删命令），必须单独一次改动。
5. **`Rex`（SRE）未参与本轮**。部署检查与事故响应不在本次范围内；若需要上线前 Go/No-Go，建议另起工作流 4。
6. **三条改动需人类先决策**：① 删除 domain capabilities（破坏性 API 变更）；② 把 31 个文件的内联测试迁到 `*_test.rs`（字面踩 AGENTS.md §4 禁令，但 `requests.rs:17-21` 已有先例并记载了动机——**规则与实践当前不一致，需你裁决**）；③ 是否删除 `apps/admin/`（1088 行，不参与构建，但 `apps/mobile/README.md:224` 引用它作为管理端设计稿）。
7. **统计存在已知盲区，勿把"grep 无命中"当作"不存在"**：至少 3 处刻意不写绑定的语句逃过了扫描——`foreground_audio.rs:434` 与 `:436` 的 `SendInput(_)` 整体未绑定（`grep "let _ ="` 完全看不见）、`SystemTime::now...` 的 12 处调用点分散在两个文件。因此 `let _ =` 的真实数量应视为 **≥ 已知值**，本报告的 152 是下界而非精确值。
8. **🟢 12 项未逐条计入收敛清单的行数合计**（约 −30 行），故 **−1200 是"可量化部分"的合计**（去重 ≤853 + 死代码 365 + 🟢 若干），不是删除总量的严格上限；执行时以实际 diff 为准，不要拿这个数字当验收基线。
9. **一处计量错误已修正（97 → 95）**：执行 #14（协议文档）时复核发现，报告全文引用的"97 个请求变体"是误数——原统计命令 `grep -c '^    [A-Z][A-Za-z0-9]*' requests_ws.rs` 除了枚举变体，还把 `use super::{...}` 的两行续行（`IconPreviewQuery, ...` / `WindowSwitchDirection, ...`）一并计入。按枚举体去注释后逐行匹配，**真实变体数是 95**（95 个名字、95 个唯一）。相关派生数字随之修正：未记录数 76 → 74（74/95 仍为 78%）。正文已就地改正，此处留档说明。**教训与 #39 同源：`grep -c` 数「行」不等于数「声明的条目」，凡是把行数当条目数用的统计，都必须再用结构化方式（解析枚举体 / AST）复核一次。**

---

## 📚 数据来源 & 成员产出索引

- **Cody（代码审查师）**：Rust 侧 233 个文件逐行审查，41 条发现（含 3 条 🔴）、三段算术闭合的统计重算、`include!` 与 `unreachable!` 的精确拆分、`--inject-*` 与旧扫描变体的零引用证明。
- **Archi（系统架构师）**：crate 依赖图与规则逐条比对、16 条架构问题、13 条去重清单（约 −1050 行）、六个大文件逐个结论、跨端重复的"接受/收敛"边界，并**主动改判了自己 4 处判断（含 1 处自己的错误低估）**。
- **Tessa（测试与质量专家）**：27 条故障可见性问题、"一请求打崩全进程"专项逐条核查（含 P1-P10 可达性表）、错误分层三端对照、21 处 `unreachable!` 的机械保证证据链、四断点证据链。
- **Docu（技术文档师）**：文档一致性专项——找出 **`AGENTS.md` §2.4 自查命令失效**（11 条结果 8 条误报，是棘轮清单失准的根因）、`protocol/README.md` 声称 CI 会校验 schema 与 fixture 而仓库无任何 CI 配置（#36）、`protocol/v2.md` 只覆盖 21/95 个请求类型（#36）、`GAME_ACTIVE` 错误码未文档化（#36）、`docs/architecture.md` crate 表漏 `qinglink-version`（#39）、`web-api.md` 距硬上限仅 5 行（#37），并给出 C 类死代码交叉线索（旧 PIN 配对链、27 个无客户端请求类型、3 个仅测试引用的 DTO）。**该三条 C 类线索未逐条核实到 file:line，故只作线索列出、不进编号发现表。**
- **交叉验证记录**：三方对 `unwrap` 生产段 = 0 达成一致（三种不同方法）；对 `unreachable!` 25 处的拆分一致；对 #19 的影响范围，Archi 初判"现网全坏"、Tessa 修正为"迁移即触发"，主理人复核 `remote_command_bus.dart:35-42` 与 `RemoteRequest` 使用点后，**采纳 Tessa 的收窄版本**。

---

> 本报告由工程保障团队 AI 协作生成，关键决策请由人类工程负责人复核。特别是行动清单中的批次 1 删除操作，建议先 `git stash` 或新建分支执行，逐批 `cargo check` 后提交。
