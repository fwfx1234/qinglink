<script setup lang="ts">
import { computed } from 'vue'
import type { AppItem } from '@/api/types'
import { tailPath } from '@/utils/format'
import AppIcon from './AppIcon.vue'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import Spinner from '@/components/ui/Spinner.vue'
import DataRow from '@/components/ui/DataRow.vue'
import RowMenu, { type RowMenuItem } from '@/components/ui/RowMenu.vue'

const props = defineProps<{
  app: AppItem
  busy: boolean
  /** 多选中的状态 */
  selected: boolean
  /** 服务是否支持启动条目（capabilities.app_launch） */
  canLaunch: boolean
  /** 服务是否支持单独重扫图标（capabilities.app_icon_manage） */
  canManageIcon: boolean
}>()

const emit = defineEmits<{
  (event: 'rename', app: AppItem): void
  (event: 'toggle-hidden', app: AppItem): void
  (event: 'toggle-favorite', app: AppItem): void
  (event: 'toggle-pinned', app: AppItem): void
  (event: 'delete', app: AppItem): void
  (event: 'launch', app: AppItem): void
  (event: 'restore-name', app: AppItem): void
  (event: 'toggle-select', app: AppItem): void
  (event: 'icon', app: AppItem): void
  (event: 'detail', app: AppItem): void
}>()

/**
 * 行里只放路径尾部两段：完整路径留给详情面板。
 * 保留末段是为了能一眼区分同名 exe（例如多个 `launcher.exe`），
 * 前面的目录层级对用户几乎没有信息量。
 */
const path = computed(() => tailPath(props.app.exe_path || props.app.id, 2))

const metaLabel = computed(() => {
  const parts = [props.app.source || '未知来源', props.app.category || '未分类']
  if (props.app.args.length > 0) parts.push(`${props.app.args.length} 个启动参数`)
  return parts.join(' · ')
})

/**
 * 图标可能不对的条目：要么根本没有图标，要么只拿到了系统默认图标。
 * 这类条目的图标必须支持单独重扫，否则用户只能被错误图标一直误导。
 */
const iconSuspect = computed(
  () => !props.app.has_icon || props.app.icon_source === 'shell' || props.app.icon_source === 'none',
)

/** 低频动作收进「⋯」，行上只留启动、常用与详情。 */
const menuItems = computed<RowMenuItem[]>(() => {
  const items: RowMenuItem[] = [
    { id: 'rename', label: '重命名' },
    { id: 'icon', label: '更换图标', disabled: !props.canManageIcon },
    { id: 'toggle-hidden', label: props.app.hidden ? '取消隐藏' : '隐藏条目' },
  ]
  if (props.app.favorite) {
    items.push({
      id: 'toggle-pinned',
      label: props.app.pinned ? '取消首页置顶' : '置顶到手机首页',
    })
  }
  if (props.app.renamed) {
    items.push({ id: 'restore-name', label: '恢复原名' })
  }
  items.push({ id: 'delete', label: '从目录中移除', danger: true })
  return items
})

function onMenuSelect(id: string): void {
  switch (id) {
    case 'rename':
      return emit('rename', props.app)
    case 'icon':
      return emit('icon', props.app)
    case 'toggle-hidden':
      return emit('toggle-hidden', props.app)
    case 'toggle-pinned':
      return emit('toggle-pinned', props.app)
    case 'restore-name':
      return emit('restore-name', props.app)
    case 'delete':
      return emit('delete', props.app)
  }
}
</script>

<template>
  <DataRow
    :muted="props.app.hidden"
    :selected="props.selected"
    clickable
    @activate="emit('detail', props.app)"
  >
    <template #leading>
      <span class="select">
        <input
          type="checkbox"
          :checked="props.selected"
          :aria-label="`选择 ${props.app.name || props.app.id}`"
          @change="emit('toggle-select', props.app)"
        />
      </span>
      <AppIcon
        :id="props.app.id"
        :has-icon="props.app.has_icon"
        :version="props.app.icon_updated_at"
        :name="props.app.name"
        :size="36"
      />
    </template>

    <div class="title">
      <span class="title__name" :title="props.app.name">{{ props.app.name || '（未命名）' }}</span>
      <Chip v-if="props.app.favorite" size="sm" tone="accent">常用</Chip>
      <Chip v-if="props.app.pinned" size="sm" tone="accent">首页置顶</Chip>
      <Chip v-if="props.app.hidden" size="sm" tone="warning">已隐藏</Chip>
      <Chip v-if="props.app.renamed" size="sm" tone="accent">已重命名</Chip>
      <Chip v-if="props.app.user_added" size="sm" tone="neutral">手动添加</Chip>
      <Chip v-if="iconSuspect" size="sm" tone="warning" title="图标可能不准确，可在详情里单独重扫">
        图标待确认
      </Chip>
    </div>

    <p class="sub">
      <span class="sub__path" :title="props.app.exe_path || props.app.id">{{ path }}</span>
      <span class="sub__dot" aria-hidden="true">·</span>
      <span class="sub__meta">{{ metaLabel }}</span>
    </p>

    <template #actions>
      <Spinner v-if="props.busy" :size="12" tone="muted" />
      <Button size="sm" variant="ghost" :disabled="props.busy" title="查看完整路径与属性" @click="emit('detail', props.app)">
        详情
      </Button>
      <button
        type="button"
        class="star"
        :class="{ 'star--on': props.app.favorite }"
        :disabled="props.busy"
        :aria-pressed="props.app.favorite"
        :title="props.app.favorite ? '从常用中移出' : '加入常用（手机端「库」会同步显示）'"
        @click="emit('toggle-favorite', props.app)"
      >
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
          <path
            d="M8 1.8l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.6l-3.8 2 .7-4.3-3.1-3 4.3-.6L8 1.8Z"
            :fill="props.app.favorite ? 'currentColor' : 'none'"
            stroke="currentColor"
            stroke-width="1.3"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <Button
        size="sm"
        variant="secondary"
        :disabled="props.busy || !props.canLaunch"
        :title="props.canLaunch ? '在本机启动该条目' : '当前服务不支持启动，请修复服务（升级）'"
        @click="emit('launch', props.app)"
      >
        启动
      </Button>
      <RowMenu :items="menuItems" :disabled="props.busy" @select="onMenuSelect" />
    </template>
  </DataRow>
</template>

<style scoped>
.select {
  display: inline-flex;
  align-items: center;
}

.select input {
  width: 15px;
  height: 15px;
  cursor: pointer;
  accent-color: var(--accent);
}

.title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  min-width: 0;
}

.title__name {
  font-size: var(--font-md);
  font-weight: var(--weight-medium);
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.sub {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  font-size: var(--font-xs);
  color: var(--text-3);
}

.sub__path {
  font-family: var(--font-mono);
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub__dot {
  color: var(--border-2);
}

.sub__meta {
  flex: none;
  white-space: nowrap;
}

.star {
  width: 30px;
  height: 30px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-3);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.star:hover:not(:disabled) {
  background: var(--surface-2);
  color: var(--accent);
}

.star--on {
  color: var(--accent);
}

.star:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}
</style>
