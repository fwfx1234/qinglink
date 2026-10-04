import { storeToRefs } from 'pinia'
import { useNotificationsStore } from '@/stores/notifications'

export interface ConfirmOptions {
  title: string
  message: string
  detail?: string | null
  confirmText?: string
  cancelText?: string
  tone?: 'default' | 'danger'
}

/** 全局二次确认：返回用户是否确认（由 App.vue 的 ConfirmDialog 渲染）。 */
export function useConfirm() {
  const store = useNotificationsStore()
  const { confirmRequest } = storeToRefs(store)

  function confirm(options: ConfirmOptions): Promise<boolean> {
    return store.confirm(options)
  }

  function settle(accepted: boolean): void {
    store.settleConfirm(accepted)
  }

  return { confirmRequest, confirm, settle }
}
