import { ref } from 'vue'
import { useAppsStore } from '@/stores/apps'
import { useDebouncedRef } from '@/composables/useDebouncedRef'
import type { CatalogKind } from '@/api/types'

/**
 * 目录的筛选与搜索：分类、隐藏项、仅常用，以及带防抖的关键词输入。
 * 条件变更后由 store 重置分页并重新拉取。
 */
export function useAppsFilters() {
  const apps = useAppsStore()

  const searchDraft = ref(apps.query)
  const debounced = useDebouncedRef(searchDraft, 280, (value) => {
    void apps.setQuery(value)
  })

  /** 空状态里的「清空搜索」：立即提交一次，不等防抖窗口 */
  function clearSearch(): void {
    searchDraft.value = ''
    debounced.flush()
  }

  function onKindChange(value: CatalogKind): void {
    void apps.setKind(value)
  }

  function onIncludeHiddenChange(value: boolean): void {
    void apps.setIncludeHidden(value)
  }

  function onFavoriteOnlyChange(value: boolean): void {
    void apps.setFavoriteOnly(value)
  }

  return {
    searchDraft,
    clearSearch,
    onKindChange,
    onIncludeHiddenChange,
    onFavoriteOnlyChange,
  }
}
