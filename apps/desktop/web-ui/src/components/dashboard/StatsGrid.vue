<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useOverviewStore } from '@/stores/overview'
import { formatCount } from '@/utils/format'
import { ROUTE_NAMES, type Tone } from '@/utils/constants'
import Card from '@/components/ui/Card.vue'
import Spinner from '@/components/ui/Spinner.vue'

const overview = useOverviewStore()
const router = useRouter()

interface StatTile {
  key: string
  label: string
  value: number | null
  tone: Tone
  to: string | null
}

const tiles = computed<StatTile[]>(() => {
  const stats = overview.stats
  return [
    { key: 'apps', label: '应用条目', value: stats?.apps ?? null, tone: 'accent', to: ROUTE_NAMES.apps },
    { key: 'hidden', label: '已隐藏', value: stats?.hidden_apps ?? null, tone: 'neutral', to: ROUTE_NAMES.apps },
    { key: 'devices', label: '已配对设备', value: stats?.devices ?? null, tone: 'success', to: null },
    {
      key: 'errors',
      label: '错误日志',
      value: stats?.errors ?? null,
      tone: (stats?.errors ?? 0) > 0 ? 'error' : 'neutral',
      to: ROUTE_NAMES.logs,
    },
  ]
})

function open(target: string | null): void {
  if (target) void router.push({ name: target })
}
</script>

<template>
  <Card title="目录与运行统计">
    <template #actions>
      <Spinner v-if="overview.loading" :size="12" tone="muted" />
    </template>

    <div class="tiles">
      <component
        :is="tile.to ? 'button' : 'div'"
        v-for="tile in tiles"
        :key="tile.key"
        class="tile"
        :class="[`tile--${tile.tone}`, { 'tile--link': tile.to !== null }]"
        :type="tile.to ? 'button' : undefined"
        @click="open(tile.to)"
      >
        <span class="tile__label">{{ tile.label }}</span>
        <span class="tile__value">{{ tile.value === null ? '—' : formatCount(tile.value) }}</span>
      </component>
    </div>
  </Card>
</template>

<style scoped>
.tiles {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: var(--space-3);
}

.tile {
  display: flex;
  flex-direction: column;
  gap: 2px;
  align-items: flex-start;
  text-align: left;
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-2);
  min-width: 0;
  transition: background var(--transition-fast), border-color var(--transition-fast),
    transform var(--transition-fast);
}

.tile--link {
  cursor: pointer;
}

.tile--link:hover {
  background: var(--surface-3);
  border-color: var(--border-2);
  transform: translateY(-1px);
}

.tile__label {
  font-size: var(--font-sm);
  color: var(--text-3);
}

/* 数字用表格数字对齐即可，不用等宽字体（等宽在这里会显得像终端输出） */
.tile__value {
  font-size: var(--font-2xl);
  font-weight: var(--weight-semibold);
  line-height: 1.15;
  letter-spacing: -0.02em;
  color: var(--text-1);
  font-variant-numeric: tabular-nums;
}

.tile--accent .tile__value {
  color: var(--accent);
}

.tile--success .tile__value {
  color: var(--success);
}

.tile--error .tile__value {
  color: var(--error);
}
</style>
