import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { serviceApi, type ServiceAction } from '@/api/endpoints'
import type { InstallState, ServiceLinkState, ServiceStatus } from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'

type BusyAction = ServiceAction | 'pause' | 'reconnect' | null

/** 链路状态的中文短标签，供头部与卡片共用。 */
const LINK_LABELS: Record<ServiceLinkState, string> = {
  connecting: '正在连接服务',
  connected: '实时连接正常',
  protocol_mismatch: '服务版本过旧',
  unauthorized: '桌面端未授权',
  unreachable: '服务未连接',
}

/** 服务状态 + 安全输入服务体检（§4.5 – §4.8） */
export const useServiceStore = defineStore('service', () => {
  const status = ref<ServiceStatus | null>(null)
  const installState = ref<InstallState | null>(null)
  const loading = ref(false)
  const error = ref<AppError | null>(null)
  const busyAction = ref<BusyAction>(null)
  const lastResult = ref<string | null>(null)

  const linkState = computed<ServiceLinkState>(() => status.value?.state ?? 'connecting')
  const linkLabel = computed(() => LINK_LABELS[linkState.value])
  const connected = computed(() => linkState.value === 'connected')
  const protocolMismatch = computed(() => linkState.value === 'protocol_mismatch')
  const unauthorized = computed(() => linkState.value === 'unauthorized')
  /** 需要「修复服务」而不是重装/重启的任一种情况 */
  const needsRepair = computed(() => protocolMismatch.value || unauthorized.value)
  const expectedVersion = computed(() => status.value?.expected_version ?? null)
  const serviceVersion = computed(() => status.value?.service_version ?? null)
  const paused = computed(() => status.value?.paused === true)
  const networkReady = computed(() => status.value?.network_ready === true)
  const address = computed(() => status.value?.address ?? null)
  const lastServiceError = computed(() => status.value?.last_error ?? null)
  const capabilities = computed(() => status.value?.capabilities ?? null)
  const canManage = computed(() => capabilities.value?.management === true)
  const canPause = computed(() => capabilities.value?.pause === true)
  const canAddApp = computed(() => capabilities.value?.app_add === true)
  const canLaunchApp = computed(() => capabilities.value?.app_launch === true)
  const canManageIcon = computed(() => capabilities.value?.app_icon_manage === true)

  const install = computed(() => installState.value)
  const packageAvailable = computed(() => installState.value?.package_available === true)
  const serviceInstalled = computed(() => installState.value?.service_installed === true)
  const serviceRunning = computed(() => installState.value?.service_running === true)
  const binaryMatches = computed(() => installState.value?.service_binary_matches === true)
  const protocolOk = computed(() => installState.value?.service_protocol_ok ?? null)
  const available = computed(() => installState.value?.available === true)
  /** 提权操作进行中：由 install_state 事件的 busy 字段驱动 */
  const elevatedBusy = computed(() => installState.value?.busy === true)
  const busy = computed(() => busyAction.value !== null || elevatedBusy.value)

  const healthTone = computed<'success' | 'warning' | 'error' | 'neutral'>(() => {
    if (!installState.value) return 'neutral'
    if (!packageAvailable.value) return 'warning'
    if (protocolOk.value === false) return 'error'
    if (serviceInstalled.value && serviceRunning.value && binaryMatches.value) return 'success'
    if (installState.value.busy) return 'warning'
    return 'error'
  })

  function isBusy(action: BusyAction): boolean {
    return busyAction.value === action || elevatedBusy.value
  }

  function applyStatus(next: ServiceStatus): void {
    status.value = next
  }

  function applyInstallState(next: InstallState): void {
    installState.value = { ...installState.value, ...next }
    if (next.busy === false && typeof next.result === 'string' && next.result.trim() !== '') {
      lastResult.value = next.result
    }
  }

  async function loadStatus(): Promise<void> {
    loading.value = true
    try {
      applyStatus(await serviceApi.get())
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      loading.value = false
    }
  }

  async function loadInstallState(): Promise<void> {
    try {
      applyInstallState(await serviceApi.installState())
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
    }
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      const [nextStatus, nextInstallState] = await Promise.all([serviceApi.get(), serviceApi.installState()])
      applyStatus(nextStatus)
      applyInstallState(nextInstallState)
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      loading.value = false
    }
  }

  async function setPaused(paused: boolean): Promise<void> {
    busyAction.value = 'pause'
    try {
      await serviceApi.setPaused(paused)
      // 不做乐观更新：等服务端 service/status 事件回流
    } finally {
      busyAction.value = null
    }
  }

  /**
   * 重连：接口立即返回 accepted，真实结果由 service/status 事件回流。
   * 先做一次乐观置位，让按钮在事件到达前就显示「正在连接」。
   */
  async function reconnect(): Promise<void> {
    busyAction.value = 'reconnect'
    try {
      if (status.value) {
        applyStatus({ ...status.value, connected: false, state: 'connecting' })
      }
      await serviceApi.reconnect()
    } finally {
      busyAction.value = null
    }
  }

  /**
   * 提权操作：接口立即返回 accepted，真实结果由 install_state 事件收敛。
   * 这里只把 busy 态打开（对齐「提权操作后要显示 busy 态并靠事件收敛」）。
   */
  async function runAction(action: ServiceAction): Promise<boolean> {
    busyAction.value = action
    try {
      const result = await serviceApi.action(action)
      return result?.accepted === true
    } catch (caught) {
      error.value = toAppError(caught)
      throw caught
    } finally {
      busyAction.value = null
    }
  }

  return {
    status,
    installState,
    loading,
    error,
    busyAction,
    lastResult,
    linkState,
    linkLabel,
    connected,
    protocolMismatch,
    unauthorized,
    needsRepair,
    expectedVersion,
    serviceVersion,
    paused,
    networkReady,
    address,
    lastServiceError,
    capabilities,
    canManage,
    canPause,
    canAddApp,
    canLaunchApp,
    canManageIcon,
    install,
    packageAvailable,
    serviceInstalled,
    serviceRunning,
    binaryMatches,
    protocolOk,
    available,
    elevatedBusy,
    busy,
    healthTone,
    isBusy,
    applyStatus,
    applyInstallState,
    loadStatus,
    loadInstallState,
    load,
    setPaused,
    reconnect,
    runAction,
  }
})
