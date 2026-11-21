# QingLink 上行请求类型索引

本表是 `WS /api/v2/events` 与 `WS /api/v2/input` 两个通道上**全部上行请求类型**的索引，
用于反查「某个 `type` 是否已在服务端登记」。语义细节不在这里重复，见 [`v2.md`](v2.md)。

## 数据来源与维护规则

本表由三处交叉得出，任一处改动都必须同批更新本文件：

| 来源 | 取什么 |
| --- | --- |
| `crates/qinglink-server/src/protocol/requests_ws.rs` | 变体定义与字段；`#[serde(tag = "type", rename_all = "snake_case")]` 决定线格式类型名 |
| `crates/qinglink-server/src/server/ws_request_handler.rs` | 四张路由表 `route_control_request` / `route_catalog_request` / `route_host_request` / `route_app_management_request` 决定归属分节 |
| `crates/qinglink-server/src/server/ws_handler_*.rs` | 响应类型与**实际可能返回**的错误码字面量 |

**计数口径。** 本表共 **95** 条，等于 `WsClientRequest` 枚举体变体数，也等于四张路由表去重后认领的变体数。
注意不要用「数缩进行」的方式统计 `requests_ws.rs`——会把文件顶部 `use super::{...}` 导入块的 2 行算进去而虚增为 97。
`input_ws_handler.rs`（`WS /api/v2/input`）复用同一个 `WsClientRequest`，没有第二套请求枚举。

**新增请求类型的登记流程。** 变体、四张路由表之一、对应 `ws_handler_*.rs` 分支、本表的一行，四者必须同批提交。
漏登记本表不会编译报错，只会让索引失真；漏登记路由表则会在运行时返回 `UNSUPPORTED_REQUEST`。

## 列说明

- **请求类型**：线格式 `type` 字符串（`snake_case`）。
- **载荷字段**：由变体字段与 `#[serde(...)]` 推导；`—` 表示无字段。`默认` 指服务端在缺省时采用的值。
- **响应 data**：成功分支的 `type`（括号内为 `WsServerResponse` 变体名）；`/ error` 表示存在错误分支，
  错误帧统一为 `{"type":"error","data":{"code","message"}}`。
- **错误码**：该 handler 源码里能逐字找到的错误码字面量。`—` 表示该请求在 handler 中没有错误分支。

**通配错误码**（下表不再逐行重复）。所有请求在进入路由表**之前**都会经过
`handle_client_request_in_generation`，因此任一行都可能额外返回：

| 错误码 | 触发条件 |
| --- | --- |
| `SERVICE_PAUSED` | 服务已暂停，或连接的输入代次已过期 |
| `REVOKED` | 当前会话令牌校验为失效 |
| `AUTH_UNAVAILABLE` | 令牌存储/数据库不可用（**不等于**令牌失效） |
| `UNSUPPORTED_REQUEST` | 路由表与 handler 匹配集合不一致（即新增变体漏配路由，属实现缺陷） |

## 分节与数量

| 节 | handler 模块 | 行数 |
| --- | --- | --- |
| 1 认证与令牌 | `ws_handler_auth.rs` | 3 |
| 2 系统、电源与解锁 | `ws_handler_system.rs` | 18 |
| 3 显示与远程输入 | `ws_handler_display_input.rs` | 10 |
| 4 进程 | `ws_handler_process.rs` | 6 |
| 5 应用搜索与启动 | `ws_handler_apps.rs` | 2 |
| 6 启动器目录 | `ws_handler_mobile_catalog.rs` | 9 |
| 7 扫描器应用（重命名 / 启动 / 目录扫描） | `ws_handler_scanner.rs` | 4 |
| 8 Steam | `ws_handler_steam.rs` | 4 |
| 9 自定义扫描目录 | `ws_handler_custom_dirs.rs` | 5 |
| 10 任务 | `ws_handler_tasks.rs` | 4 |
| 11 音量与媒体 | `ws_handler_volume_media.rs` | 12 |
| 12 文件浏览 | `ws_handler_files.rs` | 2 |
| 13 窗口 | `ws_handler_windows.rs` | 10 |
| 14 扫描器应用管理（增删改 / 隐藏 / 常用 / 置顶） | `ws_handler_scanner_management.rs` | 6 |
| **合计** |  | **95** |

