import { ref } from 'vue'
import { useAppsStore } from '@/stores/apps'
import type { AppItem } from '@/api/types'

/**
 * 图标弹窗的目标与回流（§4.34）。
 *
 * 候选文件列表与手动路径由 AppIconModal 自己加载；这里只维护「当前正在换图标的条目」，
 * 并在图标重新解析成功后让列表拿到新的版本号。
 */
export function useAppIcons() {
  const apps = useAppsStore()
  const iconTarget = ref<AppItem | null>(null)

  /** 换图标弹窗：图标成功刷新后同步列表里的版本号，避免浏览器缓存旧图 */
  function openIcon(app: AppItem): void {
    iconTarget.value = app
  }

  function closeIcon(): void {
    iconTarget.value = null
  }

  function onIconRefreshed(id: string): void {
    if (iconTarget.value?.id === id) {
      iconTarget.value = { ...iconTarget.value, has_icon: true, icon_updated_at: Date.now() / 1000 }
    }
    void apps.reload()
  }

  return { iconTarget, openIcon, closeIcon, onIconRefreshed }
}
