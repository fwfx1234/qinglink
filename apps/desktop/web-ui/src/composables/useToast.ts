import { storeToRefs } from 'pinia'
import { useNotificationsStore } from '@/stores/notifications'

/** toast 便捷接口：组件只依赖这个组合式函数，不直接操作 store。 */
export function useToast() {
  const store = useNotificationsStore()
  const { items } = storeToRefs(store)

  return {
    items,
    info: (message: string) => store.info(message),
    warn: (message: string) => store.warn(message),
    error: (message: string) => store.error(message),
    fromError: (error: unknown, fallback?: string) => store.fromError(error, fallback),
    dismiss: (id: number) => store.dismiss(id),
  }
}
