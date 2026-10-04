import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { pairingApi } from '@/api/endpoints'
import type { PairingRequest } from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'

/** 待确认配对请求（§4.12 / §4.13） */
export const usePairingStore = defineStore('pairing', () => {
  const pairing = ref<PairingRequest | null>(null)
  const loading = ref(false)
  const error = ref<AppError | null>(null)
  const cancelling = ref(false)
  const loadedOnce = ref(false)

  const hasPending = computed(() => pairing.value !== null)
  const pin = computed(() => pairing.value?.pin ?? '')
  const deviceName = computed(() => pairing.value?.device_name ?? '')
  const deviceId = computed(() => pairing.value?.device_id ?? '')
  const expiresAt = computed(() => pairing.value?.expires_at_unix ?? null)

  function apply(next: PairingRequest | null): void {
    pairing.value = next
    loadedOnce.value = true
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      const result = await pairingApi.get()
      apply(result?.pairing ?? null)
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      loading.value = false
    }
  }

  /** 取消后不做乐观更新：由 pairing/pending 事件把值收敛为 null */
  async function cancel(): Promise<void> {
    const current = pairing.value
    if (!current) return
    cancelling.value = true
    try {
      await pairingApi.cancel(current.request_id)
    } finally {
      cancelling.value = false
    }
  }

  return {
    pairing,
    loading,
    error,
    cancelling,
    loadedOnce,
    hasPending,
    pin,
    deviceName,
    deviceId,
    expiresAt,
    apply,
    load,
    cancel,
  }
})
