<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { MAX_BUFFERED_LOGS, useLogsStore } from '@/stores/logs'
import { useNotificationsStore } from '@/stores/notifications'
import { useChannelSubscription } from '@/composables/useEventChannel'
import { toAppError } from '@/utils/errors'
import { formatBytes, formatCount, formatRelative } from '@/utils/format'
import type { LogLevelFilter, LogSource } from '@/api/types'
import type { TimeRangeValue } from '@/utils/constants'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import SelectField from '@/components/ui/SelectField.vue'
import Spinner from '@/components/ui/Spinner.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import LoadingBlock from '@/components/ui/LoadingBlock.vue'
import ErrorLine from '@/components/ui/ErrorLine.vue'
import LogToolbar from '@/components/logs/LogToolbar.vue'
import LogRow from '@/components/logs/LogRow.vue'

const logs = useLogsStore()
const notifications = useNotificationsStore()
const { items, filters, loading, loadingMore, stats, total, error } = storeToRefs(logs)

const viewport = ref<HTMLElement | null>(null)
const wrap = ref(true)
const autoFollow = ref(true)
const showFilters = ref(false)
const localSearch = ref(filters.value.search)
const localTarget = ref(filters.value.target)
const exporting = ref(false)

const sourceOptions: { value: LogSource; label: string }[] = [
  { value: 'desktop', label: '桌面端（实时）' },
  { value: 'service', label: 'QingLinkService（磁盘）' },
]

/** 服务端来源是磁盘快照：没有实时流，也没有内存缓冲，清空按钮对它无意义。 */
const isServiceSource = computed(() => logs.source === 'service')
const activeItems = computed(() => (isServiceSource.value ? logs.serviceItems : items.value))
const activeTotal = computed(() => (isServiceSource.value ? logs.serviceTotal : total.value))
const activeLoading = computed(() => (isServiceSource.value ? logs.serviceLoading : loading.value))
const activeLoadingMore = computed(() =>
  isServiceSource.value ? logs.serviceLoadingMore : loadingMore.value,
)
const activeHasMore = computed(() =>
  isServiceSource.value ? logs.serviceNextOffset !== null : logs.hasMore,
)

const sourceFiles = computed(() => {
  const files = logs.files
  if (!files) return []
  return isServiceSource.value ? files.service : files.desktop
})

const serviceFileOptions = computed(() => [
  ...sourceFiles.value.map((file) => ({
    value: file.name,
    label: `${file.name} · ${formatBytes(file.size_bytes)}`,
  })),
])

useChannelSubscription(['logs'])

onMounted(async () => {
  logs.setStreaming(true)
  await Promise.all([logs.load(), logs.loadStats(), logs.loadFiles()])
  await nextTick()
  bindScroll()
})

onBeforeUnmount(() => {
  logs.setStreaming(false)
  unbindScroll()
})

function bindScroll(): void {
  const element = viewport.value
  if (!element) return
  element.addEventListener('scroll', onScroll, { passive: true })
  onScroll()
}

function unbindScroll(): void {
  viewport.value?.removeEventListener('scroll', onScroll)
}

function onScroll(): void {
  const element = viewport.value
  if (!element) return
  const atTop = element.scrollTop <= 6
  logs.setAtTop(atTop)
  if (atTop) autoFollow.value = true
  if (element.scrollHeight - element.scrollTop - element.clientHeight <= 120) loadMore()
}

// 新日志到达时：只有自动跟随开启且用户停在顶部才滚动到顶部（§5.4）
watch(
  () => items.value.length,
  async (length, previous) => {
    if (!autoFollow.value || !logs.atTop) return
    if (previous === undefined || length <= previous) return
    await nextTick()
    viewport.value?.scrollTo({ top: 0 })
  },
)

const hasNewBadge = computed(() => logs.pendingCount > 0 && !logs.atTop)

function scrollToTop(): void {
  viewport.value?.scrollTo({ top: 0, behavior: 'smooth' })
  logs.acknowledgeNew()
  autoFollow.value = true
}

function onLevelChange(value: LogLevelFilter | null): void {
  void logs.applyFilters({ level: value })
}

function onRangeChange(value: TimeRangeValue): void {
  void logs.applyFilters({ range: value })
}

function onSinceChange(value: number | null): void {
  void logs.applyFilters({ since: value })
}

function onUntilChange(value: number | null): void {
  void logs.applyFilters({ until: value })
}

function applyFilters(): void {
  void logs.applyFilters({ search: localSearch.value, target: localTarget.value })
}

function resetFilters(): void {
  localSearch.value = ''
  localTarget.value = ''
  void logs.resetFilters()
}

