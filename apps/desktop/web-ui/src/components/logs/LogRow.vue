<script setup lang="ts">
import { computed, ref } from 'vue'
import type { LogItem } from '@/api/types'
import { LOG_LEVEL_MAP } from '@/utils/constants'
import { formatLogTime, orDash, truncate } from '@/utils/format'
import Chip from '@/components/ui/Chip.vue'

const props = defineProps<{
  item: LogItem
  wrap: boolean
}>()

const expanded = ref(false)

const meta = computed(() => LOG_LEVEL_MAP[props.item.level] ?? LOG_LEVEL_MAP.INFO)
const location = computed(() => {
  const file = props.item.file
  const line = props.item.line
  if (!file) return '—'
  return line === null ? file : `${file}:${line}`
})
const thread = computed(() => orDash(props.item.thread_name))
/** 单行模式下的截断阈值：足够长以便横向滚动查看，又不至于撑爆布局 */
const singleLineMessage = computed(() => truncate(props.item.message, 2_000))
const isTruncated = computed(() => singleLineMessage.value.length !== props.item.message.length)
</script>

<template>
  <li class="log" :class="{ 'log--expanded': expanded }">
    <div class="log__head">
      <span class="log__time">{{ formatLogTime(props.item) }}</span>
      <Chip size="sm" mono :tone="meta.tone">{{ meta.label }}</Chip>
      <span class="log__target" :title="props.item.target">{{ props.item.target }}</span>
      <span class="log__thread">{{ thread }}</span>
      <span class="log__location" :title="location">{{ location }}</span>
      <button type="button" class="log__toggle" @click="expanded = !expanded">
        {{ expanded ? '收起' : '详情' }}
      </button>
    </div>

    <p class="log__message" :class="props.wrap ? 'log__message--wrap' : 'log__message--nowrap'">
      {{ expanded ? props.item.message : singleLineMessage }}
    </p>

    <div v-if="expanded" class="log__detail">
      <span>id {{ props.item.id }}</span>
      <span>Unix {{ props.item.timestamp }}s</span>
      <span>线程 {{ thread }}</span>
      <span>位置 {{ location }}</span>
      <span v-if="isTruncated" class="log__detail-warn">单行视图已截断，详情为完整内容</span>
    </div>
  </li>
</template>

<style scoped>
.log {
  border-bottom: 1px solid var(--border-1);
  padding: var(--space-2) var(--space-4);
  transition: background var(--transition-fast);
}

.log:hover {
  background: var(--surface-2);
}

.log:last-child {
  border-bottom: none;
}

.log__head {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.log__time {
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  color: var(--text-2);
  flex: none;
}

.log__target {
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 320px;
}

.log__thread,
.log__location {
  font-family: var(--font-mono);
  font-size: var(--font-xs);
  color: var(--text-3);
}

.log__toggle {
  margin-left: auto;
  border: none;
  background: transparent;
  color: var(--text-3);
  font-size: var(--font-xs);
  cursor: pointer;
  padding: 0;
}

.log__toggle:hover {
  color: var(--accent);
}

.log__message {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  color: var(--text-1);
  line-height: 1.55;
}

.log__message--wrap {
  white-space: pre-wrap;
  word-break: break-word;
}

.log__message--nowrap {
  white-space: pre;
  overflow-x: auto;
}

.log__detail {
  display: flex;
  gap: var(--space-4);
  flex-wrap: wrap;
  margin-top: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  border: 1px solid var(--border-1);
  font-family: var(--font-mono);
  font-size: var(--font-xs);
  color: var(--text-3);
}

.log__detail-warn {
  color: var(--warning);
}
</style>
