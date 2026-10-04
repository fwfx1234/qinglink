<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { usePairingStore } from '@/stores/pairing'
import { useOverviewStore } from '@/stores/overview'
import { useNotificationsStore } from '@/stores/notifications'
import { toAppError } from '@/utils/errors'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import Chip from '@/components/ui/Chip.vue'
import StatusDot from '@/components/ui/StatusDot.vue'

const pairing = usePairingStore()
const overview = useOverviewStore()
const notifications = useNotificationsStore()

const pending = ref(false)
const now = ref(Date.now())
let ticker: number | null = null

onMounted(() => {
  ticker = window.setInterval(() => {
    now.value = Date.now()
  }, 1_000)
})

onBeforeUnmount(() => {
  if (ticker !== null) window.clearInterval(ticker)
})

const expiresLabel = computed(() => {
  const expiresAt = pairing.expiresAt
  if (expiresAt === null) return null
  const remaining = expiresAt * 1000 - now.value
  if (remaining <= 0) return '已过期，等待服务端清理'
  const seconds = Math.ceil(remaining / 1000)
  if (seconds < 60) return `${seconds} 秒后过期`
  return `${Math.ceil(seconds / 60)} 分钟后过期`
})

const pinDigits = computed(() => (pairing.pin || '').split(''))

async function cancel(): Promise<void> {
  const accepted = await notifications.confirm({
    title: '取消配对请求',
    message: `确定取消来自「${pairing.deviceName || pairing.deviceId}」的配对请求吗？`,
    detail: '取消后对端需要重新发起配对。',
    confirmText: '取消配对',
    cancelText: '保留请求',
    tone: 'danger',
  })
  if (!accepted) return
  pending.value = true
  try {
    await pairing.cancel()
    notifications.info('已请求取消配对，等待服务端确认')
    // 不乐观更新：pairing/pending 事件回流后卡片自动隐藏
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  } finally {
    pending.value = false
  }
}

function refresh(): void {
  void pairing.load()
  void overview.refresh()
}
</script>

<template>
  <Card v-if="pairing.hasPending" title="待确认配对">
    <template #actions>
      <Chip tone="warning" dot>等待确认</Chip>
      <Button size="sm" variant="ghost" :loading="pairing.loading" @click="refresh">刷新</Button>
    </template>

    <div class="pairing">
      <div class="pairing__device">
        <span class="pairing__name">{{ pairing.deviceName || '未命名设备' }}</span>
        <span class="pairing__id">{{ pairing.deviceId }}</span>
        <span v-if="expiresLabel" class="pairing__expiry">
          <StatusDot tone="warning" />
          {{ expiresLabel }}
        </span>
      </div>

      <div class="pairing__pin-block">
        <span class="pairing__label">配对码</span>
        <div class="pin" role="text" :aria-label="`配对码 ${pairing.pin}`">
          <span v-for="(digit, index) in pinDigits" :key="`${digit}-${index}`" class="pin__digit">{{ digit }}</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="pairing__footer">
        <Button variant="danger" size="sm" :loading="pending || pairing.cancelling" @click="cancel">
          取消配对请求
        </Button>
      </div>
    </template>
  </Card>
</template>

<style scoped>
.pairing {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: var(--space-5);
  align-items: center;
}

.pairing__device {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.pairing__label {
  font-size: var(--font-xs);
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.pairing__name {
  font-size: var(--font-lg);
  color: var(--text-1);
}

.pairing__id {
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  color: var(--text-2);
  word-break: break-all;
}

.pairing__expiry {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  margin-top: var(--space-1);
  font-size: var(--font-sm);
  color: var(--warning);
}

.pairing__pin-block {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  align-items: flex-start;
}

.pin {
  display: flex;
  gap: var(--space-2);
}

.pin__digit {
  width: 34px;
  height: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-2);
  background: var(--surface-2);
  font-family: var(--font-mono);
  font-size: var(--font-xl);
  color: var(--accent);
}

.pairing__footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-4);
  width: 100%;
  flex-wrap: wrap;
}

@media (max-width: 1180px) {
  .pairing {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