## 1. 认证与令牌

`route_control_request` → `auth::handle`。`authenticate` 是唯一无需预先认证的指令。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `authenticate` | `method`(string，必填)、`token`(string，可选)、`pin`(string，可选)、`device_name`(string，可选) | `authenticated`(`Authenticated`) / `error` | `INVALID_TOKEN`、`PAIRING_V2_REQUIRED`、`INVALID_AUTH_METHOD`、`AUTH_UNAVAILABLE` | `method` 为 `pair` 恒拒绝；`pin` 仅为兼容老客户端保留。见 v2.md「认证」 |
| `ping` | — | `pong`(`Pong`) | — | 心跳，handler 无错误分支 |
| `revoke_token` | — | `revoked`(`Revoked`) / `error` | `REVOKE_FAILED`、`REVOKE_NOT_FOUND` | `REVOKE_NOT_FOUND` 表示令牌本就不存在，客户端按已撤销处理 |

## 2. 系统、电源与解锁

`route_control_request` → `system::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `get_system_status` | — | `system_status`(`SystemStatus`) | — | CPU / 内存 / 磁盘总览 |
| `get_mac_address` | — | `mac_address`(`MacAddress`) / `error` | `MAC_ERROR` | 读不到主网卡 MAC 时返回错误 |
| `get_subnet_prefix` | — | `subnet_prefix`(`SubnetPrefix`) / `error` | `SUBNET_ERROR` | 用于计算网络唤醒广播地址 |
| `get_wake_networks` | — | `wake_networks`(`WakeNetworks`) | — | 无错误分支，无可用网卡时返回空列表 |
| `shutdown` | `force`(bool，默认 `false`)、`delay_secs`(u64，默认 `0`) | `success`(`Success`) / `error` | `SHUTDOWN_FAILED` | `delay_secs` 为 `0` 表示立即执行 |
| `sleep` | `hibernate`(bool，默认 `false`) | `success`(`Success`) / `error` | `SLEEP_FAILED` | `true` 为休眠（状态写盘） |
| `restart` | `force`(bool，默认 `false`)、`delay_secs`(u64，默认 `0`) | `success`(`Success`) / `error` | `RESTART_FAILED` |  |
| `logoff` | — | `success`(`Success`) / `error` | `LOGOFF_FAILED` |  |
| `lock` | — | `success`(`Success`) / `error` | `LOCK_FAILED` |  |
| `get_keep_awake` | — | `keep_awake_status`(`KeepAwakeStatus`) | — | 读的是服务进程持有的开关。见 v2.md「保持唤醒」 |
| `set_keep_awake` | `enabled`(bool，必填) | `keep_awake_status`(`KeepAwakeStatus`) / `error` | `KEEP_AWAKE_FAILED` | 常驻线程不可用、平台调用失败或非 Windows 平台时返回该码。见 v2.md「保持唤醒」 |
| `get_unlock_status` | — | `unlock_status`(`UnlockStatus`) / `error` | `UNLOCK_STATUS_FAILED` | 非 Windows 平台恒返回此错误 |
| `auto_unlock` | `pin`(string，可选) | `success`(`Success`) / `error` | `INVALID_PIN_FORMAT`、`UAC_PROMPT_ACTIVE`、`SESSION_AGENT_NOT_READY`、`PIN_SENT_NOT_UNLOCKED`、`LOGIN_SURFACE_NOT_READY`、`AUTO_UNLOCK_FAILED` | 错误码取错误消息 `:` 之前的前缀；消息无 `:` 前缀时兜底 `AUTO_UNLOCK_FAILED`。`pin` 须为 4–32 位数字 |
| `get_lock_input_status` | — | `lock_input_status`(`LockInputStatus`) / `error` | `LOCK_INPUT_STATUS_FAILED` | 输入服务未运行时**不是**错误，返回 `available:false` |
| `submit_lock_pin` | `pin`(string，必填) | `lock_input_result`(`LockInputResult`) / `error` | `INVALID_PIN_FORMAT`、`SECURE_INPUT_FAILED`、`SECURE_INPUT_NOT_ACCEPTED`、`LOCK_INPUT_SERVICE_UNAVAILABLE`、`LOCK_INPUT_INVALID_RESPONSE`、`LOGIN_SURFACE_NOT_READY`、`LOCK_INPUT_FAILED`，以及输入服务透传的 `code` | 错误码同样取 `:` 前前缀；`pin` 须为 4–32 位数字。成功时 `data.code` 常见 `SECURE_INPUT_ACCEPTED` |
| `get_secure_desktop_status` | — | `secure_desktop_status`(`SecureDesktopStatus`) / `error` | `SECURE_DESKTOP_STATUS_FAILED` | 返回当前桌面、是否锁屏、`uac_detected` 等 |
| `secure_attention` | — | `success`(`Success`) / `error` | `SECURE_ATTENTION_RATE_LIMITED`、`SECURE_ATTENTION_FAILED` | 3 秒内重复请求被限流。见 v2.md 安全桌面段落 |
| `send_secure_ascii` | `text`(string，必填) | `success`(`Success`) / `error` | `INVALID_SECURE_TEXT`、`SECURE_TEXT_FAILED` | 仅接受 1–256 个可打印 ASCII（`0x20`–`0x7e`），从不使用剪贴板 |

## 3. 显示与远程输入

`route_control_request` → `display_input::handle`。本节请求中除 `get_displays` / `set_display_mode` 外都持输入闸门锁。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `get_displays` | — | `displays`(`Displays`) / `error` | `DISPLAY_LIST_FAILED` | 见 v2.md「显示设置」 |
| `set_display_mode` | `display_id`(string)、`width`(u32)、`height`(u32)、`refresh_millihz`(u32) | `success`(`Success`) / `error` | `DISPLAY_SET_FAILED` | 四字段扁平传递，`refresh_millihz` 必须与候选值逐位相同。见 v2.md「显示设置」 |
| `mouse_move` | `dx`(i32)、`dy`(i32) | `success`(`Success`) / `error` | `MOUSE_INPUT_FAILED`、`SECURE_DESKTOP_INPUT_BLOCKED`、`UAC_AGENT_TIMEOUT`、`UAC_DESKTOP_SWITCH_FAILED` | 位移先按 `MAX_MOUSE_MOVE_DELTA`=500 裁剪，不报错 |
| `mouse_move_absolute` | `x`(u32)、`y`(u32) | `success`(`Success`) / `error` | `MOUSE_MOVE_ABSOLUTE_FAILED` | 安全桌面上应改用此命令定位按钮 |
| `mouse_button` | `button`(string) | `success`(`Success`) / `error` | `MOUSE_INPUT_FAILED`、`SECURE_DESKTOP_INPUT_BLOCKED`、`UAC_AGENT_TIMEOUT`、`UAC_DESKTOP_SWITCH_FAILED` | 在当前位置单击一次 |
| `mouse_button_state` | `button`(string)、`pressed`(bool) | `success`(`Success`) / `error` | `MOUSE_INPUT_FAILED`、`SECURE_DESKTOP_INPUT_BLOCKED`、`UAC_AGENT_TIMEOUT`、`UAC_DESKTOP_SWITCH_FAILED` | 用于拖拽；仅在平台调用成功后同步 `held_input` |
| `mouse_wheel` | `delta`(i32) | `success`(`Success`) / `error` | `MOUSE_INPUT_FAILED`、`SECURE_DESKTOP_INPUT_BLOCKED`、`UAC_AGENT_TIMEOUT`、`UAC_DESKTOP_SWITCH_FAILED` | 正值向上滚动 |
| `send_text` | `text`(string，必填) | `success`(`Success`) / `error` | `TEXT_TOO_LARGE`、`TEXT_SEND_FAILED` | 超过 64 KiB 在触达平台前拒绝；安全桌面拒绝 Unicode 文本 |
| `key_press` | `key`(string，必填) | `success`(`Success`) / `error` | `INVALID_INPUT`、`KEY_PRESS_FAILED` | 仅接受 `is_supported_remote_key` 白名单内的键名，其余返回 `INVALID_INPUT` |
| `key_event` | `usage`(u8)、`modifiers`(u8，默认 `0`)、`pressed`(bool) | `success`(`Success`) / `error` | `INVALID_INPUT`、`KEY_EVENT_FAILED`、`UAC_AGENT_TIMEOUT`、`UAC_DESKTOP_SWITCH_FAILED` | `usage` 有效范围 `0x04`–`0x73`，越界返回 `INVALID_INPUT` |

## 4. 进程

`route_catalog_request` → `process::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `get_processes` | `search`(string，可选)、`page`(usize，默认 `0`)、`page_size`(usize，默认 `50`)、`filter`(string，默认 `user`)、`sort`(string，默认 `active`) | `process_list`(`ProcessList`) | — | handler 无错误分支；`page_size` 服务端强制 `min(50)`；`sort` 被解析但**不参与**排序 |
| `get_foreground` | — | `foreground`(`Foreground`) / `error` | `FOREGROUND_ERROR` | 附带 `focus_text`，取值口径见 v2.md「前台窗口与焦点文本」 |
| `kill_process` | `pid`(u32) | `success`(`Success`) / `error` | `KILL_FAILED` |  |
| `suspend_process` | `pid`(u32) | `success`(`Success`) / `error` | `SUSPEND_FAILED` |  |
| `resume_process` | `pid`(u32) | `success`(`Success`) / `error` | `RESUME_FAILED` |  |
| `get_process_icon` | `pid`(u32) | `process_icon`(`ProcessIcon`) | — | 无错误分支；无自带图标时返回 `source:"none"`，不是错误。见 v2.md「进程图标」 |

