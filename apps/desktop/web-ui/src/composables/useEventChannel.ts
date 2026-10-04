import { onBeforeUnmount, onMounted, type Ref } from 'vue'
import { useConnectionStore } from '@/stores/connection'
import type { EventChannel } from '@/api/types'

/**
 * 频道订阅范围管理：挂载时订阅、卸载时退订。
 * 事件到 store 的路由统一在 api/wiring.ts 中完成，视图不重复注册处理器。
 * logs 频道默认不订阅，必须由日志页显式加入（§5.2）。
 */
export function useChannelSubscription(channels: Ref<EventChannel[]> | EventChannel[]): void {
  const connection = useConnectionStore()

  onMounted(() => {
    connection.subscribe(Array.isArray(channels) ? channels : channels.value)
  })

  onBeforeUnmount(() => {
    connection.unsubscribe(Array.isArray(channels) ? channels : channels.value)
  })
}
