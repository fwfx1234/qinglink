<script setup lang="ts">
/**
 * 左侧主导航。
 *
 * 六个目的地 + 常驻的链路状态，竖排在左侧能让内容区用满宽度并保持一条清晰的
 * 阅读轴线；顶部横排导航会把这两者挤在同一行里争夺空间。
 *
 * 窄屏收成图标轨道（只留图标 + title），不折叠成汉堡菜单：管理台常在副屏半宽
 * 窗口里使用，一次点击就能到目标比"先展开菜单"更省事。
 */
import { computed } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import { ROUTE_NAMES } from '@/utils/constants'
import { useConnectionStore } from '@/stores/connection'
import { useOverviewStore } from '@/stores/overview'
import { useServiceStore } from '@/stores/service'
import { useUpdateStore } from '@/stores/update'
import { formatRelative } from '@/utils/format'
import StatusDot from '@/components/ui/StatusDot.vue'
import ThemeToggle from '@/components/ui/ThemeToggle.vue'

const route = useRoute()
const connection = useConnectionStore()
const overview = useOverviewStore()
const service = useServiceStore()

/** 图标只用 path 数据：一个 <svg> 渲染全部，模板里不堆 v-if 分支。 */
const navigation = [
  {
    name: ROUTE_NAMES.overview,
    label: '概览',
    path: 'M2.5 2.5h4.2v4.2H2.5zM9.3 2.5h4.2v4.2H9.3zM2.5 9.3h4.2v4.2H2.5zM9.3 9.3h4.2v4.2H9.3z',
  },
  { name: ROUTE_NAMES.apps, label: '目录', path: 'M3 4.2h10M3 8h10M3 11.8h6.4' },
  { name: ROUTE_NAMES.logs, label: '日志', path: 'M3.5 3.5h9v9h-9zM6 6.6h4.4M6 9.4h3' },
  { name: ROUTE_NAMES.diagnostics, label: '诊断', path: 'M2.6 8.6h2.6l1.3-4 2.1 7 1.4-3h3.4' },
  { name: ROUTE_NAMES.settings, label: '设置', path: 'M3 5.4h10M3 10.6h10M6.4 3.6v3.6M9.6 8.8v3.6' },
  { name: ROUTE_NAMES.update, label: '更新', path: 'M8 2.4v7.6M5.2 7.2 8 10l2.8-2.8M3 12.6h10' },
]

const update = useUpdateStore()
const updateAvailable = computed(() => update.hasUpdate)

const pairingPending = computed(() => overview.pairing !== null)
const lastError = computed(() => overview.service?.last_error ?? connection.lastError)
/** 链路状态优先：它区分「没连上 / 版本过旧 / 未授权」，而通道标签说不清。 */
const statusText = computed(() => service.linkLabel)
const statusTone = computed<'success' | 'warning' | 'error'>(() => {
  switch (service.linkState) {
    case 'connected':
      return 'success'
    case 'connecting':
      return 'warning'
    default:
      return 'error'
  }
})
const statusTitle = computed(() =>
  lastError.value
    ? `最近错误：${lastError.value}`
    : `最近握手：${connection.lastHelloAt ? formatRelative(connection.lastHelloAt) : '—'}`,
)
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <span class="brand__mark" aria-hidden="true">
        <svg viewBox="0 0 16 16" width="16" height="16">
          <path
            d="M3.2 8a4.8 4.8 0 1 0 4.8-4.8"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linecap="round"
          />
          <circle cx="8" cy="8" r="1.9" fill="currentColor" />
        </svg>
      </span>
      <span class="brand__name">QingLink</span>
    </div>

    <nav class="nav" aria-label="主导航">
      <RouterLink
        v-for="item in navigation"
        :key="item.name"
        class="nav__item"
        :class="{ 'nav__item--active': route.name === item.name }"
        :title="item.label"
        :to="{ name: item.name }"
      >
        <svg class="nav__icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
          <path
            :d="item.path"
            fill="none"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <span class="nav__label">{{ item.label }}</span>
        <StatusDot v-if="item.name === ROUTE_NAMES.overview && pairingPending" tone="warning" pulse />
        <StatusDot v-if="item.name === ROUTE_NAMES.update && updateAvailable" tone="warning" pulse />
      </RouterLink>
    </nav>

    <div class="foot">
      <span class="state" :title="statusTitle">
        <StatusDot
          :tone="statusTone"
          :pulse="connection.isReconnecting || service.linkState === 'connecting'"
        />
        <span class="state__text">{{ statusText }}</span>
      </span>
      <ThemeToggle />
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  height: 100vh;
  padding: var(--space-5) var(--space-4);
  border-right: 1px solid var(--border-1);
  background: var(--surface-1);
}

.brand {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.brand__mark {
  width: 30px;
  height: 30px;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: var(--accent-bright);
  color: var(--accent-ink);
}

.brand__name {
  font-size: var(--font-lg);
  font-weight: var(--weight-semibold);
  letter-spacing: -0.01em;
  white-space: nowrap;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav__item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  height: 36px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-md);
  color: var(--text-2);
  font-weight: var(--weight-medium);
  text-decoration: none;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.nav__item:hover {
  background: var(--surface-2);
  color: var(--text-1);
  text-decoration: none;
}

.nav__item--active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: var(--weight-semibold);
}

.nav__icon {
  flex: none;
  opacity: 0.9;
}

.nav__label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.foot {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-1);
}

.state {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  font-size: var(--font-sm);
  color: var(--text-2);
}

.state__text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 窄屏收成图标轨道：标签与状态文字让位，图标与 title 保留全部功能。 */
@media (max-width: 1080px) {
  .sidebar {
    align-items: center;
    padding: var(--space-5) var(--space-2);
  }

  .brand__name,
  .nav__label,
  .state__text {
    display: none;
  }

  .nav__item {
    justify-content: center;
    padding: 0;
    width: 36px;
  }

  .foot {
    flex-direction: column;
    border-top: none;
    padding-top: 0;
  }
}
</style>