async function clearLogs(): Promise<void> {
  const accepted = await notifications.confirm({
    title: '清空日志',
    message: `确定清空全部日志记录吗？当前命中 ${formatCount(total.value)} 条。`,
    detail: '清空内存缓冲；磁盘上的日志文件不会被删除，仍可下载。',
    confirmText: '清空日志',
    cancelText: '取消',
    tone: 'danger',
  })
  if (!accepted) return
  try {
    const cleared = await logs.clear()
    notifications.info(`已清空 ${formatCount(cleared)} 条日志记录`)
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
}

async function download(): Promise<void> {
  try {
    // 服务端来源显式指定当前文件；桌面端沿用「最新一份」的推断。
    await logs.downloadFrom(logs.source, isServiceSource.value ? (logs.serviceFile ?? undefined) : undefined)
    notifications.info('已开始下载磁盘日志文件')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
}

async function exportBundle(): Promise<void> {
  exporting.value = true
  try {
    await logs.downloadDiagnosticsBundle()
    notifications.info('诊断包已开始下载')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  } finally {
    exporting.value = false
  }
}

function changeSource(value: string): void {
  logs.setSource(value as LogSource)
}

function loadMore(): void {
  if (isServiceSource.value) {
    void logs.loadServiceMore()
    return
  }
  void logs.loadMore()
}
</script>

<template>
  <Card padding="none" :subtitle="`最新在前 · 命中 ${formatCount(activeTotal)} 条`">
    <template #actions>
      <Spinner v-if="activeLoading" :size="13" />
      <SelectField
        inline
        :width="220"
        :model-value="logs.source"
        :options="sourceOptions"
        @update:model-value="changeSource"
      />
      <Button size="sm" variant="secondary" @click="showFilters = !showFilters">
        {{ showFilters ? '收起筛选' : '筛选' }}
      </Button>
      <Button size="sm" variant="ghost" @click="download">下载</Button>
      <Button size="sm" variant="ghost" :loading="exporting" @click="exportBundle">导出诊断包</Button>
      <Button v-if="!isServiceSource" size="sm" variant="danger-ghost" @click="clearLogs">清空</Button>
    </template>

    <div class="source">
      <SelectField
        v-if="isServiceSource && serviceFileOptions.length > 0"
        inline
        :width="320"
        :model-value="logs.serviceFile ?? ''"
        :options="serviceFileOptions"
        @update:model-value="logs.setServiceFile($event)"
      />
      <span class="source__hint">
        {{
          isServiceSource
            ? 'QingLinkService 的磁盘日志：按需解析，不是实时流；更早的文件可在上方切换。'
            : '桌面端内存缓冲（最近 5000 条）+ 实时推送；更早的记录请下载磁盘文件。'
        }}
      </span>
    </div>

    <LogToolbar
      v-model:show-filters="showFilters"
      :level="filters.level"
      :target="localTarget"
      :search="localSearch"
      :range="filters.range"
      :custom-since="filters.since"
      :custom-until="filters.until"
      :wrap="wrap"
      :auto-follow="autoFollow"
      :loading="activeLoading"
      :stats="stats"
      :streaming="logs.streaming && !isServiceSource"
      :total="activeTotal"
      :buffered="MAX_BUFFERED_LOGS"
      @update:level="onLevelChange"
      @update:target="localTarget = $event"
      @update:search="localSearch = $event"
      @update:range="onRangeChange"
      @update:custom-since="onSinceChange"
      @update:custom-until="onUntilChange"
      @update:wrap="wrap = $event"
      @update:auto-follow="autoFollow = $event"
      @reload="applyFilters"
      @reset="resetFilters"
    />

    <div v-if="!isServiceSource && logs.droppedCount > 0" class="notice">
      <span>
        服务端丢弃了 <strong>{{ formatCount(logs.droppedCount) }}</strong> 条日志
        <template v-if="logs.droppedAt">（{{ formatRelative(logs.droppedAt) }}）</template>
        ，可缩小时间范围后重试。
      </span>
      <Button size="sm" variant="ghost" @click="logs.acknowledgeDropped()">知道了</Button>
    </div>

    <LoadingBlock v-if="activeLoading && activeItems.length === 0" label="正在查询日志…" />

    <EmptyState
      v-else-if="activeItems.length === 0"
      :title="logs.filtersActive ? '当前过滤条件没有命中日志' : '暂无日志记录'"
      :description="
        logs.filtersActive
          ? '可以放宽级别或清空时间范围后重试。'
          : isServiceSource
            ? '该服务端日志文件里还没有可解析的记录。'
            : '服务运行过程中产生的日志会实时出现在这里。'
      "
      :action-label="logs.filtersActive ? '重置过滤条件' : undefined"
      @action="resetFilters"
    />

    <div v-else ref="viewport" class="viewport">
      <ul class="log-list">
        <LogRow v-for="item in activeItems" :key="item.id" :item="item" :wrap="wrap" />
      </ul>

      <div v-if="activeHasMore" class="load-more">
        <Button size="sm" :loading="activeLoadingMore" @click="loadMore">加载更早的日志</Button>
      </div>
    </div>

    <button
      v-if="!isServiceSource && hasNewBadge"
      type="button"
      class="new-badge"
      @click="scrollToTop"
    >
      <span class="new-badge__dot" aria-hidden="true" />
      有 {{ formatCount(logs.pendingCount) }} 条新日志 · 回到顶部
    </button>
  </Card>

  <ErrorLine :error="error" />
</template>

<style scoped>
.source {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--border-1);
  flex-wrap: wrap;
}

.source__hint {
  font-size: var(--font-sm);
  color: var(--text-3);
}

.notice {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-2) var(--space-4);
  font-size: var(--font-sm);
  border-bottom: 1px solid var(--border-1);
  background: var(--warning-soft);
  color: var(--text-1);
}

.notice strong {
  font-family: var(--font-mono);
}

.viewport {
  max-height: calc(100vh - 320px);
  min-height: 260px;
  overflow-y: auto;
  position: relative;
}

.log-list {
  display: flex;
  flex-direction: column;
}

.load-more {
  display: flex;
  justify-content: center;
  padding: var(--space-4);
  border-top: 1px solid var(--border-1);
}

.new-badge {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  align-self: center;
  margin: var(--space-3) auto;
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-pill);
  border: 1px solid var(--accent);
  background: var(--surface-2);
  box-shadow: var(--shadow-pop);
  color: var(--accent);
  font-size: var(--font-sm);
  cursor: pointer;
}

.new-badge__dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent-bright);
}
</style>
