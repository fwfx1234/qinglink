/**
 * 一次性自检报告（docs/web-api.md §4.27 / §4.28）。
 *
 * 报告内嵌本模块之外的服务状态、安装体检与托盘结论，因此依赖 ./service.ts。
 */

import type { InstallState, ServiceStatus, TrayReport } from './service'

/* ------------------------------------------------------------------ */
/* §4.27 / §4.28 诊断                                                   */
/* ------------------------------------------------------------------ */

export interface DiagnosticsPipeProbe {
  /** 管道是否能完成一次 v2 management 往返 */
  reachable: boolean
  /** 服务应答的协议版本；无应答时为 null */
  version: number | null
  code: string | null
  message: string | null
  /** 失败时的完整因果链 */
  error: string | null
}

export interface DiagnosticsReport {
  version: string
  pid: number
  uptime_secs: number
  data_dir: string
  log_dir: string
  service_log_dir: string
  integrity: string
  shell_integrity: string
  web_interface: {
    assets_embedded: number
    index_bytes: number
    placeholder: boolean
  }
  tray: TrayReport | null
  install_state: InstallState
  service: ServiceStatus
  pipe_probe: DiagnosticsPipeProbe
}
