/**
 * 按资源分组的接口函数。组件与 store 只调用这里，不直接 fetch。
 * 路径、请求体、响应体严格来自 docs/web-api.md §3 / §4。
 */

import { API_BASE, apiPath, request, requestBlob } from './client'
import type {
  AcceptedResult,
  AppAddRequest,
  AppAddResult,
  AppBatchRequest,
  AppBatchResult,
  AppIconCandidates,
  AppIconUpdate,
  AppLaunchResult,
  AppPage,
  AppsStats,
  CancelPairingResult,
  CatalogKind,
  ClearLogsResult,
  DeleteResult,
  DeviceItem,
  DevicePage,
  DiagnosticsReport,
  FavoriteResult,
  HiddenResult,
  InstallState,
  LogFilesResult,
  LogLevelFilter,
  LogPage,
  LogSource,
  LogsStats,
  OpenResult,
  OverviewData,
  PairingEnvelope,
  PauseResult,
  PinnedResult,
  RenameResult,
  RevokeResult,
  ScanResult,
  ServiceLogPage,
  ServiceStatus,
  SettingsData,
  SettingsPatch,
  UpdateView,
} from './types'

/* ---------------------------------- health / overview ---------------------------------- */

export const overviewApi = {
  get(): Promise<OverviewData> {
    return request<OverviewData>('/overview')
  },
}

/* -------------------------------------- settings --------------------------------------- */

export const settingsApi = {
  get(): Promise<SettingsData> {
    return request<SettingsData>('/settings')
  },
  update(patch: SettingsPatch): Promise<SettingsData> {
    return request<SettingsData>('/settings', { method: 'POST', body: patch })
  },
}

/* -------------------------------------- service ---------------------------------------- */

export type ServiceAction = 'install' | 'uninstall' | 'start' | 'stop' | 'restart'

export const serviceApi = {
  get(): Promise<ServiceStatus> {
    return request<ServiceStatus>('/service')
  },
  /** §4.6 暂停 / 恢复对外服务 */
  setPaused(paused: boolean): Promise<PauseResult> {
    return request<PauseResult>('/service/pause', { method: 'POST', body: { paused } })
  },
  /** §4.5.1 立刻重连并重新握手（结果走 service/status 事件） */
  reconnect(): Promise<AcceptedResult> {
    return request<AcceptedResult>('/service/reconnect', { method: 'POST' })
  },
  /** §4.7 提权操作：立即返回 accepted，真实结果走 install_state 事件 */
  action(action: ServiceAction): Promise<AcceptedResult> {
    return request<AcceptedResult>(`/service/${action}`, { method: 'POST' })
  },
  /** §4.8 安全输入服务安装体检结果 */
  installState(): Promise<InstallState> {
    return request<InstallState>('/service/install-state')
  },
}

/* ------------------------------------ diagnostics -------------------------------------- */

export const diagnosticsApi = {
  /** §4.27 一次性自检报告 */
  get(): Promise<DiagnosticsReport> {
    return request<DiagnosticsReport>('/diagnostics')
  },
  /** §4.28 纯文本诊断包下载地址 */
  bundleUrl(): string {
    return apiPath('/diagnostics/bundle')
  },
}

/* -------------------------------------- desktop ---------------------------------------- */

export const desktopApi = {
  /** §4.9 在默认浏览器打开本界面（只有真正交给 shell 后才成功） */
  open(): Promise<OpenResult> {
    return request<OpenResult>('/desktop/open', { method: 'POST' })
  },
  /** §4.9 返回 accepted 后进程才关闭，真实结果靠 system/shutdown 事件 */
  restart(): Promise<AcceptedResult> {
    return request<AcceptedResult>('/desktop/restart', { method: 'POST' })
  },
  exit(): Promise<AcceptedResult> {
    return request<AcceptedResult>('/desktop/exit', { method: 'POST' })
  },
}

/* ------------------------------------- file dialog ------------------------------------- */

/** §4.34 原生文件选择窗口：`image` 只要图片与程序，`executable` 只要可执行项。 */
export type FileDialogKind = 'image' | 'executable'

/** 目录由桌面端在 PC 本机弹出，用户浏览超过默认 30s 超时是常态。 */
const FILE_DIALOG_TIMEOUT_MS = 5 * 60_000