## 5. 应用搜索与启动

`route_catalog_request` → `apps::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `search_apps` | `query`(string，必填) | `search_apps`(`SearchApps`) | — | 无错误分支 |
| `launch_app` | `path`(string，必填)、`args`(string[]，默认空) | `success`(`Success`) / `error` | `APP_NOT_FOUND`、`LAUNCH_FAILED` | `path` 必须是已存在的文件，否则不触达平台直接返回 `APP_NOT_FOUND` |

## 6. 启动器目录

`route_catalog_request` → `mobile_catalog::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `list_launcher_items` | `kind`(string，必填)、`query`(string，可选)、`page`(usize，默认 `0`)、`page_size`(usize，默认 `50`) | `launcher_items`(`LauncherItems`) | — | 无错误分支；`kind` 取 `game`/`app`/`favorite`/`steam`/`software`/`other`，其他取值不筛选 |
| `get_recent_launcher_items` | `kind`(string，必填) | `recent_launcher_items`(`RecentLauncherItems`) | — | 无错误分支；服务端最多取 12 条最近记录 |
| `get_launcher_icon` | `id`(string，必填) | `launcher_icon`(`LauncherIcon`) | — | 无错误分支；未命中时 `icon_base64` 省略，客户端保留占位图 |
| `refresh_scanner_app_icon` | `id`(string，必填)、`icon_path`(string，可选) | `icon_update`(`IconUpdate`) / `error` | `ICON_UPDATE_FAILED` | 省略 `icon_path` 时重跑自动解析链 |
| `list_scanner_app_icons` | `id`(string，必填) | `icon_candidates`(`IconCandidates`) / `error` | `NOT_FOUND` | 条目不存在时返回 `NOT_FOUND` |
| `set_scanner_app_icon` | `id`(string，必填)、`icon_path`(string，可选) | `icon_update`(`IconUpdate`) / `error` | `ICON_UPDATE_FAILED` | `icon_path` 为 `null` / 省略表示清除覆盖并恢复自动解析 |
| `get_icon_preview` | `path`(string，必填) | `icon_preview`(`IconPreview`) | — | 无错误分支；类型不支持、文件不存在、大小为 0 或超过 16 MiB 时 `icon_base64` 省略，不是错误。见 v2.md「图标预览」 |
| `list_blocked_scanner_apps` | — | `blocked_launcher_items`(`BlockedLauncherItems`) / `error` | `LIST_FAILED` | 整表返回，按名称排序 |
| `refresh_launcher_catalog` | `kind`(string，默认空) | `success`(`Success`) / `error` | `GAME_ACTIVE`、`SCAN_START_FAILED` | 异步扫描，`success` 只表示线程已启动；`kind` 只认 `game`，其余（含缺省空串）一律按 `app`；游戏运行中返回 `GAME_ACTIVE` |

