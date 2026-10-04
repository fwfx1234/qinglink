import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppsStore } from '@/stores/apps'
import { useNotificationsStore } from '@/stores/notifications'
import { usePagination } from '@/composables/usePagination'
import { toAppError } from '@/utils/errors'

/**
 * 应用 / 游戏目录的数据侧（§4.14 – §4.20）：加载、刷新、分页与触发扫描。
 *
 * 筛选条件（关键词、分类、隐藏项、仅常用）由 useAppsFilters 写入 store，
 * 这里只消费 store 里的查询结果。
 */
export function useAppsCatalog() {
  const apps = useAppsStore()
  const notifications = useNotificationsStore()
  const { kind, items, offset, limit, total, nextOffset, loading, error } = storeToRefs(apps)

  const scanBusy = ref(false)
  const scanError = ref<string | null>(null)

  const { page, pageCount, hasPrev, hasNext, rangeStart, rangeEnd } = usePagination({
    offset,
    limit,
    total,
    nextOffset,
    goToPage: (value) => apps.goToPage(value),
    nextPage: () => apps.nextPage(),
    prevPage: () => apps.prevPage(),
  })

  const subtitle = computed(() => `共 ${total.value} 条匹配记录 · 第 ${page.value} / ${pageCount.value} 页`)

  onMounted(() => {
    void apps.load()
  })

  async function runScan(): Promise<void> {
    scanBusy.value = true
    scanError.value = null
    try {
      const accepted = await apps.scan(apps.kind)
      if (accepted) {
        notifications.info('已触发重新扫描，完成后目录会自动刷新')
      }
    } catch (caught) {
      const appError = toAppError(caught)
      scanError.value = appError.conflict
        ? '扫描进行中或检测到游戏正在运行，已延后；请稍后再试。'
        : appError.message
      notifications.warn(scanError.value)
    } finally {
      scanBusy.value = false
    }
  }

  function reload(): void {
    void apps.reload()
  }

  return {
    kind,
    items,
    offset,
    limit,
    total,
    nextOffset,
    loading,
    error,
    scanBusy,
    scanError,
    page,
    pageCount,
    hasPrev,
    hasNext,
    rangeStart,
    rangeEnd,
    subtitle,
    runScan,
    reload,
  }
}
