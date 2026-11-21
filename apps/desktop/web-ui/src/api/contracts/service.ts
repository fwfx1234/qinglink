/**
 * 服务状态与能力、设置、桌面端动作结果（docs/web-api.md §4.1 - §4.9）。
 *
 * 覆盖 overview / health / service / settings 四个资源；overview 内嵌的配对请求
 * 类型在 ./pairing.ts（本模块只引用，不重复定义），诊断报告在 ./diagnostics.ts。
 */

import type { PairingRequest } from './pairing'

/* ------------------------------------------------------------------ */
/* §4.1 health / §4.2 overview                                          */
/* ------------------------------------------------------------------ */

export interface ServiceCapabilities {
  management: boolean
  pairing: boolean
  pause: boolean
  devices: boolean
  catalog: boolean
  catalog_management: boolean
  /** AddApp */
  app_add: boolean
  /** LaunchApp */
  app_launch: boolean
  /** RefreshAppIcon / ListAppIcons */
  app_icon_manage: boolean
}

/** 桌面端↔服务链路状态（§4.5 的 state 字段） */
export type ServiceLinkState =
  | 'connecting'
  | 'connected'
  | 'protocol_mismatch'
  | 'unauthorized'
  | 'unreachable'

export interface ServiceStatus {
  /** state === 'connected' 的兼容别名 */
  connected: boolean
  state: ServiceLinkState
  /** 桌面端使用的协议版本 */
  expected_version: number
  /** 服务实际应答的协议版本；从未应答过时为 null */
  service_version: number | null
  /** 服务端产品版本号（如 `0.1.0+13`）；旧服务不返回时为 null */
  service_app_version: string | null
  paused: boolean
  network_ready: boolean
  /** 链路可用时为 null */
  address: string | null
  last_error: string | null
  /** 链路可用时为 null */
  capabilities: ServiceCapabilities | null
}

export interface SettingsData {
  autostart: boolean
  keep_awake: boolean
  debug_log: boolean
}

export interface InstallState {
  package_available: boolean
  service_installed: boolean
  service_running: boolean
  service_binary_matches: boolean
  /** 服务回环健康协议是否应答；与 available 分开以便定位失败环节 */
  network_ready: boolean
  /** 已安装且运行，但管理链路不可用（协议过旧/桌面端未授权）时为 false；未知为 null */
  service_protocol_ok?: boolean | null
  available: boolean
  /** 提权操作执行中（§4.8；overview 内嵌版本可能省略） */
  busy?: boolean
  /** 上一次提权操作的结论文本 */
  result?: string | null
  detail: string
}

export interface OverviewStats {
  apps: number
  hidden_apps: number
  devices: number
  logs: number
  errors: number
}

/** overview.health 是 /health 的子集，另含 status / web_port。 */
export interface OverviewHealth {
  status?: string
  version: string
  pid: number
  uptime_secs: number
  web_port?: number
  log_level: string
}

/** 托盘图标注册结果（overview.tray）。 */
export interface TrayReport {
  visible: boolean
  /** Shell_NotifyIconGetRect 是否确认图标已在通知区域（折叠区也算已注册） */
  verified?: boolean
  /** GetRect 返回的真实坐标；null 表示图标在折叠区或未注册 */
  rect: { x: number; y: number; width: number; height: number } | null
  icon_size: number
  /** Shell_NotifyIcon 失败时的 GetLastError 值 */
  error_code?: number | null
  /** 本进程令牌完整性：与任务栏不一致时 shell 一定拒绝图标 */
  integrity: string
  /** 通知区域所属 shell 进程的完整性（通常为 medium） */
  shell_integrity: string
  /** 已尝试的注册次数 */
  attempts?: number
  /** 非空表示重试无法修复（通常是完整性级别不匹配），界面应直接展示原因 */
  blocked_reason?: string | null
  detail: string
}

export interface OverviewData {
  health: OverviewHealth
  service: ServiceStatus
  settings: SettingsData
  install_state: InstallState
  tray: TrayReport | null
  stats: OverviewStats
  pairing: PairingRequest | null
}

/* ------------------------------------------------------------------ */
/* §4.3 / §4.4 settings 写操作                                          */
/* ------------------------------------------------------------------ */

export type SettingsPatch = Partial<SettingsData>

/* ------------------------------------------------------------------ */
/* §4.6 / §4.7 / §4.9 服务与桌面端动作                                   */
/* ------------------------------------------------------------------ */

/** §4.6 `paused` 回显的是请求值，权威值走 service/status 事件。 */
export interface PauseResult {
  accepted?: boolean
  paused: boolean
}

export interface AcceptedResult {
  accepted: boolean
}

export interface OpenResult {
  url: string
}