## 7. 扫描器应用（重命名 / 启动 / 目录扫描）

`route_catalog_request` → `scanner::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `rename_scanner_app` | `id`(string)、`original_name`(string)、`new_name`(string) | `success`(`Success`) / `error` | `RENAME_FAILED` | `original_name` 用于同步内存目录 |
| `launch_scanner_app` | `id`(string，必填) | `success`(`Success`) / `error` | `LAUNCH_FAILED`、`APP_NOT_FOUND`、`NOT_FOUND` | `steam:bigpicture` 与 `steam:<app_id>` 走 Steam 分支；条目已配置 `launch_mode` 时经显示切换路径启动 |
| `scan_game_directory` | `path`(string，必填)、`kind`(string，默认空) | `game_scan_candidates`(`GameScanCandidates`) / `error` | `GAME_DIR_SCAN_FAILED` | `kind` 只认 `game`，其余（含缺省空串）一律按 `app`；扫描阶段不提取图标 |
| `add_scanned_games` | `path`(string)、`exe_paths`(string[])、`names`(string[]，可选)、`kind`(string，默认空) | `scanner_apps`(`ScannerApps`) / `error` | `NO_ITEMS_SELECTED`、`ADD_GAMES_FAILED` | 只接受**确实存在**且位于 `path` 之内的 exe，其余被静默过滤；过滤后为空才返回 `NO_ITEMS_SELECTED`；`kind` 为 `app` 时归入 `Application`，否则 `Game` |

## 8. Steam

`route_catalog_request` → `steam::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `get_steam_games` | — | `steam_games`(`SteamGames`) | — | 无错误分支；未安装 Steam 时 `steam_installed:false` |
| `refresh_steam_games` | — | `steam_refresh`(`SteamRefresh`) / `error` | `GAME_ACTIVE`、`STEAM_SAVE_FAILED` | 同步扫描，游戏运行中直接返回 `GAME_ACTIVE`；成功时 `data.scanned` 为本次数量 |
| `install_steam_game` | `app_id`(u32) | `success`(`Success`) / `error` | `STEAM_INSTALL_FAILED` | 只触发安装，不等待完成 |
| `launch_steam_game` | `app_id`(u32) | `success`(`Success`) / `error` | `LAUNCH_FAILED` |  |

