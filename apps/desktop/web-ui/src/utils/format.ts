/** 展示层格式化工具：不改动数据语义，只负责可读性。 */

import type { LogItem } from '@/api/types'

function pad(value: number): string {
  return value < 10 ? `0${value}` : String(value)
}

/** Unix 秒 → YYYY-MM-DD HH:mm:ss（本地时区） */
export function formatUnixSeconds(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return '—'
  const date = new Date(seconds * 1000)
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(
    date.getMinutes(),
  )}:${pad(date.getSeconds())}`
}

/** Unix 毫秒 → YYYY-MM-DD HH:mm:ss.SSS（本地时区） */
export function formatUnixMillis(millis: number | null | undefined): string {
  if (millis === null || millis === undefined || !Number.isFinite(millis)) return '—'
  const date = new Date(millis)
  const base = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(
    date.getMinutes(),
  )}:${pad(date.getSeconds())}`
  return `${base}.${String(date.getMilliseconds()).padStart(3, '0')}`
}

/** 日志行时间：精确到毫秒 */
export function formatLogTime(item: LogItem): string {
  return formatUnixMillis(item.timestamp_ms || item.timestamp * 1000)
}

/** 相对时间：刚刚 / N 秒前 / N 分钟前 / …… */
export function formatRelative(millis: number | null | undefined, now = Date.now()): string {
  if (millis === null || millis === undefined || !Number.isFinite(millis)) return '—'
  const diff = Math.max(0, now - millis)
  const seconds = Math.floor(diff / 1000)
  if (seconds < 10) return '刚刚'
  if (seconds < 60) return `${seconds} 秒前`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours} 小时前`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days} 天前`
  return formatUnixMillis(millis)
}

/** 运行时长：3d 4h / 4h 12m / 12m 30s / 30s */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds) || seconds < 0) return '—'
  const total = Math.floor(seconds)
  const days = Math.floor(total / 86_400)
  const hours = Math.floor((total % 86_400) / 3_600)
  const minutes = Math.floor((total % 3_600) / 60)
  const secs = total % 60
  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  if (minutes > 0) return `${minutes}m ${secs}s`
  return `${secs}s`
}

/** 字节 → 人类可读 */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes) || bytes < 0) return '—'
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1024
  let unitIndex = 0
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`
}

/** 大数字千分位 */
export function formatCount(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return '—'
  return value.toLocaleString('zh-CN')
}

/** 单行截断：超出 max 字符后接省略号 */
export function truncate(text: string, max: number): string {
  if (!text) return ''
  if (text.length <= max) return text
  return `${text.slice(0, Math.max(0, max - 1))}…`
}

/** 路径展示：保留盘符 / 首段与文件名，中间用 … 折叠 */
export function shortenPath(path: string, max = 64): string {
  if (!path) return ''
  if (path.length <= max) return path
  const separator = path.includes('\\') ? '\\' : '/'
  const parts = path.split(separator)
  const file = parts[parts.length - 1] ?? path
  const head = parts.slice(0, 2).join(separator)
  const candidate = `${head}${separator}…${separator}${file}`
  return candidate.length <= max ? candidate : truncate(candidate, max)
}

/**
 * 路径末尾若干段：`D:\Games\Example\Example.exe` → `…\Example\Example.exe`。
 *
 * 列表行用它代替完整路径：Windows 路径动辄上百字符，直接铺在行里既挤掉名称，
 * 也几乎没人会去读中间那几层目录。完整路径留给详情面板。
 */
export function tailPath(path: string, segments = 2): string {
  if (!path) return ''
  const separator = path.includes('\\') ? '\\' : '/'
  const parts = path.split(separator).filter((part) => part.length > 0)
  if (parts.length <= segments) return path
  return `…${separator}${parts.slice(-segments).join(separator)}`
}

/** target 字段的空值处理（契约中 thread_name / file 可能为 null） */
export function orDash(value: string | null | undefined): string {
  const text = (value ?? '').trim()
  return text === '' ? '—' : text
}

/** Unix 秒 → datetime-local 输入框值（本地时区） */
export function toDateTimeLocalValue(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return ''
  const date = new Date(seconds * 1000)
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(
    date.getMinutes(),
  )}`
}

/** datetime-local 输入框值 → Unix 秒（无效返回 null） */
export function fromDateTimeLocalValue(value: string): number | null {
  if (!value) return null
  const millis = new Date(value).getTime()
  if (!Number.isFinite(millis)) return null
  return Math.floor(millis / 1000)
}
