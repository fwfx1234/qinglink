import { nextTick, ref } from 'vue'
import { useAppsStore } from '@/stores/apps'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import type { AppItem } from '@/api/types'

/**
 * 单个条目的行内动作（§4.16 – §4.19、§4.32）：常用、首页置顶、隐藏、移除、启动、
 * 恢复原名，以及「重命名」弹窗的目标与提交。
 *
 * 写操作的成功路径是服务端广播 catalog_changed 后回流刷新，这里负责调用与提示。
 */
export function useAppRowActions() {
  const apps = useAppsStore()
  const notifications = useNotificationsStore()

  const renameTarget = ref<AppItem | null>(null)
  const renameBusy = ref(false)

  async function toggleFavorite(app: AppItem): Promise<void> {
    try {
      await apps.setFavorite(app.id, !app.favorite)
      notifications.info(app.favorite ? `已把「${app.name}」移出常用` : `已把「${app.name}」加入常用`)
    } catch (caught) {
      notifications.error(toAppError(caught).message)
    }
  }

  async function togglePinned(app: AppItem): Promise<void> {
    try {
      await apps.setPinned(app.id, !app.pinned)
      notifications.info(app.pinned ? `已取消「${app.name}」的首页置顶` : `已把「${app.name}」置顶到手机首页`)
    } catch (caught) {
      notifications.error(toAppError(caught).message)
    }
  }

  async function toggleHidden(app: AppItem): Promise<void> {
    try {
      await apps.setHidden(app.id, !app.hidden)
    } catch (caught) {
      notifications.error(toAppError(caught).message)
    }
  }

  async function removeApp(app: AppItem): Promise<void> {
    const accepted = await notifications.confirm({
      title: '从目录中移除',
      message: `确定移除「${app.name}」吗？`,
      detail: `id: ${app.id}\n这是软删除：数据保留，后续扫描可能重新发现该条目。`,
      confirmText: '移除条目',
      cancelText: '取消',
      tone: 'danger',
    })
    if (!accepted) return
    try {
      await apps.remove(app.id)
    } catch (caught) {
      notifications.error(toAppError(caught).message)
    }
  }

  async function launchApp(app: AppItem): Promise<void> {
    try {
      await apps.launch(app.id)
      notifications.info(`已请求在本机启动「${app.name}」`)
    } catch (caught) {
      notifications.error(toAppError(caught).message)
    }
  }

  /** 恢复原名 = 清空显示名覆盖（§4.17 传空字符串） */
  async function restoreName(app: AppItem): Promise<void> {
    try {
      await apps.rename(app.id, '')
    } catch (caught) {
      notifications.error(toAppError(caught).message)
    }
  }

  function openRename(app: AppItem): void {
    renameTarget.value = app
  }

  function closeRename(): void {
    renameTarget.value = null
  }

  async function submitRename(payload: { id: string; name: string }): Promise<void> {
    renameBusy.value = true
    try {
      await apps.rename(payload.id, payload.name)
      renameTarget.value = null
      await nextTick()
    } catch (caught) {
      const appError = toAppError(caught)
      notifications.error(appError.serviceUnavailable ? '服务未连接，无法写入目录' : appError.message)
    } finally {
      renameBusy.value = false
    }
  }

  return {
    renameTarget,
    renameBusy,
    toggleFavorite,
    togglePinned,
    toggleHidden,
    removeApp,
    launchApp,
    restoreName,
    openRename,
    closeRename,
    submitRename,
  }
}
