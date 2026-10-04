<script setup lang="ts">
import Button from './Button.vue'
import { formatCount } from '@/utils/format'

const props = defineProps<{
  page: number
  pageCount: number
  total: number
  rangeStart: number
  rangeEnd: number
  hasPrev: boolean
  hasNext: boolean
  loading?: boolean
  unit?: string
}>()

const emit = defineEmits<{
  (event: 'prev'): void
  (event: 'next'): void
}>()
</script>

<template>
  <div class="pager">
    <span class="pager__summary">
      共 {{ formatCount(props.total) }}{{ props.unit ?? '条' }}
      <template v-if="props.total > 0">· 当前 {{ props.rangeStart }}–{{ props.rangeEnd }}</template>
    </span>
    <div class="pager__controls">
      <Button size="sm" variant="ghost" :disabled="!props.hasPrev || props.loading" @click="emit('prev')">
        上一页
      </Button>
      <span class="pager__page">{{ props.page }} / {{ props.pageCount }}</span>
      <Button size="sm" variant="ghost" :disabled="!props.hasNext || props.loading" @click="emit('next')">
        下一页
      </Button>
    </div>
  </div>
</template>

<style scoped>
.pager {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-5);
  border-top: 1px solid var(--border-1);
  flex-wrap: wrap;
}

.pager__summary {
  font-size: var(--font-sm);
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
}

.pager__controls {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.pager__page {
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  color: var(--text-2);
  min-width: 56px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
</style>
