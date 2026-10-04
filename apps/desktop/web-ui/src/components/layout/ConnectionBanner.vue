<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useConnectionStore } from '@/stores/connection'
import StatusDot from '@/components/ui/StatusDot.vue'
import Button from '@/components/ui/Button.vue'

const connection = useConnectionStore()
const { shutdown, isReconnecting, isClosed, lastError, statusLabel } = storeToRefs(connection)

const visible = computed(() => isReconnecting.value || isClosed.value || shutdown.value !== null)

const tone = computed(() => {
  if (shutdown.value) return 'warning'
  return isClosed.value ? 'error' : 'warning'
})

const message = computed(() => {
  if (shutdown.value) {
    return shutdown.value.data.reason === 'restart'
      ? '桌面端正在重启，界面会自动重连。'
      : '桌面端正在退出，本界面即将失去数据源。'
  }
  if (isClosed.value) return '实时通道已断开，正在等待重连。'
  return statusLabel.value
})

/** 立即重连：先断开再连接，跳过剩余退避时间。 */
function reconnect(): void {
  connection.stop()
  connection.start()
}
</script>

<template>
  <div v-if="visible" class="banner" :class="`banner--${tone}`">
    <span class="banner__status">
      <StatusDot :tone="tone" :pulse="isReconnecting" />
      <strong>{{ tone === 'error' ? '连接中断' : '连接异常' }}</strong>
    </span>
    <span class="banner__text">{{ message }}</span>
    <span v-if="lastError" class="banner__detail">{{ lastError }}</span>
    <Button v-if="!shutdown" size="sm" variant="ghost" class="banner__action" @click="reconnect">
      立即重连
    </Button>
  </div>
</template>

<style scoped>
.banner {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-6);
  border-bottom: 1px solid var(--border-1);
  font-size: var(--font-sm);
  flex-wrap: wrap;
}

/* 提示条用柔和衬底 + 左侧色条，文字保持可读的标准前景色 */
.banner--warning {
  background: var(--warning-soft);
}

.banner--warning .banner__status {
  color: var(--warning);
}

.banner--error {
  background: var(--error-soft);
}

.banner--error .banner__status {
  color: var(--error);
}

.banner__status {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  font-weight: var(--weight-medium);
}

.banner__text {
  color: var(--text-1);
}

.banner__detail {
  color: var(--text-2);
}

.banner__detail::before {
  content: '· ';
}

.banner__action {
  margin-left: auto;
}
</style>