export interface FileDialogResult {
  /** 选中的绝对路径；用户取消时为 null。 */
  path: string | null
}

export const dialogApi = {
  /**
   * §4.34 在 PC 上弹出原生选择窗口，返回用户选中的绝对路径。
   *
   * 浏览器看不到本机文件系统（`<input type="file">` 只给不给路径），所以路径
   * 只能由桌面端代为选择。
   */
  pickFile(kind: FileDialogKind, startIn?: string | null): Promise<FileDialogResult> {
    return request<FileDialogResult>('/dialog/file', {
      method: 'POST',
      body: { kind, start_in: startIn ?? null },
      timeoutMs: FILE_DIALOG_TIMEOUT_MS,
    })
  },
}

/* -------------------------------------- devices ---------------------------------------- */

export const devicesApi = {
  list(offset: number, limit: number): Promise<DevicePage> {
    return request<DevicePage>('/devices', { query: { offset, limit } })
  },
  /** §4.11 id 必须原样回传（含 paired:/legacy: 前缀） */
  revoke(deviceId: DeviceItem['id']): Promise<RevokeResult> {
    return request<RevokeResult>('/devices/revoke', { method: 'POST', body: { device_id: deviceId } })
  },
}

/* -------------------------------------- pairing ---------------------------------------- */

export const pairingApi = {
  get(): Promise<PairingEnvelope> {
    return request<PairingEnvelope>('/pairing')
  },
  cancel(requestId: string): Promise<CancelPairingResult> {
    return request<CancelPairingResult>('/pairing/cancel', { method: 'POST', body: { request_id: requestId } })
  },
}

/* --------------------------------------- apps ------------------------------------------ */

export interface AppsListParams {
  kind: CatalogKind
  offset?: number
  limit?: number
  query?: string
  includeHidden?: boolean
  /** 只要「常用」 */
  favoriteOnly?: boolean
}

export const appsApi = {
  /** §4.14 */
  list(params: AppsListParams): Promise<AppPage> {
    return request<AppPage>('/apps', {
      query: {
        kind: params.kind,
        offset: params.offset ?? 0,
        limit: params.limit ?? 50,
        query: params.query ?? '',
        include_hidden: params.includeHidden ?? true,
        favorite: params.favoriteOnly ?? false,
      },
    })
  },
  /** §4.15 */
  stats(): Promise<AppsStats> {
    return request<AppsStats>('/apps/stats')
  },
  /** §4.16 图标：命中返回图片本身，未命中返回 JSON 错误信封 */
  iconUrl(id: string, version?: number | string | null): string {
    return apiPath('/apps/icon', { id, v: version ?? null })
  },
  /** §4.32 该条目可选的图标文件（exe / 快捷方式图标 / 同目录 PNG、ICO） */
  iconCandidates(id: string): Promise<AppIconCandidates> {
    return request<AppIconCandidates>('/apps/icon/candidates', { query: { id } })
  },
  /** §4.33 重新解析图标；icon_path 为空表示自动重新扫描 */
  refreshIcon(id: string, iconPath?: string | null): Promise<AppIconUpdate> {
    return request<AppIconUpdate>('/apps/icon/refresh', {
      method: 'POST',
      body: { id, icon_path: iconPath ?? null },
    })
  },
  /** §4.33 指定图标文件 */
  setIcon(id: string, iconPath: string): Promise<AppIconUpdate> {
    return request<AppIconUpdate>('/apps/icon/set', {
      method: 'POST',
      body: { id, icon_path: iconPath },
    })
  },
  /** §4.17 传空字符串表示恢复原始名 */
  rename(id: string, name: string): Promise<RenameResult> {
    return request<RenameResult>('/apps/rename', { method: 'POST', body: { id, name } })
  },
  /** §4.18 */
  setHidden(id: string, hidden: boolean): Promise<HiddenResult> {
    return request<HiddenResult>('/apps/hidden', { method: 'POST', body: { id, hidden } })
  },
  /** §4.18b 加入 / 移出「常用（我的库）」，与手机端共用一份数据 */
  setFavorite(id: string, favorite: boolean): Promise<FavoriteResult> {
    return request<FavoriteResult>('/apps/favorite', { method: 'POST', body: { id, favorite } })
  },
  /** §4.18c 置顶到手机首页（会自动加入常用） */
  setPinned(id: string, pinned: boolean): Promise<PinnedResult> {
    return request<PinnedResult>('/apps/pinned', { method: 'POST', body: { id, pinned } })
  },
  /** §4.19 软删除 */
  remove(id: string): Promise<DeleteResult> {
    return request<DeleteResult>('/apps/delete', { method: 'POST', body: { id } })
  },
  /** §4.20 扫描进行中重复调用返回 CONFLICT */
  scan(kind: CatalogKind): Promise<ScanResult> {
    return request<ScanResult>('/apps/scan', { method: 'POST', body: { kind } })
  },
  /** §4.29 新增自定义应用/游戏 */
  add(payload: AppAddRequest): Promise<AppAddResult> {
    return request<AppAddResult>('/apps/add', { method: 'POST', body: payload })
  },
  /** §4.30 在 PC 上启动该条目（服务端会转发到交互会话） */
  launch(id: string): Promise<AppLaunchResult> {
    return request<AppLaunchResult>('/apps/launch', { method: 'POST', body: { id } })
  },
  /** §4.31 批量隐藏/显示/删除/恢复原名 */
  batch(payload: AppBatchRequest): Promise<AppBatchResult> {
    return request<AppBatchResult>('/apps/batch', { method: 'POST', body: payload })
  },
}

