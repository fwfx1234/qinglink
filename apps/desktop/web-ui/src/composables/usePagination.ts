import { computed, type Ref } from 'vue'

export interface PaginationState {
  offset: Ref<number>
  limit: Ref<number>
  total: Ref<number>
  nextOffset: Ref<number | null>
  goToPage: (page: number) => void | Promise<void>
  nextPage: () => void | Promise<void>
  prevPage: () => void | Promise<void>
}

export interface PaginationView {
  page: Ref<number>
  pageCount: Ref<number>
  hasPrev: Ref<boolean>
  hasNext: Ref<boolean>
  rangeStart: Ref<number>
  rangeEnd: Ref<number>
}

/**
 * 分页展示计算：把 store 的 offset/limit/total 转成界面需要的页码与区间文案。
 * 分页响应统一形状见契约 §2.3。
 */
export function usePagination(state: PaginationState): PaginationView {
  const page = computed(() => Math.floor(state.offset.value / state.limit.value) + 1)
  const pageCount = computed(() => Math.max(1, Math.ceil(state.total.value / state.limit.value)))
  const hasPrev = computed(() => state.offset.value > 0)
  const hasNext = computed(() => state.nextOffset.value !== null)
  const rangeStart = computed(() => (state.total.value === 0 ? 0 : state.offset.value + 1))
  const rangeEnd = computed(() => Math.min(state.offset.value + state.limit.value, state.total.value))

  return { page, pageCount, hasPrev, hasNext, rangeStart, rangeEnd }
}
