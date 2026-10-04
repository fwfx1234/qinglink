<script setup lang="ts">
/**
 * 服务不可用引导态（§7.5）：不是错误弹窗，而是明确的下一步操作。
 *
 * 三种「未连接」需要相反的处置，所以文案按链路状态分派：
 *  - unreachable：服务确实没跑起来，引导安装 / 启动；
 *  - protocol_mismatch：服务在跑但版本过旧，引导「修复服务（升级）」；
 *  - unauthorized：桌面端可执行文件变了，服务不再信任它，同样引导「修复服务」。
 *
 * 默认插槽在服务可用时渲染；服务不可用时显示引导内容。
 */
import { computed } from 'vue'
import { useOverviewStore } from '@/stores/overview'
import { useServiceStore } from '@/stores/service'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import type { ServiceAction } from '@/api/endpoints'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import Callout from '@/components/ui/Callout.vue'

const overview = useOverviewStore()
const service = useServiceStore()
const notifications = useNotificationsStore()

const blocked = computed(() => overview.serviceUnavailable || (overview.error?.serviceUnavailable ?? false))
const reason = computed(
  () =>
    service.lastServiceError ??
    overview.error?.message ??
    '服务未连接，请先安装并启动 QingLinkService',
)
const installable = computed(() => service.packageAvailable || overview.installState?.package_available === true)

const title = computed(() => {
  switch (service.linkState) {
    case 'protocol_mismatch':
      return '服务版本过旧，无法建立管理通道'
    case 'unauthorized':
      return '桌面端身份未获服务授权'
    case 'connecting':
      return '正在连接 QingLinkService'
    default:
      return '未连接到 QingLinkService'
  }
})

const protocolDetail = computed(() => {
  const expected = service.expectedVersion
  if (expected === null) return null
  return `服务协议 v${service.serviceVersion ?? '未知'} · 桌面端需要 v${expected}`
})

const steps = computed(() => {
  if (service.needsRepair) {
    return [
      {
        key: 'repair',
        title: '修复服务（升级 / 重新登记）',
        text: service.protocolMismatch
          ? '当前安装的服务版本早于桌面端的管理协议；修复会安装随桌面端发布的服务。'
          : '桌面端可执行文件已变化，服务的可信记录需要重新登记。',
        action: 'install' as ServiceAction,
        label: '修复服务',
        variant: 'primary' as const,
        disabled: !installable.value,
      },
      {
        key: 'reconnect',
        title: '重新连接',
        text: '跳过退避等待，立刻重新握手；服务修复完成后本页会自动恢复。',
        action: null,
        label: '重新连接',
        variant: 'secondary' as const,
        disabled: false,
      },
      {
        key: 'recheck',
        title: '重新检测',
        text: '重新拉取概览与体检结果，或等待 install_state 事件自动回流。',
        action: null,
        label: '重新检测',
        variant: 'ghost' as const,
        disabled: false,
      },
    ]
  }
  return [
    {
      key: 'install',
      title: '安装或修复安全输入服务',
      text: installable.value
        ? '发布包已就绪，点击后会拉起 UAC 提权窗口。'
        : '未检测到发布包，请先重新安装 QingLink 桌面端。',
      action: 'install' as ServiceAction,
      label: '安装 / 修复',
      variant: 'primary' as const,
      disabled: !installable.value,
    },
    {
      key: 'start',
      title: '启动服务',
      text: '服务安装完成后仍需启动；启动成功后本页会自动恢复。',
      action: 'start' as ServiceAction,
      label: '启动服务',
      variant: 'secondary' as const,
      disabled: false,
    },
    {
      key: 'reconnect',
      title: '重新连接',
      text: '跳过退避等待，立刻重新握手。',
      action: null,
      label: '重新连接',
      variant: 'ghost' as const,
      disabled: false,
    },
  ]
})

async function run(action: ServiceAction): Promise<void> {
  try {
    await service.runAction(action)
    notifications.info('已发起提权操作，结果将通过事件回落')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
}

function runStep(action: ServiceAction | null): void {
  if (action === null) {
    void refresh()
    return
  }
  void run(action)
}

async function refresh(): Promise<void> {
  try {
    await service.reconnect()
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
  void overview.refresh()
  void service.loadInstallState()
}
</script>

<template>
  <div v-if="blocked" class="gate">
    <Card :title="title">
      <p class="gate__reason">{{ reason }}</p>
      <p v-if="protocolDetail" class="gate__detail">{{ protocolDetail }}</p>

      <ol class="steps">
        <li v-for="(step, index) in steps" :key="step.key" class="step">
          <span class="step__index">{{ index + 1 }}</span>
          <div class="step__body">
            <p class="step__title">{{ step.title }}</p>
            <p class="step__text">{{ step.text }}</p>
          </div>
          <Button
            size="sm"
            :variant="step.variant"
            :disabled="step.disabled || service.busy"
            :loading="step.action === null ? service.isBusy('reconnect') : service.busyAction === step.action"
            @click="runStep(step.action)"
          >
            {{ step.label }}
          </Button>
        </li>
      </ol>
    </Card>

    <Callout tone="accent" compact>
      暂停（而非卸载）对外服务时不会出现本提示：暂停后管理能力仍然保留。
    </Callout>
  </div>

  <slot v-else />
</template>

<style scoped>
.gate {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.gate__reason {
  font-size: var(--font-md);
  color: var(--text-1);
  overflow-wrap: anywhere;
}

.gate__detail {
  margin-top: var(--space-2);
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  color: var(--text-3);
}

.steps {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin-top: var(--space-4);
}

.step {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border: 1px solid var(--border-1);
  border-radius: var(--radius-md);
  background: var(--surface-2);
}

.step__index {
  width: 22px;
  height: 22px;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 1px solid var(--border-2);
  font-family: var(--font-mono);
  font-size: var(--font-xs);
  color: var(--text-2);
}

.step__body {
  flex: 1 1 auto;
  min-width: 0;
}

.step__title {
  font-size: var(--font-md);
  color: var(--text-1);
}

.step__text {
  font-size: var(--font-sm);
  color: var(--text-3);
}
</style>
