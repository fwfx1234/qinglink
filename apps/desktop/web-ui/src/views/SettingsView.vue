<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { useNotificationsStore } from '@/stores/notifications'
import { desktopApi } from '@/api/endpoints'
import { toAppError } from '@/utils/errors'
import type { SettingsData } from '@/api/types'
import Card from '@/components/ui/Card.vue'
import Toggle from '@/components/ui/Toggle.vue'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import Spinner from '@/components/ui/Spinner.vue'

const settings = useSettingsStore()
const notifications = useNotificationsStore()

const pendingAction = ref<'restart' | 'exit' | 'open' | null>(null)

interface SettingRow {
  key: keyof SettingsData
  label: string
  description: string
}

const rows: SettingRow[] = [
  { key: 'autostart', label: '开机启动', description: '随 Windows 启动自动运行桌面端' },
  { key: 'keep_awake', label: '保持唤醒', description: '阻止系统休眠与息屏；托盘菜单与这里共享同一状态' },
  { key: 'debug_log', label: '调试日志', description: '日志级别切到 debug，日志量会显著增加' },
]

onMounted(() => {
  void settings.load()
})

async function onToggle(key: keyof SettingsData, value: boolean): Promise<void> {
  try {
    await settings.toggle(key, value)
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
}

async function openInBrowser(): Promise<void> {
  pendingAction.value = 'open'
  try {
    const result = await desktopApi.open()
    notifications.info(`已在默认浏览器打开 ${result?.url ?? '本界面'}`)
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  } finally {
    pendingAction.value = null
  }
}

async function restartDesktop(): Promise<void> {
  const accepted = await notifications.confirm({
    title: '重启桌面端',
    message: '确定重启 QingLink 桌面进程吗？',
    detail: '本页面会与实时通道断开，服务恢复后自动重连。',
    confirmText: '重启',
    cancelText: '取消',
    tone: 'danger',
  })
  if (!accepted) return
  pendingAction.value = 'restart'
  try {
    await desktopApi.restart()
    notifications.warn('已请求重启，页面将在服务恢复后自动重连')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  } finally {
    pendingAction.value = null
  }
}

async function exitDesktop(): Promise<void> {
  const accepted = await notifications.confirm({
    title: '退出桌面端',
    message: '确定退出 QingLink 吗？退出后本界面将失去数据源。',
    detail: '托盘图标会消失；移动端连接是否受影响取决于服务进程是否仍在运行。',
    confirmText: '退出应用',
    cancelText: '取消',
    tone: 'danger',
  })
  if (!accepted) return
  pendingAction.value = 'exit'
  try {
    await desktopApi.exit()
    notifications.warn('已请求退出，界面即将失去数据源')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  } finally {
    pendingAction.value = null
  }
}

const savingKey = computed(() => settings.savingKey)
</script>

<template>
  <Card title="桌面端行为">
    <template #actions>
      <Spinner v-if="settings.loading" :size="12" tone="muted" />
      <Chip v-if="settings.saving" size="sm" tone="warning">保存中…</Chip>
      <Button size="sm" variant="ghost" :loading="settings.loading" @click="settings.load()">重新读取</Button>
    </template>

    <div class="rows">
      <Toggle
        v-for="row in rows"
        :key="row.key"
        :label="row.label"
        :description="row.description"
        :model-value="Boolean(settings.settings?.[row.key])"
        :loading="savingKey === row.key"
        :disabled="!settings.loaded || settings.saving"
        @update:model-value="onToggle(row.key, $event)"
      />
    </div>

    <p v-if="settings.error && !settings.error.serviceUnavailable" class="rows__error">
      <strong>{{ settings.error.code }}</strong> — {{ settings.error.message }}
    </p>
  </Card>

  <Card title="进程控制">
    <div class="actions">
      <div class="action">
        <p class="action__title">在浏览器中打开</p>
        <Button size="sm" :loading="pendingAction === 'open'" @click="openInBrowser">打开</Button>
      </div>

      <div class="action">
        <p class="action__title">重启桌面端</p>
        <Button size="sm" variant="secondary" :loading="pendingAction === 'restart'" @click="restartDesktop">
          重启
        </Button>
      </div>

      <div class="action action--danger">
        <p class="action__title">退出桌面端</p>
        <Button size="sm" variant="danger-ghost" :loading="pendingAction === 'exit'" @click="exitDesktop">退出</Button>
      </div>
    </div>
  </Card>
</template>

<style scoped>
.rows {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.rows__error {
  margin-top: var(--space-2);
  font-size: var(--font-sm);
  color: var(--error);
}

.actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.action {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-2);
}

.action--danger {
  border-color: var(--error-border);
}

.action__title {
  font-size: var(--font-md);
  color: var(--text-1);
}
</style>
