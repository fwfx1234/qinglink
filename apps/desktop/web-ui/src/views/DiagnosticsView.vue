<script setup lang="ts">
/**
 * 服务诊断中心。
 *
 * 这张页面存在的理由只有一个：把「服务未连接」拆成可以分别处置的原因。
 * 服务在运行但管理链路不可用时，概览页只能说一句「未连接」；这里给出
 * 链路状态、协议版本、管道探测结果与完整错误链，并直接给出修复动作。
 */
import { computed, onMounted, ref } from 'vue'
import { useDiagnosticsStore } from '@/stores/diagnostics'
import { useServiceStore } from '@/stores/service'
import { useLogsStore } from '@/stores/logs'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import { formatCount, formatDuration, formatRelative } from '@/utils/format'
import type { ServiceAction } from '@/api/endpoints'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import Callout from '@/components/ui/Callout.vue'
import Chip from '@/components/ui/Chip.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import InfoTip from '@/components/ui/InfoTip.vue'
import LoadingBlock from '@/components/ui/LoadingBlock.vue'
import ErrorLine from '@/components/ui/ErrorLine.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import StatusRow from '@/components/ui/StatusRow.vue'

const diagnostics = useDiagnosticsStore()
const service = useServiceStore()
const logs = useLogsStore()
const notifications = useNotificationsStore()

const showRaw = ref(false)
const exporting = ref(false)

const linkTone = computed<'success' | 'warning' | 'error'>(() => {
  switch (service.linkState) {
    case 'connected':
      return 'success'
    case 'connecting':
      return 'warning'
    default:
      return 'error'
  }
})

const protocolLabel = computed(() => {
  const expected = diagnostics.report?.service.expected_version ?? service.expectedVersion
  const actual = diagnostics.report?.service.service_version ?? service.serviceVersion
  if (expected === null) return '—'
  return `服务 v${actual ?? '未知'} / 桌面端 v${expected}`
})

const probeLabel = computed(() => {
  const probe = diagnostics.report?.pipe_probe
  if (!probe) return '—'
  if (probe.reachable) return '可达（v2 握手成功）'
  return probe.message ?? probe.error ?? '不可达'
})

const rawReport = computed(() =>
  diagnostics.report ? JSON.stringify(diagnostics.report, null, 2) : '',
)

onMounted(() => {
  void diagnostics.load()
  if (!service.status) void service.loadStatus()
})

async function repair(): Promise<void> {
  await run('install')
}

