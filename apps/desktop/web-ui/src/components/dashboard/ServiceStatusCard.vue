<script setup lang="ts">
import { computed, ref } from 'vue'
import { useServiceStore } from '@/stores/service'
import { useOverviewStore } from '@/stores/overview'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import { formatDuration } from '@/utils/format'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import StatusRow from '@/components/ui/StatusRow.vue'

const service = useServiceStore()
const overview = useOverviewStore()
const notifications = useNotificationsStore()

const pausePending = ref(false)

const connectionTone = computed<'success' | 'warning' | 'error'>(() => {
  switch (service.linkState) {
    case 'connected':
      return 'success'
    case 'connecting':
      return 'warning'
    default:
      return 'error'
  }
})
const connectionLabel = computed(() => service.linkLabel)
const pausedLabel = computed(() => (service.paused ? '对外服务已暂停' : '对外服务运行中'))
const addressLabel = computed(() => service.address ?? '—')
const networkLabel = computed(() => (service.networkReady ? '就绪，移动端可发现' : '未就绪'))
const version = computed(() => overview.health?.version ?? '—')
/** 服务端版本号；旧服务不返回时显示占位符而不是假装为空 */
const serviceVersionLabel = computed(() => service.serviceAppVersion ?? '—')
const uptime = computed(() => formatDuration(overview.health?.uptime_secs ?? null))
const webPort = computed(() => overview.health?.web_port ?? null)
const protocolLabel = computed(() => {
  if (service.expectedVersion === null) return '—'
  return `v${service.serviceVersion ?? '未知'} / 需要 v${service.expectedVersion}`
})
/** 服务已连接但管理能力缺失：通常是服务版本过旧 */
const degraded = computed(
  () => service.connected && service.capabilities !== null && !service.capabilities.management,
)

async function togglePause(): Promise<void> {
  pausePending.value = true
  try {
    await service.setPaused(!service.paused)
    // 不做乐观更新：service/status 事件回流后状态自然收敛
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  } finally {
    pausePending.value = false
  }
}

function refresh(): void {
  void service.loadStatus()
  void overview.refresh()
}
</script>

<template>
  <Card title="服务状态">
    <template #actions>
      <Chip :tone="connectionTone" dot>{{ connectionLabel }}</Chip>
      <Button size="sm" variant="ghost" :loading="service.loading" @click="refresh">重新检测</Button>
    </template>

    <div class="status">
      <StatusRow label="管理链路">
        <StatusDot :tone="connectionTone" />
        {{ connectionLabel }}
      </StatusRow>

      <StatusRow label="协议版本" :value="protocolLabel" mono />

      <StatusRow label="对外服务">
        <StatusDot :tone="service.paused ? 'warning' : service.connected ? 'success' : 'error'" />
        {{ pausedLabel }}
      </StatusRow>

      <StatusRow label="监听地址" :value="addressLabel" mono />

      <StatusRow label="网络就绪">
        <StatusDot :tone="service.networkReady ? 'success' : 'warning'" />
        {{ networkLabel }}
      </StatusRow>

      <StatusRow label="桌面端版本" :value="version" mono />

      <StatusRow label="服务端版本" :value="serviceVersionLabel" mono />

      <StatusRow label="已运行" :value="uptime" mono />

      <StatusRow v-if="webPort !== null" label="Web 端口" :value="webPort" mono />
    </div>

    <p v-if="service.lastServiceError" class="status__error">
      <strong>服务端报告：</strong>{{ service.lastServiceError }}
    </p>

    <p v-if="service.needsRepair" class="status__hint">
      {{
        service.protocolMismatch
          ? '服务协议版本与桌面端不一致，管理接口暂不可用：请在「安全输入服务体检」中点击「安装 / 修复」，或在诊断页一键修复。'
          : '服务未授权当前桌面端：桌面端可执行文件已变化，请在「安全输入服务体检」中点击「安装 / 修复」重新登记。'
      }}
    </p>

    <p v-else-if="degraded" class="status__hint">
      服务已连接但管理能力不可用，可能版本过旧：请在下方体检卡中使用「安装 / 修复」升级服务。
    </p>

    <template #footer>
      <Button
        size="sm"
        :variant="service.paused ? 'primary' : 'secondary'"
        :disabled="!service.canPause || !service.connected"
        :loading="pausePending || service.isBusy('pause')"
        @click="togglePause"
      >
        {{ service.paused ? '恢复对外服务' : '暂停对外服务' }}
      </Button>
    </template>
  </Card>
</template>

<style scoped>
.status {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 0 var(--space-6);
}

.status__error {
  margin-top: var(--space-4);
  padding: var(--space-3);
  border-radius: var(--radius-md);
  background: var(--error-soft);
  border: 1px solid var(--error-border);
  color: var(--text-1);
  font-size: var(--font-sm);
}

.status__hint {
  margin-top: var(--space-3);
  font-size: var(--font-sm);
  color: var(--warning);
}
</style>