/* --------------------------------------- logs ------------------------------------------ */

export interface LogsListParams {
  level?: LogLevelFilter | null
  target?: string
  search?: string
  since?: number | null
  until?: number | null
  limit?: number
  offset?: number
}

export const logsApi = {
  /** §4.21 items 按时间倒序（最新在前） */
  list(params: LogsListParams = {}): Promise<LogPage> {
    return request<LogPage>('/logs', {
      query: {
        level: params.level ?? null,
        target: params.target ?? '',
        search: params.search ?? '',
        since: params.since ?? null,
        until: params.until ?? null,
        limit: params.limit ?? 200,
        offset: params.offset ?? 0,
      },
    })
  },
  /** §4.22 */
  stats(): Promise<LogsStats> {
    return request<LogsStats>('/logs/stats')
  },
  /** §4.23 清空内存缓冲与数据库记录（不删除磁盘文件） */
  clear(): Promise<ClearLogsResult> {
    return request<ClearLogsResult>('/logs', { method: 'DELETE' })
  },
  /** §4.24 下载磁盘日志文件（source 默认 desktop） */
  download(file?: string, source: LogSource = 'desktop'): Promise<Blob> {
    return requestBlob('/logs/download', { file: file ?? null, source })
  },
  /** §4.25 两个来源的磁盘日志文件清单 */
  files(): Promise<LogFilesResult> {
    return request<LogFilesResult>('/logs/files')
  },
  /** §4.26 解析服务端磁盘日志（按需拉取，不是实时流） */
  service(params: LogsListParams & { file?: string } = {}): Promise<ServiceLogPage> {
    return request<ServiceLogPage>('/logs/service', {
      query: {
        file: params.file ?? null,
        level: params.level ?? null,
        target: params.target ?? '',
        search: params.search ?? '',
        since: params.since ?? null,
        until: params.until ?? null,
        limit: params.limit ?? 200,
        offset: params.offset ?? 0,
      },
    })
  },
}

export const updateApi = {
  /** §4.37 当前更新状态快照 */
  get(): Promise<UpdateView> {
    return request<UpdateView>('/update')
  },
  /** §4.38 后台检查更新 */
  check(): Promise<AcceptedResult> {
    return request<AcceptedResult>('/update/check', { method: 'POST' })
  },
  /** §4.39 后台下载已发现的版本 */
  download(): Promise<AcceptedResult> {
    return request<AcceptedResult>('/update/download', { method: 'POST' })
  },
  /** §4.40 触发安装（主程序随后退出） */
  install(): Promise<AcceptedResult> {
    return request<AcceptedResult>('/update/install', { method: 'POST' })
  },
}

export { API_BASE }
