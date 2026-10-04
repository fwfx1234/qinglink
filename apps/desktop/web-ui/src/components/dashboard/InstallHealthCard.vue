<script setup lang="ts">
import { computed, ref } from 'vue'
import { useServiceStore } from '@/stores/service'
import { useOverviewStore } from '@/stores/overview'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import type { ServiceAction } from '@/api/endpoints'
import Card from '@/components/ui/Card.vue'
import Chip from '@/components/ui/Chip.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import StatusRow from '@/components/ui/StatusRow.vue'
import Spinner from '@/components/ui/Spinner.vue'

const service = useServiceStore()
const overview = useOverviewStore()
const notifications = useNotificationsStore()

const lastAction = ref<ServiceAction | null>(null)

/**
 * 托盘图标的注册结果：Windows 11 折叠区里拿不到 rect，那是正常状态；
 * `visible=false` 才是 shell 拒绝了注册。
 */
const tray = computed(() => {
  const report = overview.tray
  if (!report) return null
  const reachable = report.visible
  return {
    ok: reachable,
    label: '托盘图标',
    value: reachable ? (report.rect ? '已显示' : '在折叠区') : '注册被拒绝',
    detail: report.detail,
  }
})

const checks = computed(() => {
  const install = service.install
  return [
    { key: 'package', label: '发布包可用', ok: service.packageAvailable, value: install ? (install.package_available ? '已就绪' : '缺失') : '—' },
    {
      key: 'installed',
      label: '服务已安装',
      ok: service.serviceInstalled,
      value: install ? (install.service_installed ? '已安装' : '未安装') : '—',
    },
    {
      key: 'running',
      label: '服务运行中',
      ok: service.serviceRunning,
      value: install ? (install.service_running ? '运行中' : '已停止') : '—',
    },
    {
      key: 'binary',
      label: '二进制版本匹配',
      ok: service.binaryMatches,
      value: install ? (install.service_binary_matches ? '匹配' : '需更新') : '—',
    },
    {
      // 体检的既有项只能看「文件与安装」；管理协议是否兼容只有真实握手才知道，
      // 缺了这一项就会出现「体检说可用、链路说未连接」的自相矛盾。
      key: 'protocol',
      label: '管理协议匹配',
      ok: service.protocolOk !== false,
      value:
        service.protocolOk === false
          ? '不兼容（需修复服务）'
          : service.protocolOk === true
            ? '已握手'
            : '待确认',
    },
  ]
})

const stateChip = computed(() => {
  if (!service.install) return { tone: 'neutral' as const, label: '未检测' }
  if (service.elevatedBusy) return { tone: 'warning' as const, label: '提权操作执行中' }
  if (service.protocolOk === false) return { tone: 'error' as const, label: '管理通道不可用' }
  if (service.available) return { tone: 'success' as const, label: '可用' }
  if (!service.packageAvailable) return { tone: 'warning' as const, label: '缺少发布包' }
  return { tone: 'error' as const, label: '不可用' }
})

const actions: {
  action: ServiceAction
  label: string
  variant: 'primary' | 'secondary' | 'danger' | 'danger-ghost'
}[] = [
  { action: 'install', label: '安装 / 修复', variant: 'primary' },
  { action: 'start', label: '启动', variant: 'secondary' },
  { action: 'stop', label: '停止', variant: 'secondary' },
  { action: 'restart', label: '重启', variant: 'secondary' },
  { action: 'uninstall', label: '卸载', variant: 'danger-ghost' },
]

async function run(action: ServiceAction): Promise<void> {
  if (action === 'uninstall') {
    const accepted = await notifications.confirm({
      title: '卸载安全输入服务',
      message: '卸载后移动端的输入控制能力将不可用，直到重新安装。',
      detail: '该操作会弹出 UAC 提权确认窗口。',
      confirmText: '确认卸载',
      cancelText: '取消',
      tone: 'danger',
    })
    if (!accepted) return
  }
  lastAction.value = action
  try {
    const accepted = await service.runAction(action)
    if (accepted) {
      notifications.info(`已发起提权操作：${labelOf(action)}，结果将通过事件回落`)
    }
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  } finally {
    lastAction.value = null
  }
}

function labelOf(action: ServiceAction): string {
  return actions.find((item) => item.action === action)?.label ?? action
}

function refresh(): void {
  void service.loadInstallState()
  void overview.refresh()
}
</script>

<template>
  <Card title="安全输入服务体检">
    <template #actions>
      <Chip :tone="stateChip.tone" dot>
        {{ stateChip.label }}
        <Spinner v-if="service.elevatedBusy" :size="10" />
      </Chip>
      <Button size="sm" variant="ghost" :loading="service.loading" @click="refresh">重新体检</Button>
    </template>

    <p class="summary">{{ service.install?.detail ?? '尚未获取到体检详情' }}</p>

    <p v-if="service.lastResult" class="summary">
      <strong>上次操作结论：</strong>{{ service.lastResult }}
    </p>

    <details class="details">
      <summary>检查明细</summary>

      <div class="checks">
        <StatusRow v-for="check in checks" :key="check.key" :label="check.label">
          <StatusDot :tone="check.ok ? 'success' : 'warning'" />
          <span :class="{ 'check--bad': !check.ok }">{{ check.value }}</span>
        </StatusRow>

        <StatusRow v-if="tray" :label="tray.label">
          <StatusDot :tone="tray.ok ? 'success' : 'error'" />
          <span :class="{ 'check--bad': !tray.ok }" :title="tray.detail">{{ tray.value }}</span>
        </StatusRow>
      </div>

      <p v-if="tray" class="summary">{{ tray.detail }}</p>
    </details>

    <template #footer>
      <div class="actions">
        <Button
          v-for="entry in actions"
          :key="entry.action"
          size="sm"
          :variant="entry.variant"
          :loading="service.busyAction === entry.action"
          :disabled="service.busy && service.busyAction !== entry.action"
          @click="run(entry.action)"
        >
          {{ entry.label }}
        </Button>
        <span v-if="service.elevatedBusy" class="actions__busy">
          <Spinner :size="12" />
          提权操作进行中，请勿关闭窗口…
        </span>
      </div>
    </template>
  </Card>
</template>

<style scoped>
.summary {
  font-size: var(--font-sm);
  color: var(--text-2);
}

.details {
  margin-top: var(--space-3);
}

.details summary {
  cursor: pointer;
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  color: var(--text-2);
}

.details summary::marker {
  color: var(--text-3);
}

.details summary:hover {
  color: var(--text-1);
}

.checks {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 0 var(--space-6);
  margin-top: var(--space-2);
}

.check--bad {
  color: var(--warning);
}

.actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  width: 100%;
}

.actions__busy {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
  font-size: var(--font-sm);
  color: var(--warning);
}
</style>
