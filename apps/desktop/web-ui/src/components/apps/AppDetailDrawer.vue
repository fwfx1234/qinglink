<script setup lang="ts">
/**
 * 单个目录条目的详情面板。
 *
 * 存在的理由很直接：Windows 路径太长，列表行里放不下也没人读；而"这个条目到底
 * 指向哪个 exe、id 是什么、图标是哪来的"又只有完整信息才说得清。所以列表行只留
 * 尾部两段路径，完整信息放这里，并提供一键复制。
 */
import { computed, ref, watch } from 'vue'
import type { AppItem } from '@/api/types'
import { formatUnixSeconds } from '@/utils/format'
import { copyText } from '@/utils/clipboard'
import AppIcon from './AppIcon.vue'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import Drawer from '@/components/ui/Drawer.vue'

const props = defineProps<{
  app: AppItem | null
  busy: boolean
  canLaunch: boolean
  canManageIcon: boolean
}>()

const emit = defineEmits<{
  (event: 'close'): void
  (event: 'launch', app: AppItem): void
  (event: 'toggle-favorite', app: AppItem): void
  (event: 'toggle-pinned', app: AppItem): void
  (event: 'rename', app: AppItem): void
  (event: 'icon', app: AppItem): void
  (event: 'toggle-hidden', app: AppItem): void
  (event: 'restore-name', app: AppItem): void
  (event: 'delete', app: AppItem): void
}>()

const copied = ref<string | null>(null)
let copiedTimer: ReturnType<typeof setTimeout> | null = null

const ICON_SOURCE_LABELS: Record<string, string> = {
  manual: '手动指定',
  steam: 'Steam 官方封面',
  shortcut: '快捷方式目标',
  executable: '程序内嵌资源',
  image: '同目录图片文件',
  shell: '系统默认图标（可能不准确）',
  none: '未找到可用图标',
}

const iconSourceLabel = computed(() =>
  props.app?.icon_source ? (ICON_SOURCE_LABELS[props.app.icon_source] ?? props.app.icon_source) : '尚未解析',
)

const iconSuspect = computed(
  () =>
    !props.app?.has_icon ||
    props.app?.icon_source === 'shell' ||
    props.app?.icon_source === 'none',
)

const argsLabel = computed(() =>
  props.app && props.app.args.length > 0 ? props.app.args.join(' ') : '未设置',
)

const iconUpdatedLabel = computed(() => formatUnixSeconds(props.app?.icon_updated_at ?? null))

/** 常用/隐藏等状态标签，集中在一处，空时不渲染空行。 */
const statusChips = computed(() => {
  const app = props.app
  if (!app) return []
  const chips: { label: string; tone: 'accent' | 'warning' | 'neutral' }[] = []
  if (app.favorite) chips.push({ label: '常用', tone: 'accent' })
  if (app.pinned) chips.push({ label: '首页置顶', tone: 'accent' })
  if (app.hidden) chips.push({ label: '已隐藏', tone: 'warning' })
  if (app.user_added) chips.push({ label: '手动添加', tone: 'neutral' })
  if (app.renamed) chips.push({ label: '已重命名', tone: 'accent' })
  if (iconSuspect.value) chips.push({ label: '图标待确认', tone: 'warning' })
  return chips
})

watch(
  () => props.app?.id,
  () => {
    copied.value = null
  },
)

async function copy(field: string, value: string): Promise<void> {
  const ok = await copyText(value)
  copied.value = ok ? field : null
  if (copiedTimer) clearTimeout(copiedTimer)
  copiedTimer = setTimeout(() => {
    copied.value = null
  }, 1600)
}
</script>

