<script setup lang="ts">
import { computed } from 'vue'
import type { AppBatchAction, AppsStats, CatalogKind } from '@/api/types'
import { formatCount } from '@/utils/format'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import SelectField from '@/components/ui/SelectField.vue'
import Spinner from '@/components/ui/Spinner.vue'

const props = defineProps<{
  kind: CatalogKind
  query: string
  includeHidden: boolean
  /** 只看「常用（我的库）」 */
  favoriteOnly: boolean
  hiddenVisibleCount: number
  stats: AppsStats | null
  scanning: boolean
  loading: boolean
  scanBusy: boolean
  scanError: string | null
  /** 已勾选条数（0 表示批量操作条不出现） */
  selectedCount: number
  /** 批量请求进行中 */
  batchBusy: boolean
  /** 服务是否支持新增条目（capabilities.app_add） */
  canAdd: boolean
}>()

const emit = defineEmits<{
  (event: 'update:kind', value: CatalogKind): void
  (event: 'update:query', value: string): void
  (event: 'update:includeHidden', value: boolean): void
  (event: 'update:favoriteOnly', value: boolean): void
  (event: 'scan'): void
  (event: 'reload'): void
  (event: 'add'): void
  (event: 'batch', action: AppBatchAction): void
  (event: 'select-all'): void
  (event: 'clear-selection'): void
}>()

const kindOptions = [
  { value: 'app', label: '应用' },
  { value: 'game', label: '游戏' },
]

const kindStats = computed(() => (props.stats ? props.stats[props.kind] : null))

function onSearchInput(event: Event): void {
  emit('update:query', (event.target as HTMLInputElement).value)
}

function toggleHidden(): void {
  emit('update:includeHidden', !props.includeHidden)
}

function toggleFavoriteOnly(): void {
  emit('update:favoriteOnly', !props.favoriteOnly)
}
</script>

<template>
  <div class="toolbar">
    <div class="toolbar__row">
      <div class="search">
        <svg class="search__icon" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
          <path
            d="M7 2.5a4.5 4.5 0 1 0 2.7 8.1l3.1 3.1 1-1-3.1-3.1A4.5 4.5 0 0 0 7 2.5Zm0 1.4a3.1 3.1 0 1 1 0 6.2 3.1 3.1 0 0 1 0-6.2Z"
            fill="currentColor"
          />
        </svg>
        <input
          class="search__input"
          type="search"
          :value="props.query"
          placeholder="搜索名称、原始名或分类…"
          aria-label="搜索应用目录"
          @input="onSearchInput"
        />
        <Spinner v-if="props.loading" :size="12" tone="muted" />
      </div>

      <SelectField
        inline
        :width="104"
        :model-value="props.kind"
        :options="kindOptions"
        :disabled="props.loading"
        @update:model-value="emit('update:kind', $event as CatalogKind)"
      />

      <Button size="sm" variant="ghost" @click="toggleHidden">
        {{ props.includeHidden ? '隐藏项可见' : '隐藏项过滤中' }}
      </Button>

      <Button
        size="sm"
        :variant="props.favoriteOnly ? 'primary' : 'ghost'"
        title="只看「常用」，与手机端「库 → 常用」是同一份数据"
        @click="toggleFavoriteOnly"
      >
        {{ props.favoriteOnly ? '★ 仅常用' : '☆ 仅看常用' }}
      </Button>

      <Button
        size="sm"
        variant="secondary"
        :disabled="!props.canAdd"
        :title="props.canAdd ? '添加自定义应用或游戏' : '当前服务不支持新增条目，请修复服务（升级）'"
        @click="emit('add')"
      >
        新增条目
      </Button>

      <Button size="sm" variant="primary" :loading="props.scanBusy || props.scanning" @click="emit('scan')">
        {{ props.scanning ? '扫描进行中…' : '重新扫描' }}
      </Button>

      <Button size="sm" variant="ghost" @click="emit('reload')">刷新</Button>
    </div>

    <div v-if="props.selectedCount > 0" class="toolbar__row toolbar__row--batch">
      <Chip size="sm" tone="accent">已选 {{ formatCount(props.selectedCount) }} 条</Chip>
      <Button size="sm" variant="ghost" :disabled="props.batchBusy" @click="emit('select-all')">全选本页</Button>
      <Button size="sm" variant="secondary" :disabled="props.batchBusy" @click="emit('batch', 'hide')">批量隐藏</Button>
      <Button size="sm" variant="secondary" :disabled="props.batchBusy" @click="emit('batch', 'show')">批量显示</Button>
      <Button size="sm" variant="secondary" :disabled="props.batchBusy" @click="emit('batch', 'restore_name')">
        恢复原名
      </Button>
      <Button
        size="sm"
        variant="secondary"
        :disabled="props.batchBusy"
        title="对选中的条目逐个重新解析图标（最多 50 条）"
        @click="emit('batch', 'refresh_icon')"
      >
        重扫图标
      </Button>
      <Button size="sm" variant="danger-ghost" :disabled="props.batchBusy" @click="emit('batch', 'delete')">
        批量删除
      </Button>
      <Button size="sm" variant="ghost" :disabled="props.batchBusy" @click="emit('clear-selection')">取消选择</Button>
      <Spinner v-if="props.batchBusy" :size="12" />
    </div>

    <div class="toolbar__row toolbar__row--meta">
      <span class="toolbar__summary">
        本分类 {{ formatCount(kindStats?.total ?? 0) }} 条 · 常用 {{ formatCount(kindStats?.favorite ?? 0) }} 条 ·
        隐藏 {{ formatCount(kindStats?.hidden ?? 0) }} 条 ·
        已重命名 {{ formatCount(kindStats?.renamed ?? 0) }} 条
      </span>
      <Chip v-if="props.hiddenVisibleCount > 0" size="sm" tone="warning">
        当前页 {{ formatCount(props.hiddenVisibleCount) }} 条隐藏项
      </Chip>
      <span v-if="props.scanning" class="toolbar__scan-time">
        <Spinner :size="11" />
        服务端正在后台扫描…
      </span>
    </div>

    <p v-if="props.scanError" class="toolbar__error">{{ props.scanError }}</p>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--border-1);
}

.toolbar__row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.toolbar__row--meta {
  gap: var(--space-2);
}

/* 批量操作条：有勾选时才出现，用柔和强调底色与常规工具条区分开 */
.toolbar__row--batch {
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-md);
  border: 1px solid var(--accent-border);
  background: var(--accent-soft);
}

.toolbar__summary {
  font-size: var(--font-sm);
  color: var(--text-3);
}

.search {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  height: var(--control-height);
  padding: 0 var(--space-3);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-1);
  flex: 1 1 260px;
  min-width: 200px;
  max-width: 420px;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}

.search:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.search__icon {
  color: var(--text-3);
  flex: none;
}

.search__input {
  flex: 1 1 auto;
  min-width: 0;
  border: none;
  background: transparent;
  color: var(--text-1);
  height: 100%;
  outline: none;
}

.search__input::placeholder {
  color: var(--text-3);
}

.toolbar__scan-time {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
  font-size: var(--font-sm);
  color: var(--text-3);
}

.toolbar__error {
  font-size: var(--font-sm);
  color: var(--error);
}
</style>
