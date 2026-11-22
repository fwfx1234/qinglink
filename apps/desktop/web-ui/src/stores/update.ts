import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { updateApi } from '@/api/endpoints'
import type { UpdatePhase, UpdateView } from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'

/** 阶段对应的中文短标签，供状态徽标与按钮标题共用。 */
const PHASE_LABELS: Record<UpdatePhase, string> = {
  idle: '尚未检查',
  checking: '正在检查',
  up_to_date: '已是最新',
  available: '发现新版本',
  downloading: '正在下载',
  ready: '等待安装',
  installing: '正在安装',
  error: '检查失败',
}

/** 程序更新状态与动作（§4.37 – §4.40、§5 update_state）。 */
export const useUpdateStore = defineStore('update', () => {
  const view = ref<UpdateView | null>(null)
  const loading = ref(false)
  const actionError = ref<AppError | null>(null)

  const phase = computed<UpdatePhase>(() => view.value?.phase ?? 'idle')
  const phaseLabel = computed(() => PHASE_LABELS[phase.value])
  const currentVersion = computed(() => view.value?.current_version ?? '—')
  const latestVersion = computed(() => view.value?.latest_version ?? null)
  const releaseUrl = computed(() => view.value?.release_url ?? null)
  const notes = computed(() => view.value?.notes ?? '')
  const progress = computed(() => view.value?.progress ?? 0)
  const downloadedBytes = computed(() => view.value?.downloaded_bytes ?? 0)
  const totalBytes = computed(() => view.value?.total_bytes ?? 0)
  const stateError = computed(() => view.value?.error ?? null)
  const busy = computed(() => view.value?.busy ?? false)
  const hasUpdate = computed(() => {
    const p = phase.value
    return p === 'available' || p === 'downloading' || p === 'ready'
  })
  const canDownload = computed(() => phase.value === 'available' && !busy.value)
  const canInstall = computed(() => phase.value === 'ready' && !busy.value)
  const canCheck = computed(() =>
    ['idle', 'up_to_date', 'available', 'error'].includes(phase.value) && !busy.value,
  )
  const lastCheckSecs = computed(() => view.value?.last_check_secs ?? null)

  /** 事件回流：整份快照覆盖，界面以此为准。 */
  function apply(next: UpdateView): void {
    view.value = next
    actionError.value = null
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      apply(await updateApi.get())
    } catch (caught) {
      actionError.value = toAppError(caught)
    } finally {
      loading.value = false
    }
  }

  async function check(): Promise<void> {
    actionError.value = null
    try {
      await updateApi.check()
    } catch (caught) {
      actionError.value = toAppError(caught)
    }
  }

  async function download(): Promise<void> {
    actionError.value = null
    try {
      await updateApi.download()
    } catch (caught) {
      actionError.value = toAppError(caught)
    }
  }

  async function install(): Promise<void> {
    actionError.value = null
    try {
      await updateApi.install()
    } catch (caught) {
      actionError.value = toAppError(caught)
    }
  }

  return {
    view,
    loading,
    actionError,
    phase,
    phaseLabel,
    currentVersion,
    latestVersion,
    releaseUrl,
    notes,
    progress,
    downloadedBytes,
    totalBytes,
    stateError,
    busy,
    hasUpdate,
    canDownload,
    canInstall,
    canCheck,
    lastCheckSecs,
    apply,
    load,
    check,
    download,
    install,
  }
})
