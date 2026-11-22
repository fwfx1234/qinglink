<script setup lang="ts">
/**
 * 程序更新页。
 *
 * 检查/下载/安装都是后端动作，本页只负责呈现状态与触发动作；真实状态由
 * `system/update_state` 事件驱动（§5），进入页面时拉一次快照作为初值。
 */
import { computed, onMounted } from 'vue'
import { useUpdateStore } from '@/stores/update'
import { useNotificationsStore } from '@/stores/notifications'
import { formatBytes, formatRelative } from '@/utils/format'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import Callout from '@/components/ui/Callout.vue'
import Chip from '@/components/ui/Chip.vue'
import ErrorLine from '@/components/ui/ErrorLine.vue'
import StatusRow from '@/components/ui/StatusRow.vue'

const update = useUpdateStore()
const notifications = useNotificationsStore()

onMounted(() => {
  void update.load()
})

const phaseTone = computed<'success' | 'warning' | 'error' | 'neutral' | 'accent'>(() => {
  switch (update.phase) {
    case 'ready':
      return 'success'
    case 'available':
    case 'downloading':
    case 'installing':
      return 'accent'
    case 'error':
      return 'error'
    case 'up_to_date':
      return 'success'
    default:
      return 'neutral'
  }
})

const sizeLabel = computed(() => {
  if (update.totalBytes <= 0) return formatBytes(update.downloadedBytes)
  return `${formatBytes(update.downloadedBytes)} / ${formatBytes(update.totalBytes)}`
})

const lastCheckLabel = computed(() => {
  const secs = update.lastCheckSecs
  return secs === null ? '尚未检查' : `${formatRelative(Date.now() - secs * 1000)}检查`
})

async function onCheck(): Promise<void> {
  await update.check()
}

async function onDownload(): Promise<void> {
  await update.download()
}

async function onInstall(): Promise<void> {
  await update.install()
  notifications.info('已启动安装，桌面端即将退出并完成更新')
}

async function openRelease(): Promise<void> {
  const url = update.releaseUrl
  if (!url || !window.open(url, '_blank', 'noopener')) {
    notifications.warn('无法打开发布页面，请手动复制地址')
  }
}
</script>

<template>
  <div class="update-page">
    <Card title="程序更新" subtitle="从 GitHub Releases 检查新版本、后台下载并手动安装">
      <template #actions>
        <Chip :tone="phaseTone" dot>{{ update.phaseLabel }}</Chip>
        <Button size="sm" variant="secondary" :disabled="update.busy" :loading="update.phase === 'checking'" @click="onCheck">
          检查更新
        </Button>
      </template>

      <ErrorLine v-if="update.actionError" :error="update.actionError" />
      <p v-if="update.stateError" class="state-error">{{ update.stateError }}</p>

      <div class="rows">
        <StatusRow label="当前版本" :value="update.currentVersion" mono />
        <StatusRow label="最新版本" :value="update.latestVersion ?? '—'" mono />
        <StatusRow label="最近检查" :value="lastCheckLabel" />
      </div>

      <Callout v-if="update.phase === 'up_to_date'" tone="default" compact>
        已是最新版本，无需更新。
      </Callout>

      <Callout v-if="update.phase === 'available' && update.latestVersion" tone="accent" compact>
        发现新版本 {{ update.latestVersion }}。点击下方「下载更新」在后台获取安装包，下载完成后由你手动触发安装。
      </Callout>

      <div v-if="update.phase === 'downloading'" class="progress-block">
        <div class="progress-head">
          <span>正在后台下载更新包</span>
          <span class="mono">{{ update.progress }}%</span>
        </div>
        <div
          class="progress-track"
          role="progressbar"
          :aria-valuenow="update.progress"
          aria-valuemin="0"
          aria-valuemax="100"
        >
          <div class="progress-fill" :style="{ width: `${update.progress}%` }" />
        </div>
        <div class="progress-size mono">{{ sizeLabel }}</div>
      </div>

      <Callout v-if="update.phase === 'ready'" tone="accent" compact>
        更新包已下载并通过校验。点击下方「立即安装」后，桌面端会退出以替换程序；随后会自动重新登记服务并重启。
      </Callout>

      <Callout v-if="update.phase === 'installing'" tone="accent" compact>
        正在安装更新，桌面端即将退出。若 UAC 弹窗出现，请确认以完成服务重新登记。
      </Callout>

      <div v-if="update.notes" class="notes">
        <div class="notes__title">更新说明</div>
        <pre class="notes__body">{{ update.notes }}</pre>
      </div>

      <div class="actions">
        <Button v-if="update.canDownload" variant="primary" @click="onDownload">下载更新</Button>
        <Button v-if="update.canInstall" variant="primary" @click="onInstall">立即安装</Button>
        <Button v-if="update.releaseUrl" variant="ghost" @click="openRelease">查看发布页</Button>
        <Button v-if="update.phase === 'error'" variant="secondary" @click="onCheck">重试检查</Button>
      </div>
    </Card>
  </div>
</template>

<style scoped>
.update-page { max-width: 760px; }
.state-error { font-size: var(--font-sm); color: var(--error, #d64545); }
.rows { display: grid; gap: 2px; margin: 4px 0 18px; }
.mono { font-family: var(--font-mono, ui-monospace, monospace); }
.progress-block { margin: 16px 0; }
.progress-head { display: flex; justify-content: space-between; margin-bottom: 8px; font-size: 13px; }
.progress-track {
  height: 8px;
  border-radius: 999px;
  background: var(--surface-3, rgba(127, 127, 127, 0.18));
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  background: var(--accent, #4f7cff);
  transition: width 200ms ease;
}
.progress-size { margin-top: 6px; font-size: 12px; color: var(--text-subtle, #888); }
.notes { margin: 16px 0; }
.notes__title { font-size: 13px; font-weight: 600; margin-bottom: 6px; }
.notes__body {
  margin: 0;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--surface-2, rgba(127, 127, 127, 0.1));
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 220px;
  overflow: auto;
}
.actions { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 18px; }
</style>
