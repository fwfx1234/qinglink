import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { errorMessage } from '@/utils/errors'
import { TOAST_DURATION_MS } from '@/utils/constants'
import type { ToastLevel } from '@/api/types'

export interface ToastItem {
  id: number
  level: ToastLevel
  message: string
  createdAt: number
}

let sequence = 0

/** 全局二次确认请求的状态（由 App.vue 中的唯一 ConfirmDialog 渲染） */
export interface ConfirmRequest {
  id: number
  title: string
  message: string
  detail: string | null
  confirmText: string
  cancelText: string
  tone: 'default' | 'danger'
}

/** 本地 toast（含 WS notice/toast 事件回流提示）与全局二次确认 */
export const useNotificationsStore = defineStore('notifications', () => {
  const items = ref<ToastItem[]>([])
  const confirmRequest = ref<ConfirmRequest | null>(null)
  const timers = new Map<number, number>()
  let confirmResolver: ((value: boolean) => void) | null = null

  const hasItems = computed(() => items.value.length > 0)

  function push(level: ToastLevel, message: string, duration = TOAST_DURATION_MS): number {
    const id = ++sequence
    items.value = [...items.value, { id, level, message, createdAt: Date.now() }]
    const timer = window.setTimeout(() => dismiss(id), duration)
    timers.set(id, timer)
    return id
  }

  function dismiss(id: number): void {
    const timer = timers.get(id)
    if (timer !== undefined) {
      window.clearTimeout(timer)
      timers.delete(id)
    }
    items.value = items.value.filter((item) => item.id !== id)
  }

  function info(message: string): number {
    return push('info', message)
  }

  function warn(message: string): number {
    return push('warn', message)
  }

  function error(message: string): number {
    return push('error', message)
  }

  /** 从任意异常提示用户 */
  function fromError(error: unknown, fallback = '操作失败'): number {
    const message = errorMessage(error)
    return push('error', message || fallback)
  }

  /** 打开二次确认，返回用户是否确认 */
  function confirm(options: {
    title: string
    message: string
    detail?: string | null
    confirmText?: string
    cancelText?: string
    tone?: 'default' | 'danger'
  }): Promise<boolean> {
    if (confirmResolver) {
      confirmResolver(false)
      confirmResolver = null
    }
    return new Promise<boolean>((resolve) => {
      confirmResolver = resolve
      confirmRequest.value = {
        id: ++sequence,
        title: options.title,
        message: options.message,
        detail: options.detail ?? null,
        confirmText: options.confirmText ?? '确认',
        cancelText: options.cancelText ?? '取消',
        tone: options.tone ?? 'default',
      }
    })
  }

  function settleConfirm(accepted: boolean): void {
    confirmRequest.value = null
    const resolver = confirmResolver
    confirmResolver = null
    resolver?.(accepted)
  }

  function clear(): void {
    for (const timer of timers.values()) {
      window.clearTimeout(timer)
    }
    timers.clear()
    items.value = []
  }

  return {
    items,
    confirmRequest,
    hasItems,
    push,
    dismiss,
    info,
    warn,
    error,
    fromError,
    confirm,
    settleConfirm,
    clear,
  }
})
