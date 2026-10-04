import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { diagnosticsApi } from '@/api/endpoints'
import type { DiagnosticsReport } from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'

/**
 * 服务诊断中心数据（§4.26）。
 *
 * 只在用户主动刷新或首次进入诊断页时拉取：它包含管道探测，属于按需操作，
 * 不适合参与任何轮询。
 */
export const useDiagnosticsStore = defineStore('diagnostics', () => {
  const report = ref<DiagnosticsReport | null>(null)
  const loading = ref(false)
  const error = ref<AppError | null>(null)
  const loadedAt = ref<number | null>(null)

  const pipeReachable = computed(() => report.value?.pipe_probe.reachable === true)
  const serviceState = computed(() => report.value?.service.state ?? null)

  async function load(): Promise<void> {
    loading.value = true
    try {
      report.value = await diagnosticsApi.get()
      loadedAt.value = Date.now()
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      loading.value = false
    }
  }

  return { report, loading, error, loadedAt, pipeReachable, serviceState, load }
})
