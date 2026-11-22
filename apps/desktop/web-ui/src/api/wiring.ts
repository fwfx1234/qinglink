/**
 * 事件接线：把 WS 事件路由到各 store（§5.3 / §6）。
 * 只在 main.ts 启动时调用一次，视图不得重复注册。
 *
 * 两条原则：
 *  - 状态变化只由事件驱动，页面不轮询（§7.2）；
 *  - /overview 是聚合接口，多个事件在一块刷新窗口内只拉一次。
 */

import { useAppsStore } from '@/stores/apps'
import { useConnectionStore } from '@/stores/connection'
import { useDevicesStore } from '@/stores/devices'
import { useLogsStore } from '@/stores/logs'
import { useNotificationsStore } from '@/stores/notifications'
import { useOverviewStore } from '@/stores/overview'
import { usePairingStore } from '@/stores/pairing'
import { useServiceStore } from '@/stores/service'
import { useSettingsStore } from '@/stores/settings'
import { useUpdateStore } from '@/stores/update'
import type { ServerEvent } from '@/api/types'

const REASON_LABELS: Record<string, string> = {
  rename: '重命名',
  hidden: '可见性',
  delete: '删除',
}

/** 聚合接口的合并窗口：多个事件同时到达时只拉一次 /overview。 */
const OVERVIEW_COALESCE_MS = 300

export function installEventWiring(): void {
  const connection = useConnectionStore()
  const overview = useOverviewStore()
  const service = useServiceStore()
  const pairing = usePairingStore()
  const devices = useDevicesStore()
  const apps = useAppsStore()
  const logs = useLogsStore()
  const settings = useSettingsStore()
  const update = useUpdateStore()
  const notifications = useNotificationsStore()

  let overviewTimer: number | null = null
  let lastConnected = overview.service?.connected ?? false

  function scheduleOverviewRefresh(): void {
    if (overviewTimer !== null) return
    overviewTimer = window.setTimeout(() => {
      overviewTimer = null
      void overview.refresh()
    }, OVERVIEW_COALESCE_MS)
  }

  connection.onAny((event: ServerEvent) => {
    switch (event.channel) {
      case 'system': {
        if (event.event === 'shutdown') {
          notifications.warn(
            event.data.reason === 'restart' ? '桌面端正在重启，界面将在服务恢复后自动重连' : '桌面端正在退出',
          )
        } else if (event.event === 'settings') {
          settings.apply(event.data)
          overview.patchSettings(event.data)
        } else if (event.event === 'update_state') {
          update.apply(event.data)
        }
        return
      }
      case 'service': {
        if (event.event === 'status') {
          const connected = event.data.connected
          service.applyStatus(event.data)
          overview.patchService(event.data)
          if (connected !== lastConnected) {
            lastConnected = connected
            // SCM 侧状态只有在连接发生跳变时才会变，且没有别的事件会带来它。
            void service.loadInstallState()
          }
          if (connected && event.data.last_error) {
            notifications.warn(event.data.last_error)
          }
        } else if (event.event === 'install_state') {
          service.applyInstallState(event.data)
          overview.patchInstallState(event.data)
          if (event.data.busy === false && typeof event.data.result === 'string' && event.data.result.trim() !== '') {
            notifications.info(event.data.result)
            void service.loadStatus()
            scheduleOverviewRefresh()
          }
        }
        return
      }
      case 'devices': {
        if (event.event === 'changed') {
          void devices.refresh()
          scheduleOverviewRefresh()
        }
        return
      }
      case 'pairing': {
        if (event.event === 'pending') {
          pairing.apply(event.data.pairing ?? null)
          overview.patchPairing(event.data.pairing ?? null)
        }
        return
      }
      case 'apps': {
        if (event.event === 'catalog_changed') {
          const reason = event.data?.reason ?? 'catalog_changed'
          apps.onCatalogChanged(reason)
          scheduleOverviewRefresh()
          // resync 是丢帧后的自愈帧，扫描完成由列表自身的状态表达；
          // 只有用户发起的目录写操作值得提示一次。
          const label = REASON_LABELS[reason]
          if (label) notifications.info(`目录已更新（${label}）`)
        } else if (event.event === 'scan_started') {
          apps.onScanStarted(event.data.kind)
        }
        return
      }
      case 'logs': {
        if (event.event === 'entries') {
          logs.appendEntries(event.data.items ?? [])
        } else if (event.event === 'dropped') {
          logs.markDropped(event.data.count ?? 0)
        } else if (event.event === 'cleared') {
          logs.clearLocal()
          scheduleOverviewRefresh()
        }
        return
      }
      case 'notice': {
        if (event.event === 'toast') {
          notifications.push(event.data.level, event.data.message)
        }
        return
      }
      default:
        return
    }
  })

  // §7.4 只在“连接已建立”时重新拉取状态：首帧一次，此后每次重连一次。
  connection.onOpen(() => {
    void overview.refresh()
    void pairing.load()
    void service.loadInstallState()
    if (settings.loaded) void settings.load()
    void devices.refresh()
    if (logs.streaming) {
      void logs.load()
      void logs.loadStats()
    }
  })
}
