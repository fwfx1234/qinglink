import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import { eventClient } from '@/api/events'
import type { ConnectionSnapshot, EventSocketState } from '@/api/events'
import type { EventChannel, ServerEvent, SystemShutdownEvent } from '@/api/types'

export type EventListener = (event: ServerEvent) => void
export type ChannelListener = (event: ServerEvent) => void
export type OpenListener = (snapshot: ConnectionSnapshot) => void

/** WS 连接状态与事件分发中枢（§5）。 */
export const useConnectionStore = defineStore('connection', () => {
  const state = ref<EventSocketState>('idle')
  const attempt = ref(0)
  const nextRetryMs = ref<number | null>(null)
  const retryDeadlineAt = ref<number | null>(null)
  /** 由 retryDeadlineAt 派生的实时倒计时（每 500ms 刷新） */
  const retryInMs = ref<number | null>(null)
  const lastHelloAt = ref<number | null>(null)
  const lastError = ref<string | null>(null)
  const openedAt = ref<number | null>(null)
  /** 每成功连接一次加一；订阅方据此只在“真重连”时重新拉取状态（§7.4） */
  const generation = ref(0)
  const pingMillis = ref<number | null>(null)
  const shutdown = shallowRef<SystemShutdownEvent | null>(null)

  const channelListeners = new Map<EventChannel, Set<ChannelListener>>()
  const anyListeners = new Set<EventListener>()
  const connectionListeners = new Set<(snapshot: ConnectionSnapshot) => void>()
  const openListeners = new Set<OpenListener>()
  let lastGeneration = 0
  let detachState: (() => void) | null = null
  let detachEvent: (() => void) | null = null
  let retryTicker: number | null = null

  const isOpen = computed(() => state.value === 'open')
  const isReconnecting = computed(() => state.value === 'reconnecting' || state.value === 'connecting')
  const isClosed = computed(() => state.value === 'closed')

  const statusLabel = computed(() => {
    switch (state.value) {
      case 'open':
        return '实时连接正常'
      case 'connecting':
        return '正在连接实时通道…'
      case 'reconnecting': {
        const seconds = Math.max(1, Math.round((retryInMs.value ?? 0) / 1000))
        return `连接中断，${seconds}s 后重试（第 ${attempt.value} 次）`
      }
      case 'closed':
        return '实时通道已断开'
      default:
        return '实时通道未启动'
    }
  })

  const tone = computed<'success' | 'warning' | 'error' | 'neutral'>(() => {
    if (state.value === 'open') return 'success'
    if (state.value === 'connecting' || state.value === 'reconnecting') return 'warning'
    if (state.value === 'closed') return 'error'
    return 'neutral'
  })

  function syncRetryTicker(): void {
    if (retryDeadlineAt.value === null || state.value !== 'reconnecting') {
      if (retryTicker !== null) {
        window.clearInterval(retryTicker)
        retryTicker = null
      }
      retryInMs.value = null
      return
    }
    retryInMs.value = Math.max(0, retryDeadlineAt.value - Date.now())
    if (retryTicker === null) {
      retryTicker = window.setInterval(() => {
        if (retryDeadlineAt.value === null) {
          retryInMs.value = null
          return
        }
        retryInMs.value = Math.max(0, retryDeadlineAt.value - Date.now())
      }, 500)
    }
  }

  function handleEvent(event: ServerEvent): void {
    if (event.channel === 'system' && event.event === 'hello') {
      lastHelloAt.value = Date.now()
      // A fresh hello means the previous shutdown/restart notice is over.
      shutdown.value = null
    }
    for (const listener of [...anyListeners]) {
      listener(event)
    }
    const listeners = channelListeners.get(event.channel)
    if (listeners) {
      for (const listener of [...listeners]) {
        listener(event)
      }
    }
  }

  function applySnapshot(snapshot: ConnectionSnapshot): void {
    state.value = snapshot.state
    attempt.value = snapshot.attempt
    nextRetryMs.value = snapshot.nextRetryMs
    retryDeadlineAt.value = snapshot.retryDeadlineAt
    lastHelloAt.value = snapshot.lastHelloAt
    lastError.value = snapshot.lastError
    openedAt.value = snapshot.openedAt
    generation.value = snapshot.generation
    pingMillis.value = snapshot.pingMillis
    syncRetryTicker()
    const opened = snapshot.state === 'open' && snapshot.generation !== lastGeneration
    if (opened) lastGeneration = snapshot.generation
    for (const listener of [...connectionListeners]) {
      listener(snapshot)
    }
    if (opened) {
      for (const listener of [...openListeners]) {
        listener(snapshot)
      }
    }
  }

  /** 订阅某个频道的事件；返回取消订阅函数 */
  function on(channel: EventChannel, listener: ChannelListener): () => void {
    let listeners = channelListeners.get(channel)
    if (!listeners) {
      listeners = new Set<ChannelListener>()
      channelListeners.set(channel, listeners)
    }
    listeners.add(listener)
    return () => {
      listeners?.delete(listener)
    }
  }

  /** 订阅所有频道的事件 */
  function onAny(listener: EventListener): () => void {
    anyListeners.add(listener)
    return () => {
      anyListeners.delete(listener)
    }
  }

  /** 订阅连接状态快照变化（含同一状态下的字段更新） */
  function onStateChange(listener: (snapshot: ConnectionSnapshot) => void): () => void {
    connectionListeners.add(listener)
    return () => {
      connectionListeners.delete(listener)
    }
  }

  /**
   * 订阅“连接已建立”。首次打开也会触发一次；此后每次重连各触发一次。
   * 需要重新拉取状态的订阅方应使用它，而不是 onStateChange——后者在
   * 握手时间、延迟等字段变化时也会通知，会造成重复拉取。
   */
  function onOpen(listener: OpenListener): () => void {
    openListeners.add(listener)
    return () => {
      openListeners.delete(listener)
    }
  }

  function subscribe(channels: EventChannel[]): void {
    eventClient.subscribe(channels)
  }

  function unsubscribe(channels: EventChannel[]): void {
    eventClient.unsubscribe(channels)
  }

  function subscribedChannels(): EventChannel[] {
    return eventClient.subscribedChannels()
  }

  function start(): void {
    if (!detachState) {
      detachState = eventClient.onState(applySnapshot)
    }
    if (!detachEvent) {
      detachEvent = eventClient.onEvent(handleEvent)
    }
    eventClient.connect()
  }

  function stop(): void {
    eventClient.disconnect()
  }

  function resetShutdown(): void {
    shutdown.value = null
  }

  return {
    state,
    attempt,
    nextRetryMs,
    retryDeadlineAt,
    retryInMs,
    lastHelloAt,
    lastError,
    openedAt,
    generation,
    pingMillis,
    shutdown,
    isOpen,
    isReconnecting,
    isClosed,
    statusLabel,
    tone,
    on,
    onAny,
    onStateChange,
    onOpen,
    subscribe,
    unsubscribe,
    subscribedChannels,
    start,
    stop,
    resetShutdown,
  }
})