## 9. 自定义扫描目录

`route_host_request` → `custom_dirs::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `get_custom_dirs` | — | `custom_dirs`(`CustomDirs`) | — | 无错误分支 |
| `add_custom_dir` | `path`(string)、`name`(string，可选)、`enabled`(bool，默认 `true`)、`max_depth`(u32，默认 `0`)、`extensions`(string[]，可选)、`recursive`(bool，默认 `true`) | `custom_dir`(`CustomDir`) / `error` | `ADD_FAILED` | `max_depth` 为 `0` 表示不限深度 |
| `update_custom_dir` | `id`(string) 加 `path`/`name`/`enabled`/`max_depth`/`extensions`/`recursive`（均可选） | `custom_dir`(`CustomDir`) / `error` | `UPDATE_FAILED` | 未提供的字段保持原值 |
| `delete_custom_dir` | `id`(string，必填) | `success`(`Success`) / `error` | `DELETE_FAILED` |  |
| `validate_dir` | `path`(string，必填) | `dir_validation`(`DirValidation`) | — | 无错误分支；目录不可用通过 `valid`/`exists`/`is_dir`/`readable`/`error` 字段表达 |

## 10. 任务

`route_host_request` → `tasks::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `get_tasks` | `search`(string，可选)、`page`(usize，默认 `0`)、`page_size`(usize，默认 `50`)、`filter`(string，默认 `user`) | `tasks`(`Tasks`) | — | 无错误分支；载荷与 `get_processes` 共用 `ProcessListQuery`，故**没有** `sort` 字段；`page_size` 由 `ProcessManager::list_processes` 统一 `clamp(1, 200)` |
| `get_tasks_stats` | — | `system_status`(`SystemStatus`) | — | 无错误分支；复用系统总览载荷，不是独立类型 |
| `kill_task` | `pid`(u32) | `success`(`Success`) / `error` | `KILL_FAILED` | 与 `kill_process` 等价 |
| `set_task_priority` | `pid`(u32)、`priority`(string) | `success`(`Success`) / `error` | `PRIORITY_FAILED` | 平台层接受 `idle`、`below_normal`/`low`、`normal`、`above_normal`/`high`、`realtime`（不区分大小写）；**其他取值不报错**，静默按 `normal` 处理 |

