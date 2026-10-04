import { computed, ref } from 'vue'
import { useAppsStore } from '@/stores/apps'
import type { AppItem } from '@/api/types'

/** 详情抽屉的目标条目：打开、关闭，以及从当前列表里取最新的一份。 */
export function useAppDetail() {
  const apps = useAppsStore()
  const detailTarget = ref<AppItem | null>(null)

  /**
   * 详情抽屉。只记 id，再按 id 从当前列表里取最新的一份：写操作（改名、常用、
   * 隐藏）都会重新拉取目录，抽屉里的内容要跟着变，而不是停留在打开时的快照。
   */
  function openDetail(app: AppItem): void {
    detailTarget.value = app
  }

  function closeDetail(): void {
    detailTarget.value = null
  }

  const detailApp = computed(() => {
    const id = detailTarget.value?.id
    if (!id) return null
    return apps.items.find((item) => item.id === id) ?? detailTarget.value
  })

  return { detailTarget, detailApp, openDetail, closeDetail }
}
