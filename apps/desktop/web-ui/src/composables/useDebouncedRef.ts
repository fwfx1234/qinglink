import { onBeforeUnmount, ref, watch, type Ref } from 'vue'

export interface DebouncedRef<T> {
  value: Ref<T>
  /** 立即把当前值推给回调（例如用户按下回车） */
  flush: () => void
  cancel: () => void
}

/**
 * 输入防抖：界面即时响应，实际请求延后触发，避免逐字符打服务端。
 */
export function useDebouncedRef<T>(source: Ref<T>, delay: number, onChange: (value: T) => void): DebouncedRef<T> {
  const value = ref(source.value) as Ref<T>
  let timer: number | null = null

  function cancel(): void {
    if (timer !== null) {
      window.clearTimeout(timer)
      timer = null
    }
  }

  function flush(): void {
    cancel()
    onChange(value.value)
  }

  watch(source, (next) => {
    value.value = next
  })

  watch(value, (next) => {
    cancel()
    timer = window.setTimeout(() => {
      timer = null
      onChange(next)
    }, delay)
  })

  onBeforeUnmount(cancel)

  return { value, flush, cancel }
}