## 11. 音量与媒体

`route_host_request` → `volume_media::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `get_volume` | — | `volume_status`(`VolumeStatus`) / `error` | `VOLUME_FAILED` | 当前音量与静音状态 |
| `get_audio_outputs` | — | `audio_outputs`(`AudioOutputs`) / `error` | `AUDIO_OUTPUTS_FAILED` |  |
| `set_audio_output` | `device_id`(string，必填) | `success`(`Success`) / `error` | `AUDIO_OUTPUT_FAILED` |  |
| `set_volume` | `percent`(u8) | `success`(`Success`) / `error` | `VOLUME_FAILED` | 取值 0–100；超过 100 由平台层按 `percent.min(100)` 收敛，**不报错** |
| `set_muted` | `muted`(bool) | `success`(`Success`) / `error` | `VOLUME_FAILED` |  |
| `volume_up` | — | `success`(`Success`) / `error` | `VOLUME_FAILED` | 调高一级 |
| `volume_down` | — | `success`(`Success`) / `error` | `VOLUME_FAILED` | 调低一级 |
| `volume_mute` | — | `success`(`Success`) / `error` | `VOLUME_FAILED` | 切换静音，不设置具体状态 |
| `media_play_pause` | — | `success`(`Success`) / `error` | `MEDIA_FAILED` |  |
| `media_stop` | — | `success`(`Success`) / `error` | `MEDIA_FAILED` |  |
| `media_next` | — | `success`(`Success`) / `error` | `MEDIA_FAILED` |  |
| `media_prev` | — | `success`(`Success`) / `error` | `MEDIA_FAILED` |  |

## 12. 文件浏览

`route_host_request` → `files::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `browse_files` | `path`(string，可选) | `directory_listing`(`DirectoryListing`) / `error` | `BROWSE_FAILED` | `path` 省略或为空串时返回驱动器列表，`current_path` 为空串 |
| `get_quick_access` | — | `quick_access`(`QuickAccess`) | — | 无错误分支；`data` 是 `[名称, 路径]` 二元组数组 |

## 13. 窗口

