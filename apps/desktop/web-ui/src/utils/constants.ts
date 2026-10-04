import type { LogLevelName } from '@/api/types'

/** 日志级别元数据：色调用语义色，界面上始终是「色块 + 文字」共同表达 */
export type Tone = 'neutral' | 'accent' | 'success' | 'warning' | 'error'

export interface LogLevelMeta {
  label: LogLevelName
  tone: Tone
  /** 数值越大越严重，用于「最低级别」过滤说明 */
  severity: number
  description: string
}

export const LOG_LEVELS: LogLevelMeta[] = [
  { label: 'TRACE', tone: 'neutral', severity: 0, description: '追踪' },
  { label: 'DEBUG', tone: 'neutral', severity: 1, description: '调试' },
  { label: 'INFO', tone: 'accent', severity: 2, description: '信息' },
  { label: 'WARN', tone: 'warning', severity: 3, description: '警告' },
  { label: 'ERROR', tone: 'error', severity: 4, description: '错误' },
]

export const LOG_LEVEL_MAP: Record<LogLevelName, LogLevelMeta> = {
  TRACE: LOG_LEVELS[0] as LogLevelMeta,
  DEBUG: LOG_LEVELS[1] as LogLevelMeta,
  INFO: LOG_LEVELS[2] as LogLevelMeta,
  WARN: LOG_LEVELS[3] as LogLevelMeta,
  ERROR: LOG_LEVELS[4] as LogLevelMeta,
}

/** 级别严重度（大小写不敏感输入） */
export function severityOf(level: string): number {
  const upper = level.trim().toUpperCase() as LogLevelName
  return LOG_LEVEL_MAP[upper]?.severity ?? 0
}

/** 页面上的「最低级别」下拉项，值对应 §4.21 的小写参数 */
export const LOG_LEVEL_FILTERS = [
  { value: 'trace', label: '全部级别' },
  { value: 'debug', label: 'DEBUG 及以上' },
  { value: 'info', label: 'INFO 及以上' },
  { value: 'warn', label: 'WARN 及以上' },
  { value: 'error', label: 'ERROR' },
] as const

/** 时间范围预设（秒） */
export const TIME_RANGE_PRESETS = [
  { value: 'all', label: '全部时间', seconds: 0 },
  { value: '15m', label: '最近 15 分钟', seconds: 15 * 60 },
  { value: '1h', label: '最近 1 小时', seconds: 60 * 60 },
  { value: '24h', label: '最近 24 小时', seconds: 24 * 60 * 60 },
  { value: '7d', label: '最近 7 天', seconds: 7 * 24 * 60 * 60 },
] as const

export type TimeRangeValue = (typeof TIME_RANGE_PRESETS)[number]['value']

/** 应用目录分页尺寸（契约 limit 上限 100） */
export const APP_PAGE_SIZE = 50

/** 日志每页条数（契约 limit 上限 500，§7.3 禁止一次渲染数千条） */
export const LOG_PAGE_SIZE = 200

/** 设备分页尺寸 */
export const DEVICE_PAGE_SIZE = 50

/** 应用重命名长度约束（§4.17） */
export const APP_NAME_MAX_LENGTH = 128

/** 本地提示（toast）停留时间 */
export const TOAST_DURATION_MS = 5_000

export const ROUTE_NAMES = {
  overview: 'overview',
  apps: 'apps',
  logs: 'logs',
  diagnostics: 'diagnostics',
  settings: 'settings',
} as const
