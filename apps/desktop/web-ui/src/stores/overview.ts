import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { overviewApi } from '@/api/endpoints'
import type {
  InstallState,
  OverviewData,
  OverviewStats,
  PairingRequest,
  ServiceStatus,
  SettingsData,
  TrayReport,
} from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'

/** 仪表盘聚合数据：首屏只发一次 GET /overview（§6.1） */
export const useOverviewStore = defineStore('overview', () => {
  const data = ref<OverviewData | null>(null)
  const loading = ref(false)
  const error = ref<AppError | null>(null)
  const loadedAt = ref<number | null>(null)
  /** 首屏快照到达前收到的 service/status，等 apply() 时补写 */
  const pendingService = ref<ServiceStatus | null>(null)

  const health = computed(() => data.value?.health ?? null)
  const service = computed<ServiceStatus | null>(() => data.value?.service ?? null)
  const settings = computed<SettingsData | null>(() => data.value?.settings ?? null)
  const installState = computed<InstallState | null>(() => data.value?.install_state ?? null)
  const stats = computed<OverviewStats | null>(() => data.value?.stats ?? null)
  const pairing = computed<PairingRequest | null>(() => data.value?.pairing ?? null)
  const tray = computed<TrayReport | null>(() => data.value?.tray ?? null)
  const loaded = computed(() => data.value !== null)
  /**
   * 服务未安装 / 未运行 / 协议过旧 / 桌面端未授权：整块界面走引导态。
   *
   * 这四种情况需要不同的引导文案，所以这里只回答「是否阻塞」；
   * 具体是哪一种由 service store 的 linkState 决定。
   */
  const serviceUnavailable = computed(
    () => error.value?.serviceUnavailable === true || data.value?.service.connected === false,
  )

  function apply(next: OverviewData | null): void {
    if (next && pendingService.value) {
      next = { ...next, service: pendingService.value }
      pendingService.value = null
    }
    data.value = next
    loadedAt.value = Date.now()
  }

  /** 只更新内嵌的 service 片段（WS service/status 回流） */
  function patchService(status: ServiceStatus): void {
    // 首屏 /overview 还在路上时事件可能先到；先暂存，等 apply() 之后补写，
    // 否则会被随后到达的旧快照覆盖，且此后没有事件再来纠正。
    if (!data.value) {
      pendingService.value = status
      return
    }
    data.value = { ...data.value, service: status }
    pendingService.value = null
  }

  function patchInstallState(state: InstallState): void {
    if (!data.value) return
    data.value = { ...data.value, install_state: { ...data.value.install_state, ...state } }
  }

  function patchSettings(settings: SettingsData): void {
    if (!data.value) return
    data.value = { ...data.value, settings }
  }

  function patchStats(stats: OverviewStats): void {
    if (!data.value) return
    data.value = { ...data.value, stats }
  }

  function patchPairing(pairing: PairingRequest | null): void {
    if (!data.value) return
    data.value = { ...data.value, pairing }
  }

  async function refresh(): Promise<void> {
    loading.value = true
    try {
      const result = await overviewApi.get()
      apply(result)
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      loading.value = false
    }
  }

  return {
    data,
    loading,
    error,
    loadedAt,
    health,
    service,
    settings,
    installState,
    stats,
    pairing,
    tray,
    loaded,
    serviceUnavailable,
    refresh,
    patchService,
    patchInstallState,
    patchSettings,
    patchPairing,
    patchStats,
  }
})