`route_host_request` → `windows::handle`。成功的窗口动作会失效服务端窗口缓存。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `list_windows` | `force_refresh`(bool，默认 `false`) | `window_list`(`WindowList`) | — | 无错误分支 |
| `focus_window` | `hwnd`(usize) | `success`(`Success`) / `error` | `FOCUS_FAILED` |  |
| `switch_window` | `direction`(`WindowSwitchDirection`：`next`/`previous`) | `success`(`Success`) / `error` | `SWITCH_WINDOW_FAILED` | 按方向相对当前窗口切换 |
| `minimize_window` | `hwnd`(usize) | `success`(`Success`) / `error` | `MINIMIZE_FAILED` |  |
| `maximize_window` | `hwnd`(usize) | `success`(`Success`) / `error` | `MAXIMIZE_FAILED` |  |
| `restore_window` | `hwnd`(usize) | `success`(`Success`) / `error` | `RESTORE_FAILED` |  |
| `close_window` | `hwnd`(usize) | `success`(`Success`) / `error` | `CLOSE_FAILED` |  |
| `minimize_foreground` | — | `success`(`Success`) / `error` | `MINIMIZE_FAILED` | 没有前台窗口时同样返回该错误 |
| `maximize_foreground` | — | `success`(`Success`) / `error` | `MAXIMIZE_FAILED` | 同上 |
| `close_foreground` | — | `success`(`Success`) / `error` | `CLOSE_FAILED` | 同上 |

## 14. 扫描器应用管理

`route_app_management_request` → `scanner_management::handle`。

| 请求类型 | 载荷字段 | 响应 data | 错误码 | 备注 |
| --- | --- | --- | --- | --- |
| `add_scanner_app` | `name`(string)、`exe_path`(string)、`category`(string)、`icon_base64`(string，可选)、`source`(string，可选)、`args`(string[]，默认空) | `scanner_app_updated`(`ScannerAppUpdated`) / `error` | `INVALID_ICON`、`ADD_FAILED` | 省略 `icon_base64` 时由服务端解析；`source` 缺省记为 `User`；客户端上传的位图会规范化为 PNG，无法解码时返回 `INVALID_ICON` |
| `update_scanner_app` | `id`(string)、`name`(string)、`exe_path`(string)、`icon_base64`(string，可选)、`args`(string[]，可选)、`launch_mode`(object，可选)、`clear_launch_mode`(bool，默认 `false`) | `scanner_app_updated`(`ScannerAppUpdated`) / `error` | `INVALID_DISPLAY_MODE`、`INVALID_ICON`、`UPDATE_FAILED` | `name` 与 `exe_path` 必填（无 `default`）；`launch_mode` 须是主显示器候选中的宽高与刷新率组合，且只允许配置到分类为 `game` 的条目。见 v2.md「游戏启动显示模式」 |
| `delete_scanner_app` | `id`(string，必填) | `scanner_app_deleted`(`ScannerAppDeleted`) / `error` | `DELETE_FAILED` |  |
| `set_scanner_app_hidden` | `id`(string)、`hidden`(bool) | `scanner_app_updated`(`ScannerAppUpdated`) / `error` | `HIDE_FAILED` | 屏蔽后条目仍留在 PC，可经 `list_blocked_scanner_apps` 列出 |
| `set_scanner_app_favorite` | `id`(string)、`favorite`(bool) | `scanner_app_updated`(`ScannerAppUpdated`) / `error` | `FAVORITE_FAILED` | 常用状态存在服务端，跟随 PC 而非手机本地 |
| `set_scanner_app_pinned` | `id`(string)、`pinned`(bool) | `scanner_app_updated`(`ScannerAppUpdated`) / `error` | `PIN_FAILED` | 置顶会同时把条目置为常用（数据库 `set_pinned` 内联动 `is_favorite`）；取消置顶不清除常用 |

## 未决事项

无。95 行的响应类型与错误码均可在上表所列三处源码中逐字核对。
`auto_unlock` 与 `submit_lock_pin` 的错误码取自错误消息 `:` 前的前缀，前缀集合已按平台实现逐一列出；
`submit_lock_pin` 另有输入服务运行时透传的 `code`，其取值由外部服务决定，本表不枚举。
