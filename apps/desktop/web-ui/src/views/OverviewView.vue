<script setup lang="ts">
/**
 * 概览：服务状态、待确认配对、统计与已配对设备。
 *
 * 旧版把“仪表盘”和“设备管理”拆成两页，但设备数量通常只有一两台，
 * 拆页只增加一次导航；这里合并成一条纵向信息流。
 */
import { onMounted } from 'vue'
import { useOverviewStore } from '@/stores/overview'
import { useServiceStore } from '@/stores/service'
import { usePairingStore } from '@/stores/pairing'
import ServiceGate from '@/components/layout/ServiceGate.vue'
import ServiceStatusCard from '@/components/dashboard/ServiceStatusCard.vue'
import PairingCard from '@/components/dashboard/PairingCard.vue'
import StatsGrid from '@/components/dashboard/StatsGrid.vue'
import InstallHealthCard from '@/components/dashboard/InstallHealthCard.vue'
import DevicesPanel from '@/components/overview/DevicesPanel.vue'
import Button from '@/components/ui/Button.vue'

const overview = useOverviewStore()
const service = useServiceStore()
const pairing = usePairingStore()

onMounted(() => {
  // 首屏已在 main.ts 拉取 /overview；这里补齐配对与体检明细
  if (!overview.loaded) void overview.refresh()
  void pairing.load()
  void service.loadInstallState()
})

function refreshAll(): void {
  void overview.refresh()
  void service.load()
  void pairing.load()
}
</script>

<template>
  <ServiceGate>
    <ServiceStatusCard />
    <PairingCard />
    <StatsGrid />
    <DevicesPanel />
  </ServiceGate>

  <InstallHealthCard />

  <div class="actions">
    <Button size="sm" variant="ghost" :loading="overview.loading" @click="refreshAll">刷新概览</Button>
  </div>
</template>

<style scoped>
.actions {
  display: flex;
  justify-content: flex-end;
}
</style>
