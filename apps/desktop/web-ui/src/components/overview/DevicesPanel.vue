<script setup lang="ts">
/**
 * 已配对设备：概览页内的一个区块（旧版是独立页面）。
 * 撤销不做乐观更新——服务端应用成功后再重读，事件同时让其他标签页收敛。
 */
import { onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useDevicesStore } from '@/stores/devices'
import { useNotificationsStore } from '@/stores/notifications'
import { useOverviewStore } from '@/stores/overview'
import { usePagination } from '@/composables/usePagination'
import { toAppError } from '@/utils/errors'
import type { DeviceItem } from '@/api/types'
import Card from '@/components/ui/Card.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import LoadingBlock from '@/components/ui/LoadingBlock.vue'
import ErrorLine from '@/components/ui/ErrorLine.vue'
import Pagination from '@/components/ui/Pagination.vue'
import DeviceRow from '@/components/devices/DeviceRow.vue'

const devices = useDevicesStore()
const notifications = useNotificationsStore()
const overview = useOverviewStore()
const { items, offset, limit, total, nextOffset, loading, error } = storeToRefs(devices)
const expanded = ref(false)

const { page, pageCount, rangeStart, rangeEnd } = usePagination({
  offset,
  limit,
  total,
  nextOffset,
  goToPage: (value) => devices.goToPage(value),
  nextPage: () => devices.nextPage(),
  prevPage: () => devices.prevPage(),
})

onMounted(() => {
  void devices.fetchPage(0)
})

async function revoke(device: DeviceItem): Promise<void> {
  const accepted = await notifications.confirm({
    title: '撤销设备令牌',
    message: `确定撤销「${device.device_name || device.id}」的令牌吗？`,
    detail: `id: ${device.id}\n撤销后服务端会立即断开该设备的控制连接与输入连接，该设备需要重新配对。`,
    confirmText: '立即撤销',
    cancelText: '取消',
    tone: 'danger',
  })
  if (!accepted) return
  try {
    await devices.revoke(device.id)
    await overview.refresh()
    notifications.info('已撤销该设备的访问权限')
  } catch (caught) {
    notifications.error(toAppError(caught).message)
  }
}

function toggle(): void {
  expanded.value = !expanded.value
  if (expanded.value && items.value.length === 0) void devices.fetchPage(0)
}
</script>

<template>
  <Card padding="none">
    <template #header>
      <button type="button" class="head" @click="toggle">
        <span class="head__title">
          已配对设备
          <span class="head__count">{{ total }}</span>
        </span>
        <span class="head__toggle">{{ expanded ? '收起' : '展开' }}</span>
      </button>
    </template>

    <template v-if="expanded">
      <LoadingBlock v-if="loading && items.length === 0" label="正在拉取设备列表…" />

      <EmptyState
        v-else-if="items.length === 0"
        title="暂无已配对设备"
        description="在移动端发起配对后，本页会出现待确认提示；确认完成后设备才会出现在这里。"
      />

      <ul v-else class="devices">
        <DeviceRow
          v-for="device in items"
          :key="device.id"
          :device="device"
          :busy="devices.revokingId === device.id"
          @revoke="revoke"
        />
      </ul>

      <Pagination
        v-if="items.length > 0"
        :page="page"
        :page-count="pageCount"
        :total="total"
        :range-start="rangeStart"
        :range-end="rangeEnd"
        :has-prev="devices.hasPrev"
        :has-next="devices.hasNext"
        :loading="loading"
        unit="台"
        @prev="devices.prevPage()"
        @next="devices.nextPage()"
      />
    </template>

    <p v-else class="collapsed-hint">展开后可查看令牌类型并撤销设备访问权限。</p>
  </Card>

  <ErrorLine :error="error" />
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  width: 100%;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-1);
  cursor: pointer;
  text-align: left;
}

.head__title {
  display: inline-flex;
  align-items: baseline;
  gap: var(--space-2);
  font-size: var(--font-lg);
  font-weight: var(--weight-semibold);
  letter-spacing: -0.01em;
}

.head__count {
  font-size: var(--font-sm);
  font-weight: var(--weight-normal);
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
}

.head__toggle {
  font-size: var(--font-sm);
  font-weight: var(--weight-normal);
  color: var(--text-2);
}

.devices {
  display: flex;
  flex-direction: column;
}

.collapsed-hint {
  padding: var(--space-4) var(--space-5);
  font-size: var(--font-sm);
  color: var(--text-3);
}
</style>