<template>
  <Drawer
    :open="props.app !== null"
    title="条目详情"
    description="完整路径、标识与当前状态"
    :width="520"
    @close="emit('close')"
  >
    <template v-if="props.app">
      <div class="hero">
        <AppIcon
          :id="props.app.id"
          :has-icon="props.app.has_icon"
          :version="props.app.icon_updated_at"
          :name="props.app.name"
          :size="56"
        />
        <div class="hero__text">
          <p class="hero__name">{{ props.app.name || '（未命名）' }}</p>
          <p v-if="props.app.renamed" class="hero__original">原名：{{ props.app.original_name }}</p>
          <div v-if="statusChips.length > 0" class="hero__chips">
            <Chip v-for="chip in statusChips" :key="chip.label" size="sm" :tone="chip.tone">
              {{ chip.label }}
            </Chip>
          </div>
        </div>
      </div>

      <section class="block">
        <h3 class="block__title">路径</h3>
        <div class="field">
          <p class="field__label">可执行文件 / 快捷方式</p>
          <p class="field__value field__value--path">{{ props.app.exe_path || '—' }}</p>
          <Button
            size="sm"
            variant="ghost"
            :title="copied === 'path' ? '已复制' : '复制完整路径'"
            @click="copy('path', props.app.exe_path)"
          >
            {{ copied === 'path' ? '已复制' : '复制路径' }}
          </Button>
        </div>
        <div class="field">
          <p class="field__label">条目标识（id）</p>
          <p class="field__value field__value--path">{{ props.app.id }}</p>
          <Button
            size="sm"
            variant="ghost"
            :title="copied === 'id' ? '已复制' : '复制条目 id'"
            @click="copy('id', props.app.id)"
          >
            {{ copied === 'id' ? '已复制' : '复制 id' }}
          </Button>
        </div>
      </section>

      <section class="block">
        <h3 class="block__title">操作</h3>
        <div class="actions">
          <Button
            variant="primary"
            size="sm"
            :disabled="props.busy || !props.canLaunch"
            :title="props.canLaunch ? '在本机启动该条目' : '当前服务不支持启动，请修复服务（升级）'"
            @click="emit('launch', props.app)"
          >
            在本机启动
          </Button>
          <Button size="sm" :disabled="props.busy" @click="emit('toggle-favorite', props.app)">
            {{ props.app.favorite ? '移出常用' : '加入常用' }}
          </Button>
          <Button
            v-if="props.app.favorite"
            size="sm"
            :disabled="props.busy"
            @click="emit('toggle-pinned', props.app)"
          >
            {{ props.app.pinned ? '取消首页置顶' : '置顶到手机首页' }}
          </Button>
          <Button size="sm" :disabled="props.busy" @click="emit('rename', props.app)">重命名</Button>
          <Button
            v-if="props.app.renamed"
            size="sm"
            :disabled="props.busy"
            @click="emit('restore-name', props.app)"
          >
            恢复原名
          </Button>
          <Button
            size="sm"
            :disabled="props.busy || !props.canManageIcon"
            :title="props.canManageIcon ? '单独重新扫描或指定图标' : '当前服务不支持图标管理，请修复服务（升级）'"
            @click="emit('icon', props.app)"
          >
            更换图标
          </Button>
          <Button size="sm" :disabled="props.busy" @click="emit('toggle-hidden', props.app)">
            {{ props.app.hidden ? '取消隐藏' : '隐藏条目' }}
          </Button>
        </div>
      </section>

      <section class="block">
        <h3 class="block__title">属性</h3>
        <dl class="props">
          <div class="props__row">
            <dt>分类</dt>
            <dd>{{ props.app.category || '—' }}</dd>
          </div>
          <div class="props__row">
            <dt>来源</dt>
            <dd>{{ props.app.source || '—' }}</dd>
          </div>
          <div class="props__row">
            <dt>启动参数</dt>
            <dd class="props__value--wrap">{{ argsLabel }}</dd>
          </div>
          <div class="props__row">
            <dt>图标来源</dt>
            <dd>{{ iconSourceLabel }}</dd>
          </div>
          <div class="props__row">
            <dt>图标更新时间</dt>
            <dd>{{ iconUpdatedLabel }}</dd>
          </div>
        </dl>
      </section>

      <section class="block block--danger">
        <h3 class="block__title">危险操作</h3>
        <div class="actions">
          <Button
            size="sm"
            variant="danger-ghost"
            :disabled="props.busy"
            @click="emit('delete', props.app)"
          >
            从目录中移除
          </Button>
        </div>
        <p class="block__hint">这是软删除：数据保留，后续扫描可能重新发现该条目。</p>
      </section>
    </template>
  </Drawer>
</template>

<style scoped>
.hero {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.hero__text {
  min-width: 0;
}

.hero__name {
  font-size: var(--font-xl);
  font-weight: var(--weight-semibold);
  letter-spacing: -0.02em;
  color: var(--text-1);
  overflow-wrap: anywhere;
}

.hero__original {
  margin-top: 2px;
  font-size: var(--font-sm);
  color: var(--text-3);
}

.hero__chips {
  margin-top: var(--space-2);
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}

.block {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4);
  border: 1px solid var(--border-1);
  border-radius: var(--radius-lg);
  background: var(--surface-2);
}

.block--danger {
  background: var(--surface-1);
  border-color: var(--error-border);
}

.block__title {
  font-size: var(--font-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-2);
  letter-spacing: 0.01em;
}

.block__hint {
  font-size: var(--font-xs);
  color: var(--text-3);
}

.field {
  display: grid;
  grid-template-columns: 1fr auto;
  /* 复制按钮对齐到值的第一行：路径折行后按钮居中会显得错位。 */
  align-items: start;
  gap: var(--space-2) var(--space-3);
  padding: var(--space-3);
  background: var(--surface-1);
  border: 1px solid var(--border-1);
  border-radius: var(--radius-md);
}

.field__label {
  grid-column: 1 / -1;
  font-size: var(--font-xs);
  color: var(--text-3);
}

/* 路径允许换行：这里就是用来读完整值的，截断反而失去意义。 */
.field__value--path {
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  color: var(--text-1);
  overflow-wrap: anywhere;
  user-select: all;
}

.props {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--border-1);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.props__row {
  display: grid;
  grid-template-columns: 108px 1fr;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--surface-1);
}

.props__row dt {
  font-size: var(--font-sm);
  color: var(--text-3);
}

.props__row dd {
  margin: 0;
  font-size: var(--font-sm);
  color: var(--text-1);
  overflow-wrap: anywhere;
}

.props__value--wrap {
  font-family: var(--font-mono);
  word-break: break-word;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
</style>
