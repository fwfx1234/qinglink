import { ref } from 'vue'
import { useAppsStore } from '@/stores/apps'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import type { AppAddRequest } from '@/api/types'

/** 新增自定义条目（§4.31）：弹窗开关、提交中的忙碌状态与就地错误提示。 */
export function useAppAdd() {
  const apps = useAppsStore()
  const notifications = useNotificationsStore()

  const addOpen = ref(false)
  const addBusy = ref(false)
  const addError = ref<string | null>(null)

  function openAdd(): void {
    addError.value = null
    addOpen.value = true
  }

  function closeAdd(): void {
    addOpen.value = false
  }

  async function submitAdd(payload: AppAddRequest): Promise<void> {
    addBusy.value = true
    addError.value = null
    try {
      const id = await apps.add(payload)
      if (id !== null) {
        addOpen.value = false
        notifications.info('已加入目录，列表会自动刷新')
      }
    } catch (caught) {
      const appError = toAppError(caught)
      addError.value = appError.message
    } finally {
      addBusy.value = false
    }
  }

  return { addOpen, addBusy, addError, openAdd, closeAdd, submitAdd }
}
