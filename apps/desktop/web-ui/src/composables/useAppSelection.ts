import { useAppsStore } from '@/stores/apps'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import type { AppBatchAction, AppItem } from '@/api/types'

/** 一次最多重扫的图标数，与服务端上限一致（每条都要做一次 shell 取图）。 */
const MAX_BATCH_ICON_REFRESH = 50

/** 批量操作在提示语里的动作名 */
const BATCH_LABELS: Record<AppBatchAction, string> = {
  hide: '隐藏',
  show: '取消隐藏',
  delete: '删除',
  restore_name: '恢复原名',
  refresh_icon: '重新扫描图标',
}

/** 批量选择与批量操作（§4.33）：勾选、全选本页、取消选择，以及确认后的批量执行。 */
export function useAppSelection() {
  const apps = useAppsStore()
  const notifications = useNotificationsStore()

  function toggleSelect(app: AppItem): void {
    apps.toggleSelected(app.id)
  }

  function selectAll(): void {
    apps.setAllSelected(true)
  }

  function clearSelection(): void {
    apps.clearSelection()
  }

  async function runBatch(action: AppBatchAction): Promise<void> {
    const ids = [...apps.selectedIds]
    if (ids.length === 0) return
    if (action === 'delete') {
      const accepted = await notifications.confirm({
        title: '批量移除条目',
        message: `确定从目录中移除选中的 ${ids.length} 个条目吗？`,
        detail: '这是软删除：数据保留，后续扫描可能重新发现这些条目。',
        confirmText: '移除条目',
        cancelText: '取消',
        tone: 'danger',
      })
      if (!accepted) return
    }
    if (action === 'refresh_icon' && ids.length > MAX_BATCH_ICON_REFRESH) {
      notifications.warn(`一次最多重扫 ${MAX_BATCH_ICON_REFRESH} 个图标，请减少选择`)
      return
    }
    try {
      const results = await apps.batch(action, ids)
      const failed = results.filter((item) => !item.ok)
      if (failed.length === 0) {
        notifications.info(`已${BATCH_LABELS[action]} ${results.length} 个条目`)
      } else {
        notifications.warn(
          `已${BATCH_LABELS[action]} ${results.length - failed.length} 个条目，${failed.length} 个失败：${
            failed[0]?.message ?? '未知原因'
          }`,
        )
      }
    } catch (caught) {
      notifications.error(toAppError(caught).message)
    }
  }

  return { toggleSelect, selectAll, clearSelection, runBatch }
}
