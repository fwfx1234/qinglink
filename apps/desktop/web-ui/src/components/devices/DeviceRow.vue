<script setup lang="ts">
import { computed, ref } from 'vue'
import type { DeviceItem } from '@/api/types'
import { copyText } from '@/utils/clipboard'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import Spinner from '@/components/ui/Spinner.vue'
import DataRow from '@/components/ui/DataRow.vue'

const props = defineProps<{
  device: DeviceItem
  busy: boolean
}>()

const emit = defineEmits<{
  (event: 'revoke', device: DeviceItem): void
}>()

/** `paired:` 前缀是含 device_id 的新式令牌，`legacy:` 是旧式令牌（§4.10） */
const tokenKind = computed(() => {
  if (props.device.id.startsWith('paired:')) {
    return { tone: 'success' as const, label: '新式令牌' }
  }
  if (props.device.id.startsWith('legacy:')) {
    return { tone: 'warning' as const, label: '旧式令牌' }
  }
  return { tone: 'neutral' as const, label: '未知前缀' }
})

/**
 * 令牌 id 是一长串哈希，铺在行里既挤又没用。行内只显示「前缀 + 尾部」便于区分，
 * 完整值放进 title，需要粘贴时用「复制 id」。
 */
const shortId = computed(() => {
  const id = props.device.id
  if (id.length <= 34) return id
  const separator = id.indexOf(':')
  if (separator < 0) return `${id.slice(0, 12)}…${id.slice(-8)}`
  return `${id.slice(0, separator + 1)}…${id.slice(-8)}`
})

const copied = ref(false)
let copiedTimer: ReturnType<typeof setTimeout> | null = null

async function copyId(): Promise<void> {
  copied.value = await copyText(props.device.id)
  if (copiedTimer) clearTimeout(copiedTimer)
  copiedTimer = setTimeout(() => {
    copied.value = false
  }, 1600)
}
</script>

<template>
  <DataRow>
    <template #leading>
      <span class="avatar" aria-hidden="true">
        <svg viewBox="0 0 20 20" width="16" height="16">
          <path
            d="M6 2.5h8v15H6v-15Zm2.8 12.6h2.4"
            fill="none"
            stroke="currentColor"
            stroke-width="1.3"
            stroke-linejoin="round"
          />
        </svg>
      </span>
    </template>

    <div class="line">
      <span class="name" :title="props.device.device_name">{{ props.device.device_name || '未命名设备' }}</span>
      <Chip size="sm" :tone="tokenKind.tone">{{ tokenKind.label }}</Chip>
    </div>
    <p class="id" :title="props.device.id">{{ shortId }}</p>

    <template #actions>
      <Spinner v-if="props.busy" :size="12" tone="muted" />
      <Button size="sm" variant="ghost" :title="copied ? '已复制' : '复制完整令牌 id'" @click="copyId">
        {{ copied ? '已复制' : '复制 id' }}
      </Button>
      <Button size="sm" variant="danger-ghost" :disabled="props.busy" @click="emit('revoke', props.device)">
        撤销令牌
      </Button>
    </template>
  </DataRow>
</template>

<style scoped>
.avatar {
  width: 32px;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-3);
  color: var(--text-2);
}

.line {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  min-width: 0;
}

.name {
  font-size: var(--font-md);
  font-weight: var(--weight-medium);
  color: var(--text-1);
}

.id {
  font-family: var(--font-mono);
  font-size: var(--font-xs);
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