async function run(action: ServiceAction): Promise<void> {
  try {
    await service.runAction(action)
    notifications.info('已发起提权操作，结果将通过事件回流')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
}

async function reconnect(): Promise<void> {
  try {
    await service.reconnect()
    notifications.info('已请求重新连接，结果会通过事件回流')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
}

async function refresh(): Promise<void> {
  await diagnostics.load()
  await service.loadInstallState()
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
</script>

<template>
  <Card title="服务链路" subtitle="本地命名管道与协议握手结果">
    <template #actions>
      <Chip :tone="linkTone" dot>{{ service.linkLabel }}</Chip>
      <Button size="sm" variant="ghost" :loading="service.isBusy('reconnect')" @click="reconnect">
        重新连接
      </Button>
      <Button
        size="sm"
        :variant="service.needsRepair ? 'primary' : 'secondary'"
        :disabled="service.busy"
        :loading="service.busyAction === 'install'"
        @click="repair"
      >
        修复服务
      </Button>
    </template>

    <Callout v-if="service.needsRepair" tone="warning" compact>
      {{
        service.protocolMismatch
          ? '当前安装的服务版本早于桌面端的管理协议，管理接口会把这类失败报成「服务未连接」。点击「修复服务」安装随桌面端发布的服务即可。'
          : '桌面端可执行文件已重建，服务里登记的可信哈希不再匹配。点击「修复服务」重新登记即可。'
      }}
    </Callout>

    <div class="grid">
      <StatusRow label="链路状态">
        <StatusDot :tone="linkTone" />
        {{ service.linkLabel }}
      </StatusRow>

      <StatusRow label="协议版本" :value="protocolLabel" mono />

      <StatusRow label="管道探测">
        <StatusDot :tone="diagnostics.pipeReachable ? 'success' : 'error'" />
        {{ probeLabel }}
      </StatusRow>

      <StatusRow label="服务进程" :value="service.serviceInstalled ? (service.serviceRunning ? '已安装并运行' : '已安装未运行') : '未安装'" />

      <StatusRow label="服务文件一致" :value="service.binaryMatches ? '是' : '否'" />

      <StatusRow label="回环健康协议" :value="service.networkReady ? '就绪' : '未就绪'" />

      <StatusRow label="监听地址" :value="service.address ?? '—'" mono />

      <StatusRow label="管理能力" :value="service.canManage ? '可用' : '不可用'" />
    </div>

    <p v-if="service.lastServiceError" class="error-line">
      <strong>服务端报告：</strong>{{ service.lastServiceError }}
    </p>
  </Card>

  <Card title="运行环境" subtitle="进程、完整性级别与目录">
    <template #actions>
      <Button size="sm" variant="secondary" :loading="diagnostics.loading" @click="refresh">运行自检</Button>
      <Button size="sm" variant="ghost" :loading="exporting" @click="exportBundle">导出诊断包</Button>
    </template>

    <LoadingBlock v-if="diagnostics.loading && !diagnostics.report" label="正在采集诊断信息…" />

    <EmptyState
      v-else-if="!diagnostics.report"
      title="还没有诊断报告"
      description="点击「运行自检」采集一次链路、进程与目录信息。"
      action-label="运行自检"
      @action="refresh"
    />

    <div v-else class="grid">
      <StatusRow label="桌面端版本" :value="diagnostics.report.version" mono />
      <StatusRow label="进程 PID" :value="diagnostics.report.pid" mono />
      <StatusRow label="已运行" :value="formatDuration(diagnostics.report.uptime_secs)" mono />
      <StatusRow label="进程完整性" :value="diagnostics.report.integrity" mono />
      <StatusRow label="通知区域完整性" :value="diagnostics.report.shell_integrity" mono />
      <StatusRow label="数据目录" :value="diagnostics.report.data_dir" mono />
      <StatusRow label="桌面端日志目录" :value="diagnostics.report.log_dir" mono />
      <StatusRow label="服务端日志目录" :value="diagnostics.report.service_log_dir" mono />
      <StatusRow
        label="内嵌界面"
        :value="
          diagnostics.report.web_interface.placeholder
            ? '占位页（未构建）'
            : `${formatCount(diagnostics.report.web_interface.assets_embedded)} 个资源`
        "
        mono
      />
      <StatusRow
        label="托盘图标"
        :value="diagnostics.report.tray ? diagnostics.report.tray.detail : '—'"
      />
    </div>
  </Card>

  <Card v-if="diagnostics.report" title="原始报告" subtitle="与 --self-check / --check-service 同源">
    <template #actions>
      <InfoTip text="故障反馈时把这份 JSON（或诊断包）一起附上，比描述现象更快定位。" label="怎么用？" />
      <Button size="sm" variant="ghost" @click="showRaw = !showRaw">
        {{ showRaw ? '收起' : '展开' }}
      </Button>
    </template>
    <p class="hint">
      采集于 {{ diagnostics.loadedAt ? formatRelative(diagnostics.loadedAt) : '—' }}。
    </p>
    <pre v-if="showRaw" class="raw">{{ rawReport }}</pre>
  </Card>

  <ErrorLine :error="diagnostics.error" />
</template>

<style scoped>
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 0 var(--space-6);
}

.error-line {
  margin-top: var(--space-4);
  padding: var(--space-3);
  border-radius: var(--radius-md);
  background: var(--error-soft);
  border: 1px solid var(--error-border);
  color: var(--text-1);
  font-size: var(--font-sm);
  overflow-wrap: anywhere;
}

.hint {
  font-size: var(--font-sm);
  color: var(--text-3);
}

.raw {
  margin-top: var(--space-3);
  padding: var(--space-3);
  max-height: 420px;
  overflow: auto;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-2);
  font-family: var(--font-mono);
  font-size: var(--font-xs);
  color: var(--text-2);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
</style>
