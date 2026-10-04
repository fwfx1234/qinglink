import { computed, reactive, ref } from 'vue'
import { defineStore } from 'pinia'
import { appsApi } from '@/api/endpoints'
import type {
  AppAddRequest,
  AppBatchAction,
  AppBatchItemResult,
  AppIconCandidates,
  AppIconUpdate,
  AppItem,
  AppsStats,
  CatalogKind,
  CatalogKindStats,
} from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'
import { APP_PAGE_SIZE } from '@/utils/constants'
import { Watchdog } from '@/utils/async'
import { useConnectionStore } from '@/stores/connection'

/** 扫描结束的兜底等待时间：事件未到达时重试拉取一次 */
const SCAN_WATCHDOG_MS = 20_000

/** 一次批量操作允许的最大条目数，与服务端上限一致。 */
export const MAX_BATCH_IDS = 100

/** 应用 / 游戏目录：列表、分页、查询、统计、写操作（§4.14 – §4.20） */
export const useAppsStore = defineStore('apps', () => {
  const kind = ref<CatalogKind>('app')
  const query = ref('')
  const includeHidden = ref(true)
  /** 只看「常用（我的库）」 */
  const favoriteOnly = ref(false)
  const items = ref<AppItem[]>([])
  const offset = ref(0)
  const limit = ref(APP_PAGE_SIZE)
  const total = ref(0)
  const nextOffset = ref<number | null>(null)
  const loading = ref(false)
  const error = ref<AppError | null>(null)
  const stats = ref<AppsStats | null>(null)
  const statsLoading = ref(false)
  const scanning = reactive<Record<CatalogKind, boolean>>({ app: false, game: false })
  const busyIds = ref<string[]>([])
  const catalogVersion = ref(0)
  /** 批量操作进行中 */
  const batchBusy = ref(false)
  /** 已勾选的条目 id（跨页保留，但切页时清空以避免误操作不可见条目） */
  const selectedIds = ref<string[]>([])
  /** 最近一次目录变化原因，供界面提示 */
  const lastChangeReason = ref<string | null>(null)
  const watchdog = new Watchdog()

  const selectedCount = computed(() => selectedIds.value.length)
  const hasSelection = computed(() => selectedIds.value.length > 0)

  const currentKindStats = computed<CatalogKindStats | null>(() => stats.value?.[kind.value] ?? null)
  const page = computed(() => Math.floor(offset.value / limit.value) + 1)
  const pageCount = computed(() => Math.max(1, Math.ceil(total.value / limit.value)))
  const hasPrev = computed(() => offset.value > 0)
  const hasNext = computed(() => nextOffset.value !== null && items.value.length > 0)
  const isEmpty = computed(() => !loading.value && items.value.length === 0)
  const isScanning = computed(() => scanning[kind.value])
  const hiddenCount = computed(() => items.value.filter((item) => item.hidden).length)

  function isBusy(id: string): boolean {
    return busyIds.value.includes(id)
  }

  function markBusy(id: string, busy: boolean): void {
    const set = new Set(busyIds.value)
    if (busy) set.add(id)
    else set.delete(id)
    busyIds.value = [...set]
  }

  function applyStats(next: AppsStats): void {
    stats.value = next
  }

  async function fetchPage(targetOffset = offset.value): Promise<void> {
    loading.value = true
    try {
      const pageData = await appsApi.list({
        kind: kind.value,
        offset: targetOffset,
        limit: limit.value,
        query: query.value.trim(),
        includeHidden: includeHidden.value,
        favoriteOnly: favoriteOnly.value,
      })
      items.value = pageData.items
      offset.value = pageData.offset
      limit.value = pageData.limit
      total.value = pageData.total
      nextOffset.value = pageData.next_offset
      // 勾选不跨页保留：否则批量操作会作用到当前看不见的条目。
      selectedIds.value = []
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
      items.value = []
      total.value = 0
      nextOffset.value = null
    } finally {
      loading.value = false
    }
  }

  async function loadStats(): Promise<void> {
    statsLoading.value = true
    try {
      applyStats(await appsApi.stats())
    } catch (caught) {
      const appError = toAppError(caught)
      if (!appError.serviceUnavailable) {
        error.value = appError
      }
    } finally {
      statsLoading.value = false
    }
  }

  async function reload(): Promise<void> {
    await Promise.all([fetchPage(offset.value), loadStats()])
  }

  async function load(): Promise<void> {
    offset.value = 0
    await reload()
  }

  function setKind(next: CatalogKind): Promise<void> {
    if (kind.value === next) return Promise.resolve()
    kind.value = next
    offset.value = 0
    return reload()
  }

  function setQuery(next: string): Promise<void> {
    query.value = next
    offset.value = 0
    return reload()
  }

  function setIncludeHidden(next: boolean): Promise<void> {
    includeHidden.value = next
    offset.value = 0
    return reload()
  }

  function setFavoriteOnly(next: boolean): Promise<void> {
    favoriteOnly.value = next
    offset.value = 0
    return reload()
  }

  function goToPage(target: number): Promise<void> {
    const clamped = Math.min(Math.max(target, 1), pageCount.value)
    return fetchPage((clamped - 1) * limit.value)
  }

  function nextPage(): Promise<void> {
    if (nextOffset.value === null) return Promise.resolve()
    return fetchPage(nextOffset.value)
  }

  function prevPage(): Promise<void> {
    if (!hasPrev.value) return Promise.resolve()
    return fetchPage(Math.max(0, offset.value - limit.value))
  }

  /** 目录变化事件（catalog_changed）：重新拉取当前页与统计（§6.2 / §6.3） */
  function onCatalogChanged(reason: string): void {
    lastChangeReason.value = reason
    catalogVersion.value += 1
    if (reason === 'scan_finished') {
      scanning.app = false
      scanning.game = false
      watchdog.disarm('scan')
    }
    void reload()
  }

  /** 开始扫描：事件与服务端响应都会触发，重复调用只保留一个看门狗 */
  function startScanning(target: CatalogKind): void {
    scanning[target] = true
    watchdog.arm('scan', SCAN_WATCHDOG_MS, () => {
      scanning.app = false
      scanning.game = false
      void reload()
    })
  }

  function onScanStarted(scanKind: CatalogKind): void {
    startScanning(scanKind)
  }

  /**
   * 写操作的主路径是事件回流（服务端在写成功后立即广播 catalog_changed）。
   * 实时通道断开时事件不会到达，此时自己重拉一次兜底。
   */
  async function refreshUnlessEventWillArrive(): Promise<void> {
    if (!useConnectionStore().isOpen) await reload()
  }

  /* ------------------------------ 写操作：不做乐观更新 ------------------------------ */

  async function rename(id: string, name: string): Promise<void> {
    markBusy(id, true)
    try {
      await appsApi.rename(id, name.trim())
    } finally {
      markBusy(id, false)
    }
    await refreshUnlessEventWillArrive()
  }

  async function setHidden(id: string, hidden: boolean): Promise<void> {
    markBusy(id, true)
    try {
      await appsApi.setHidden(id, hidden)
    } finally {
      markBusy(id, false)
    }
    await refreshUnlessEventWillArrive()
  }

  /** 加入 / 移出「常用」。手机端「库」里看到的就是这一份。 */
  async function setFavorite(id: string, favorite: boolean): Promise<void> {
    markBusy(id, true)
    try {
      await appsApi.setFavorite(id, favorite)
    } finally {
      markBusy(id, false)
    }
    await refreshUnlessEventWillArrive()
  }

  /** 置顶到手机首页（会自动加入常用）。 */
  async function setPinned(id: string, pinned: boolean): Promise<void> {
    markBusy(id, true)
    try {
      await appsApi.setPinned(id, pinned)
    } finally {
      markBusy(id, false)
    }
    await refreshUnlessEventWillArrive()
  }

  async function remove(id: string): Promise<void> {
    markBusy(id, true)
    try {
      await appsApi.remove(id)
    } finally {
      markBusy(id, false)
    }
    await refreshUnlessEventWillArrive()
  }

  /** 触发扫描；进行中重复调用服务端返回 CONFLICT（§4.20） */
  async function scan(target: CatalogKind = kind.value): Promise<boolean> {
    const result = await appsApi.scan(target)
    if (result?.accepted) {
      startScanning(target)
    }
    return result?.accepted === true
  }

  /** 该条目可选的图标文件（§4.34），不改变目录状态 */
  function iconCandidates(id: string): Promise<AppIconCandidates> {
    return appsApi.iconCandidates(id)
  }

  /**
   * 重新解析一个条目的图标并落库（§4.34）。
   *
   * 成功后自增 `catalogVersion`，让 `AppIcon` 拿到新的 `icon_updated_at`，
   * 从而绕开浏览器对图标 URL 的长缓存。
   */
  async function refreshIcon(id: string, iconPath?: string | null): Promise<AppIconUpdate> {
    markBusy(id, true)
    try {
      const result = await appsApi.refreshIcon(id, iconPath)
      catalogVersion.value += 1
      return result
    } finally {
      markBusy(id, false)
      // 服务端写成功后会广播 catalog_changed；实时通道断开时自己兜底重拉。
      await refreshUnlessEventWillArrive()
    }
  }

  function isSelected(id: string): boolean {
    return selectedIds.value.includes(id)
  }

  function toggleSelected(id: string): void {
    const set = new Set(selectedIds.value)
    if (set.has(id)) set.delete(id)
    else set.add(id)
    selectedIds.value = [...set]
  }

  function setSelected(ids: string[]): void {
    selectedIds.value = [...new Set(ids)].slice(0, MAX_BATCH_IDS)
  }

  function setAllSelected(selected: boolean): void {
    selectedIds.value = selected ? items.value.map((item) => item.id).slice(0, MAX_BATCH_IDS) : []
  }

  function clearSelection(): void {
    selectedIds.value = []
  }

  /** 新增自定义条目（§4.31）；目录通过 catalog_changed 事件回流刷新 */
  async function add(payload: AppAddRequest): Promise<string | null> {
    batchBusy.value = true
    try {
      const result = await appsApi.add(payload)
      return result?.id ?? null
    } finally {
      batchBusy.value = false
    }
  }

  /** 在 PC 上启动条目（§4.32） */
  async function launch(id: string): Promise<void> {
    markBusy(id, true)
    try {
      await appsApi.launch(id)
    } finally {
      markBusy(id, false)
    }
  }

  /**
   * 批量操作（§4.33）。
   *
   * 逐条执行、逐条汇报：部分失败不影响其余条目，界面按 succeeded/failed 汇总。
   */
  async function batch(action: AppBatchAction, ids: string[]): Promise<AppBatchItemResult[]> {
    if (ids.length === 0) return []
    batchBusy.value = true
    try {
      const result = await appsApi.batch({ action, ids: ids.slice(0, MAX_BATCH_IDS) })
      return result?.results ?? []
    } finally {
      batchBusy.value = false
      clearSelection()
    }
  }

  return {
    kind,
    query,
    includeHidden,
    favoriteOnly,
    items,
    offset,
    limit,
    total,
    nextOffset,
    loading,
    error,
    stats,
    statsLoading,
    scanning,
    busyIds,
    catalogVersion,
    batchBusy,
    selectedIds,
    selectedCount,
    hasSelection,
    lastChangeReason,
    currentKindStats,
    page,
    pageCount,
    hasPrev,
    hasNext,
    isEmpty,
    isScanning,
    hiddenCount,
    isBusy,
    isSelected,
    toggleSelected,
    setSelected,
    setAllSelected,
    clearSelection,
    fetchPage,
    loadStats,
    reload,
    load,
    setKind,
    setQuery,
    setIncludeHidden,
    goToPage,
    nextPage,
    prevPage,
    onCatalogChanged,
    onScanStarted,
    rename,
    setHidden,
    setFavorite,
    setPinned,
    setFavoriteOnly,
    remove,
    scan,
    add,
    launch,
    batch,
    iconCandidates,
    refreshIcon,
  }
})
