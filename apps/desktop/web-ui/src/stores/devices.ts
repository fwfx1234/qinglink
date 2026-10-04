import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { devicesApi } from '@/api/endpoints'
import type { DeviceItem } from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'
import { DEVICE_PAGE_SIZE } from '@/utils/constants'

/** 已配对设备列表与撤销（§4.10 / §4.11） */
export const useDevicesStore = defineStore('devices', () => {
  const items = ref<DeviceItem[]>([])
  const offset = ref(0)
  const limit = ref(DEVICE_PAGE_SIZE)
  const total = ref(0)
  const nextOffset = ref<number | null>(null)
  const loading = ref(false)
  const error = ref<AppError | null>(null)
  const revokingId = ref<string | null>(null)

  const page = computed(() => Math.floor(offset.value / limit.value) + 1)
  const pageCount = computed(() => Math.max(1, Math.ceil(total.value / limit.value)))
  const hasPrev = computed(() => offset.value > 0)
  const hasNext = computed(() => nextOffset.value !== null && items.value.length > 0)
  const isEmpty = computed(() => !loading.value && items.value.length === 0)

  async function fetchPage(nextOffsetValue = offset.value): Promise<void> {
    loading.value = true
    try {
      const pageData = await devicesApi.list(nextOffsetValue, limit.value)
      items.value = pageData.items
      offset.value = pageData.offset
      limit.value = pageData.limit
      total.value = pageData.total
      nextOffset.value = pageData.next_offset
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

  /** 由 devices/changed 事件触发：重新拉取当前页 */
  async function refresh(): Promise<void> {
    await fetchPage(offset.value)
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

  /**
   * §4.11 `id` 必须原样回传（含 paired:/legacy: 前缀）。不做乐观更新：
   * 请求成功后重新读取真值，同时 devices/changed 事件让其他标签页一起收敛。
   */
  async function revoke(deviceId: string): Promise<void> {
    revokingId.value = deviceId
    try {
      await devicesApi.revoke(deviceId)
    } finally {
      revokingId.value = null
    }
    await refresh()
  }

  return {
    items,
    offset,
    limit,
    total,
    nextOffset,
    loading,
    error,
    revokingId,
    page,
    pageCount,
    hasPrev,
    hasNext,
    isEmpty,
    fetchPage,
    refresh,
    goToPage,
    nextPage,
    prevPage,
    revoke,
  }
})
