import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { diagnosticsApi, logsApi, type LogsListParams } from '@/api/endpoints'
import type { LogFilesResult, LogItem, LogLevelFilter, LogSource, LogsStats } from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'
import { LOG_PAGE_SIZE, severityOf, TIME_RANGE_PRESETS, type TimeRangeValue } from '@/utils/constants'

/** 内存中保留的日志条目上限（§7.3 禁止一次性渲染数千条 DOM） */
export const MAX_BUFFERED_LOGS = 2_000

export interface LogFilters {
  level: LogLevelFilter | null
  target: string
  search: string
  range: TimeRangeValue
  since: number | null
  until: number | null
}

function defaultFilters(): LogFilters {
  return { level: null, target: '', search: '', range: 'all', since: null, until: null }
}

function rangeSeconds(range: TimeRangeValue): number | null {
  const preset = TIME_RANGE_PRESETS.find((item) => item.value === range)
  if (!preset || preset.seconds === 0) return null
  return preset.seconds
}

/** 日志：过滤、分页、实时追加、清空与下载（§4.21 – §4.24） */
export const useLogsStore = defineStore('logs', () => {
  const items = ref<LogItem[]>([])
  const filters = ref<LogFilters>(defaultFilters())
  const limit = ref(LOG_PAGE_SIZE)
  const loading = ref(false)
  const loadingMore = ref(false)
  const error = ref<AppError | null>(null)
  const stats = ref<LogsStats | null>(null)
  const total = ref(0)
  const nextOffset = ref<number | null>(null)
  const droppedCount = ref(0)
  const droppedAt = ref<number | null>(null)
  const pendingCount = ref(0)
  const filterVersion = ref(0)
  /** 视图是否已订阅 logs 频道（§5.2 logs 必须显式订阅） */
  const streaming = ref(false)

  /** 日志来源：桌面端内存缓冲，或 QingLinkService 的磁盘日志 */
  const source = ref<LogSource>('desktop')
  /** 两个来源的磁盘文件清单（§4.28） */
  const files = ref<LogFilesResult | null>(null)
  /** 当前解析的服务端日志文件名 */
  const serviceFile = ref<string | null>(null)
  const serviceItems = ref<LogItem[]>([])
  const serviceTotal = ref(0)
  const serviceNextOffset = ref<number | null>(null)
  const serviceLoading = ref(false)
  const serviceLoadingMore = ref(false)

  /** 当前是否停留在列表顶部（由视图维护，只有为 true 才自动跟随） */
  const atTop = ref(true)
  const autoFollow = ref(true)
  /** 被当前过滤条件挡下的实时条目数（不代表服务端丢弃） */
  const filteredCount = ref(0)

  const levelCounts = computed(() => stats.value?.by_level ?? null)
  const errorCount = computed(() => stats.value?.by_level.ERROR ?? 0)
  const warnCount = computed(() => stats.value?.by_level.WARN ?? 0)
  const hasMore = computed(() => nextOffset.value !== null)
  const isEmpty = computed(() => !loading.value && items.value.length === 0)
  const filtersActive = computed(() => {
    const current = filters.value
    return (
      current.level !== null ||
      current.target.trim() !== '' ||
      current.search.trim() !== '' ||
      current.range !== 'all' ||
      current.since !== null ||
      current.until !== null
    )
  })
  const activeFilterSummary = computed(() => {
    const parts: string[] = []
    const current = filters.value
    if (current.level) parts.push(`级别 ≥ ${current.level.toUpperCase()}`)
    if (current.target.trim()) parts.push(`模块 ~ ${current.target.trim()}`)
    if (current.search.trim()) parts.push(`文本 ~ ${current.search.trim()}`)
    if (current.range !== 'all') {
      const preset = TIME_RANGE_PRESETS.find((item) => item.value === current.range)
      parts.push(preset ? preset.label : '自定义时间')
    }
    return parts
  })

  function queryParams(targetOffset: number, pageLimit: number): LogsListParams {
    const current = filters.value
    const windowSeconds = rangeSeconds(current.range)
    const since = current.since ?? (windowSeconds === null ? null : Math.floor(Date.now() / 1000) - windowSeconds)
    return {
      level: current.level,
      target: current.target.trim(),
      search: current.search.trim(),
      since,
      until: current.until,
      limit: pageLimit,
      offset: targetOffset,
    }
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      const page = await logsApi.list(queryParams(0, limit.value))
      items.value = page.items
      total.value = page.total
      nextOffset.value = page.next_offset
      limit.value = page.limit
      error.value = null
      filterVersion.value += 1
      pendingCount.value = 0
      filteredCount.value = 0
      droppedCount.value = 0
      droppedAt.value = null
    } catch (caught) {
      error.value = toAppError(caught)
      items.value = []
      total.value = 0
      nextOffset.value = null
    } finally {
      loading.value = false
    }
  }

  async function loadMore(): Promise<void> {
    if (nextOffset.value === null || loadingMore.value) return
    loadingMore.value = true
    try {
      const page = await logsApi.list(queryParams(nextOffset.value, limit.value))
      const seen = new Set(items.value.map((item) => item.id))
      items.value = [...items.value, ...page.items.filter((item) => !seen.has(item.id))]
      nextOffset.value = page.next_offset
      total.value = page.total
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      loadingMore.value = false
    }
  }

  async function loadStats(): Promise<void> {
    try {
      stats.value = await logsApi.stats()
    } catch (caught) {
      const appError = toAppError(caught)
      if (!appError.serviceUnavailable) {
        error.value = appError
      }
    }
  }

  /** §5.3 logs/entries：追加到顶部（列表最新在前） */
  function appendEntries(incoming: LogItem[]): void {
    if (incoming.length === 0) return
    const accepted = incoming.filter(acceptsEntry)
    const blocked = incoming.length - accepted.length
    if (blocked > 0) {
      filteredCount.value += blocked
    }
    if (accepted.length === 0) return
    const seen = new Set(items.value.map((item) => item.id))
    const fresh = accepted.filter((item) => !seen.has(item.id))
    if (fresh.length === 0) return
    items.value = [...fresh, ...items.value].slice(0, MAX_BUFFERED_LOGS)
    // `total` 是服务端过滤后的命中条数。活跃过滤下无法从单条增量推出它，
    // 因此在有过滤条件时不动，并始终以缓冲上限封顶，避免长时间运行后虚高。
    if (!filtersActive.value) {
      const cap = stats.value?.buffer_len ?? Number.POSITIVE_INFINITY
      total.value = Math.min(total.value + fresh.length, cap)
    }
    if (!atTop.value && autoFollow.value) {
      pendingCount.value += fresh.length
    } else {
      pendingCount.value = 0
    }
  }

  /** 实时条目是否满足当前过滤条件（与 §4.21 的服务端过滤语义保持一致） */
  function acceptsEntry(item: LogItem): boolean {
    const current = filters.value
    if (current.level) {
      const minimum = severityOf(current.level)
      if (severityOf(item.level.toLowerCase() as LogLevelFilter) < minimum) return false
    }
    if (current.target.trim()) {
      const needle = current.target.trim().toLowerCase()
      if (!item.target.toLowerCase().includes(needle)) return false
    }
    if (current.search.trim()) {
      const needle = current.search.trim().toLowerCase()
      if (!item.message.toLowerCase().includes(needle)) return false
    }
    const windowSeconds = rangeSeconds(current.range)
    if (current.since !== null && item.timestamp < current.since) return false
    if (current.until !== null && item.timestamp > current.until) return false
    if (windowSeconds !== null && item.timestamp < Math.floor(Date.now() / 1000) - windowSeconds) return false
    return true
  }

  function markDropped(count: number): void {
    droppedCount.value += count
    droppedAt.value = Date.now()
  }

  function acknowledgeNew(): void {
    pendingCount.value = 0
  }

  function acknowledgeDropped(): void {
    droppedCount.value = 0
    droppedAt.value = null
  }

  function setAtTop(value: boolean): void {
    atTop.value = value
    if (value) pendingCount.value = 0
  }

  function resetFilters(): Promise<void> {
    filters.value = defaultFilters()
    return load()
  }

  function applyFilters(patch: Partial<LogFilters>): Promise<void> {
    filters.value = { ...filters.value, ...patch }
    filteredCount.value = 0
    return load()
  }

  /** §4.23 清空内存缓冲与数据库记录 */
  async function clear(): Promise<number> {
    const result = await logsApi.clear()
    clearLocal()
    await loadStats()
    return result?.cleared ?? 0
  }

  /** 本地清空（二次调用来自 logs/cleared 事件，用于多标签页收敛） */
  function clearLocal(): void {
    items.value = []
    total.value = 0
    nextOffset.value = null
    pendingCount.value = 0
    filteredCount.value = 0
    droppedCount.value = 0
    droppedAt.value = null
  }

  /** 下载指定来源的磁盘日志文件 */
  async function downloadFrom(target: LogSource, file?: string): Promise<void> {
    const blob = await logsApi.download(file, target)
    const fallback = target === 'service' ? 'QingLinkService.log' : 'qinglink.log'
    saveBlob(blob, file ?? fallback)
  }

  /** §4.27 导出纯文本诊断包 */
  async function downloadDiagnosticsBundle(): Promise<void> {
    const response = await fetch(diagnosticsApi.bundleUrl(), { headers: { Accept: 'text/plain' } })
    if (!response.ok) {
      throw new Error(`导出诊断包失败（HTTP ${response.status}）`)
    }
    const blob = await response.blob()
    saveBlob(blob, `qinglink-diagnostics-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.txt`)
  }

  function saveBlob(blob: Blob, name: string): void {
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = name
    document.body.appendChild(anchor)
    anchor.click()
    anchor.remove()
    window.setTimeout(() => URL.revokeObjectURL(url), 5_000)
  }

  /** §4.28 拉取两个来源的磁盘文件清单 */
  async function loadFiles(): Promise<void> {
    try {
      files.value = await logsApi.files()
      error.value = null
    } catch (caught) {
      const appError = toAppError(caught)
      if (!appError.serviceUnavailable) error.value = appError
    }
  }

  /** 切换来源；切到服务端时按需拉取一次 */
  function setSource(next: LogSource): void {
    source.value = next
    if (next === 'service' && serviceItems.value.length === 0) {
      void loadService()
    }
  }

  function setServiceFile(file: string | null): void {
    serviceFile.value = file
    void loadService(file)
  }

  /**
   * §4.29 解析服务端磁盘日志。
   *
   * 服务端日志是磁盘快照，不是实时流：这里只按当前过滤条件拉取一页，
   * 不参与 logs 频道的推送合并。
   */
  async function loadService(file: string | null = serviceFile.value): Promise<void> {
    serviceLoading.value = true
    try {
      const page = await logsApi.service({ ...queryParams(0, limit.value), file: file ?? undefined })
      serviceItems.value = page.items
      serviceTotal.value = page.total
      serviceNextOffset.value = page.next_offset
      serviceFile.value = page.file
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
      serviceItems.value = []
      serviceTotal.value = 0
      serviceNextOffset.value = null
    } finally {
      serviceLoading.value = false
    }
  }

  async function loadServiceMore(): Promise<void> {
    if (serviceNextOffset.value === null || serviceLoadingMore.value) return
    serviceLoadingMore.value = true
    try {
      const page = await logsApi.service({
        ...queryParams(serviceNextOffset.value, limit.value),
        file: serviceFile.value ?? undefined,
      })
      const seen = new Set(serviceItems.value.map((item) => item.id))
      serviceItems.value = [...serviceItems.value, ...page.items.filter((item) => !seen.has(item.id))]
      serviceNextOffset.value = page.next_offset
      serviceTotal.value = page.total
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      serviceLoadingMore.value = false
    }
  }

  function setStreaming(value: boolean): void {
    streaming.value = value
  }

  return {
    items,
    filters,
    limit,
    loading,
    loadingMore,
    error,
    stats,
    total,
    nextOffset,
    droppedCount,
    droppedAt,
    pendingCount,
    filteredCount,
    filterVersion,
    streaming,
    source,
    files,
    serviceFile,
    serviceItems,
    serviceTotal,
    serviceNextOffset,
    serviceLoading,
    serviceLoadingMore,
    atTop,
    autoFollow,
    levelCounts,
    errorCount,
    warnCount,
    hasMore,
    isEmpty,
    filtersActive,
    activeFilterSummary,
    acceptsEntry,
    load,
    loadMore,
    loadStats,
    appendEntries,
    markDropped,
    acknowledgeNew,
    acknowledgeDropped,
    setAtTop,
    resetFilters,
    applyFilters,
    clear,
    clearLocal,
    downloadFrom,
    downloadDiagnosticsBundle,
    loadFiles,
    setSource,
    setServiceFile,
    loadService,
    loadServiceMore,
    setStreaming,
  }
})
