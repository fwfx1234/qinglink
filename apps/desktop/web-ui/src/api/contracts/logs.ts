/**
 * 日志查询、统计与磁盘文件（docs/web-api.md §4.21 - §4.26）。
 *
 * 覆盖桌面端内存日志（内存页、级别统计、清空）与两个来源的磁盘日志文件，
 * 以及解析 QingLinkService 磁盘日志的服务端日志页。
 */

import type { Page } from './envelope'

/* ------------------------------------------------------------------ */
/* §4.21 - §4.24 日志                                                   */
/* ------------------------------------------------------------------ */

export type LogLevelName = 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'

/** 查询参数中的最低级别过滤（小写） */
export type LogLevelFilter = 'trace' | 'debug' | 'info' | 'warn' | 'error'

export interface LogItem {
  id: number
  timestamp_ms: number
  timestamp: number
  level: LogLevelName
  target: string
  message: string
  /** 后端在无值时不发送这三个键，而不是发送 null */
  thread_name?: string | null
  file?: string | null
  line?: number | null
}

export interface LogPage extends Page<LogItem> {}

export interface LogFileInfo {
  name: string
  size_bytes: number
  modified_secs: number
}

/** 只包含实际出现过的级别；缺失的级别按 0 处理 */
export type LogLevelCounts = Partial<Record<LogLevelName, number>>

export interface LogsStats {
  /** 内存中当前保留的条数；更早的记录在磁盘文本文件里 */
  total: number
  by_level: LogLevelCounts
  buffer_capacity: number
  buffer_len: number
  oldest_ms: number | null
  newest_ms: number | null
  files: LogFileInfo[]
}

export interface ClearLogsResult {
  cleared: number
}

/** §4.25 两个来源的磁盘日志文件 */
export interface LogFilesResult {
  desktop: LogFileInfo[]
  service: LogFileInfo[]
}

/** 日志来源：桌面端内存/文件，或 QingLinkService 的磁盘日志 */
export type LogSource = 'desktop' | 'service'

/** §4.26 服务端日志页（形状与 LogPage 一致，另带文件名） */
export interface ServiceLogPage extends Page<LogItem> {
  file: string
}
