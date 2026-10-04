<script setup lang="ts">
import { computed } from 'vue'
import { LOG_LEVEL_FILTERS, TIME_RANGE_PRESETS, type TimeRangeValue } from '@/utils/constants'
import {
  formatBytes,
  formatCount,
  formatRelative,
  formatUnixMillis,
  toDateTimeLocalValue,
  fromDateTimeLocalValue,
} from '@/utils/format'
import type { LogLevelFilter, LogsStats } from '@/api/types'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import SelectField from '@/components/ui/SelectField.vue'
import Spinner from '@/components/ui/Spinner.vue'

const props = defineProps<{
  /** 展开后显示高级筛选（模块 / 文本 / 自定义时间 / 排版开关） */
  showFilters: boolean
  level: LogLevelFilter | null
  target: string
  search: string
  range: TimeRangeValue
  customSince: number | null
  customUntil: number | null
  wrap: boolean
  autoFollow: boolean
  loading: boolean
  stats: LogsStats | null
  streaming: boolean
  total: number
  buffered: number
}>()

const emit = defineEmits<{
  (event: 'update:showFilters', value: boolean): void
  (event: 'update:level', value: LogLevelFilter | null): void
  (event: 'update:target', value: string): void
  (event: 'update:search', value: string): void
  (event: 'update:range', value: TimeRangeValue): void
  (event: 'update:customSince', value: number | null): void
  (event: 'update:customUntil', value: number | null): void
  (event: 'update:wrap', value: boolean): void
  (event: 'update:autoFollow', value: boolean): void
  (event: 'reload'): void
  (event: 'reset'): void
}>()

const levelOptions = LOG_LEVEL_FILTERS.map((item) => ({ value: item.value, label: item.label }))
const rangeOptions = TIME_RANGE_PRESETS.map((item) => ({ value: item.value, label: item.label }))

const levelModel = computed(() => props.level ?? 'trace')
const sinceValue = computed({
  get: () => toDateTimeLocalValue(props.customSince),
  set: (value: string) => emit('update:customSince', fromDateTimeLocalValue(value)),
})
const untilValue = computed({
  get: () => toDateTimeLocalValue(props.customUntil),
  set: (value: string) => emit('update:customUntil', fromDateTimeLocalValue(value)),
})

const levelCounts = computed(() => {
  const stats = props.stats
  return [
    { key: 'INFO', value: stats?.by_level.INFO ?? 0, tone: 'accent' as const },
    { key: 'WARN', value: stats?.by_level.WARN ?? 0, tone: 'warning' as const },
    { key: 'ERROR', value: stats?.by_level.ERROR ?? 0, tone: 'error' as const },
  ]
})

const files = computed(() => props.stats?.files ?? [])

function onLevelChange(value: string): void {
  emit('update:level', value === 'trace' ? null : (value as LogLevelFilter))
}

function onRangeChange(value: string): void {
  emit('update:range', value as TimeRangeValue)
}

function onTargetInput(event: Event): void {
  emit('update:target', (event.target as HTMLInputElement).value)
}

function onSearchInput(event: Event): void {
  emit('update:search', (event.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="ql-toolbar">
    <div class="ql-toolbar__row">
      <SelectField
        inline
        :width="148"
        :model-value="levelModel"
        :options="levelOptions"
        :disabled="props.loading"
        @update:model-value="onLevelChange"
      />

      <SelectField
        inline
        :width="140"
        :model-value="props.range"
        :options="rangeOptions"
        :disabled="props.loading"
        @update:model-value="onRangeChange"
      />

      <Chip v-for="entry in levelCounts" :key="entry.key" size="sm" :tone="entry.tone">
        {{ entry.key }} {{ formatCount(entry.value) }}
      </Chip>

      <span class="ql-hint summary">
        缓冲 {{ formatCount(props.stats?.buffer_len ?? 0) }} / {{ formatCount(props.stats?.buffer_capacity ?? 0) }}
        · 页内最多 {{ formatCount(props.buffered) }}
      </span>

      <span class="ql-hint state">
        <Spinner v-if="props.loading" :size="11" />
        <Chip size="sm" :tone="props.streaming ? 'success' : 'neutral'" dot>
          {{ props.streaming ? '实时订阅中' : '未订阅实时日志' }}
        </Chip>
      </span>
    </div>

    <template v-if="props.showFilters">
      <div class="ql-toolbar__row">
        <input
          class="ql-control ql-control--grow"
          type="text"
          placeholder="模块路径子串，例如 qinglink_desktop::web"
          aria-label="模块过滤"
          :value="props.target"
          :disabled="props.loading"
          @input="onTargetInput"
        />
        <input
          class="ql-control ql-control--grow"
          type="search"
          placeholder="消息文本搜索"
          aria-label="文本搜索"
          :value="props.search"
          :disabled="props.loading"
          @input="onSearchInput"
        />
      </div>

      <div class="ql-toolbar__row">
        <input
          class="ql-control"
          type="datetime-local"
          aria-label="起始时间"
          :value="sinceValue"
          :disabled="props.loading"
          @input="sinceValue = ($event.target as HTMLInputElement).value"
        />
        <span class="tilde">→</span>
        <input
          class="ql-control"
          type="datetime-local"
          aria-label="结束时间"
          :value="untilValue"
          :disabled="props.loading"
          @input="untilValue = ($event.target as HTMLInputElement).value"
        />

        <Button size="sm" variant="primary" :loading="props.loading" @click="emit('reload')">应用过滤</Button>
        <Button size="sm" variant="ghost" @click="emit('reset')">重置</Button>
        <Button size="sm" variant="ghost" @click="emit('update:wrap', !props.wrap)">
          {{ props.wrap ? '改为单行' : '改为换行' }}
        </Button>
        <Button size="sm" variant="ghost" @click="emit('update:autoFollow', !props.autoFollow)">
          {{ props.autoFollow ? '自动跟随：开' : '自动跟随：关' }}
        </Button>
      </div>

      <div v-if="files.length > 0" class="ql-toolbar__row">
        <span class="ql-hint">磁盘日志文件：</span>
        <Chip
          v-for="file in files"
          :key="file.name"
          size="sm"
          mono
          tone="neutral"
          :title="`修改于 ${formatUnixMillis(file.modified_secs * 1000)}`"
        >
          {{ file.name }} · {{ formatBytes(file.size_bytes) }}
        </Chip>
        <span v-if="props.stats?.newest_ms" class="ql-hint">
          最新 {{ formatRelative(props.stats.newest_ms) }}
        </span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.summary {
  margin-left: var(--space-2);
}

.state {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
}

.tilde {
  color: var(--text-3);
  font-family: var(--font-mono);
}
</style>
