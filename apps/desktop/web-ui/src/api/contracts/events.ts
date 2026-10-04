/**
 * WebSocket 事件契约（docs/web-api.md §5）。
 *
 * 事件数据里内嵌的其他契约类型（服务状态、安装体检、设置、配对请求、目录种类、
 * 日志条目）一律从各自模块引用，本模块只定义事件信封与各事件的数据形状。
 */

import type { CatalogKind } from './catalog'
import type { LogItem } from './logs'
import type { PairingRequest } from './pairing'
import type { InstallState, ServiceStatus, SettingsData } from './service'

/* ------------------------------------------------------------------ */
/* §5 WebSocket 事件                                                    */
/* ------------------------------------------------------------------ */

export type EventChannel = 'system' | 'service' | 'devices' | 'pairing' | 'apps' | 'logs' | 'notice'

export interface EventEnvelope<K extends EventChannel, E extends string, D> {
  channel: K
  event: E
  data: D
}

export interface HelloData {
  server_time_unix: number
  channels: EventChannel[]
}

export interface PongData {
  server_time_unix: number
}

export interface ShutdownData {
  reason: 'restart' | 'exit'
}

export interface DevicesChangedData {
  total: number
}

export interface PairingChangedData {
  pairing: PairingRequest | null
}

export type CatalogChangeReason = 'rename' | 'hidden' | 'delete' | 'scan_finished' | 'resync' | 'add' | 'icon' | 'favorite'

export interface CatalogChangedData {
  reason: CatalogChangeReason
}

export interface ScanStartedData {
  kind: CatalogKind
}

export interface LogEntriesData {
  items: LogItem[]
}

export interface LogDroppedData {
  count: number
}

export interface LogsClearedData {
  cleared: number
}

export type ToastLevel = 'info' | 'warn' | 'error'

export interface ToastEventData {
  level: ToastLevel
  message: string
}

export type SystemHelloEvent = EventEnvelope<'system', 'hello', HelloData>
export type SystemPongEvent = EventEnvelope<'system', 'pong', PongData>
export type SystemShutdownEvent = EventEnvelope<'system', 'shutdown', ShutdownData>
export type SystemSettingsEvent = EventEnvelope<'system', 'settings', SettingsData>
export type ServiceStatusEvent = EventEnvelope<'service', 'status', ServiceStatus>
export type ServiceInstallStateEvent = EventEnvelope<'service', 'install_state', InstallState>
export type DevicesChangedEvent = EventEnvelope<'devices', 'changed', DevicesChangedData>
export type PairingPendingEvent = EventEnvelope<'pairing', 'pending', PairingChangedData>
export type AppsCatalogChangedEvent = EventEnvelope<'apps', 'catalog_changed', CatalogChangedData>
export type AppsScanStartedEvent = EventEnvelope<'apps', 'scan_started', ScanStartedData>
export type LogsEntriesEvent = EventEnvelope<'logs', 'entries', LogEntriesData>
export type LogsDroppedEvent = EventEnvelope<'logs', 'dropped', LogDroppedData>
export type LogsClearedEvent = EventEnvelope<'logs', 'cleared', LogsClearedData>
export type NoticeToastEvent = EventEnvelope<'notice', 'toast', ToastEventData>

export type ServerEvent =
  | SystemHelloEvent
  | SystemPongEvent
  | SystemShutdownEvent
  | SystemSettingsEvent
  | ServiceStatusEvent
  | ServiceInstallStateEvent
  | DevicesChangedEvent
  | PairingPendingEvent
  | AppsCatalogChangedEvent
  | AppsScanStartedEvent
  | LogsEntriesEvent
  | LogsDroppedEvent
  | LogsClearedEvent
  | NoticeToastEvent

export type ClientCommand =
  | { action: 'subscribe'; channels: EventChannel[] }
  | { action: 'unsubscribe'; channels: EventChannel[] }
  | { action: 'ping' }
