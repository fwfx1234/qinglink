/**
 * WebSocket 事件客户端（§5）。
 *  - 统一信封解析 {"channel","event","data"}
 *  - 频道订阅 / 取消订阅（logs 必须显式订阅）
 *  - 指数退避重连：1s → 2s → 4s → 8s → 15s 封顶（§7.4）
 *  - 心跳 ping + pong 往返延迟；长时间无响应视为半开连接并强制重连
 *  - 每次成功打开递增 generation，供上层只在“真正重连”时重新拉取状态
 * 本模块只管传输与分发；状态归 store。
 */

import type { ClientCommand, EventChannel, ServerEvent } from './types'

export const EVENTS_PATH = '/api/admin/v1/events'

/** §5.2 默认订阅除 logs 外的全部频道 */
export const DEFAULT_CHANNELS: EventChannel[] = ['system', 'service', 'devices', 'pairing', 'apps', 'notice']

/** §7.4 退避序列，末位封顶 */
export const BACKOFF_MS = [1_000, 2_000, 4_000, 8_000, 15_000] as const

const PING_INTERVAL_MS = 25_000

/** 连续两个心跳周期收不到任何帧就认为连接已半开（休眠 / 换网）。 */
const PONG_DEADLINE_MS = 70_000

export type EventSocketState = 'idle' | 'connecting' | 'open' | 'reconnecting' | 'closed'

export interface ConnectionSnapshot {
  state: EventSocketState
  attempt: number
  /** 距下一次重连的毫秒数（调度时的值；倒计时请用 retryDeadlineAt） */
  nextRetryMs: number | null
  /** 下一次重连的绝对时刻（Unix 毫秒）；配合本地 ticker 渲染倒计时 */
  retryDeadlineAt: number | null
  /** 最近一次成功收到 hello 的时间（Unix 毫秒） */
  lastHelloAt: number | null
  /** 最近一次成功建立连接的时间（Unix 毫秒） */
  openedAt: number | null
  /** 每成功打开一次加一：用于区分首帧与重连 */
  generation: number
  /** 最近一次 ping/pong 往返毫秒数 */
  pingMillis: number | null
  lastError: string | null
}

export type EventHandler = (event: ServerEvent) => void
export type StateHandler = (snapshot: ConnectionSnapshot) => void

function eventsUrl(): string {
  const scheme = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${scheme}//${window.location.host}${EVENTS_PATH}`
}

function backoffFor(attempt: number): number {
  const index = Math.min(Math.max(attempt - 1, 0), BACKOFF_MS.length - 1)
  return BACKOFF_MS[index] as number
}

function sameSnapshot(left: ConnectionSnapshot, right: ConnectionSnapshot): boolean {
  return (
    left.state === right.state &&
    left.attempt === right.attempt &&
    left.nextRetryMs === right.nextRetryMs &&
    left.retryDeadlineAt === right.retryDeadlineAt &&
    left.lastHelloAt === right.lastHelloAt &&
    left.openedAt === right.openedAt &&
    left.generation === right.generation &&
    left.pingMillis === right.pingMillis &&
    left.lastError === right.lastError
  )
}

export class EventClient {
  private socket: WebSocket | null = null
  private readonly handlers = new Set<EventHandler>()
  private readonly stateHandlers = new Set<StateHandler>()
  private readonly channels = new Set<EventChannel>(DEFAULT_CHANNELS)
  private retryTimer: number | null = null
  private pingTimer: number | null = null
  private attempt = 0
  private manuallyClosed = false
  /** 已发出 ping 但还没收到任何帧的时刻；用于延迟与半开判定 */
  private awaitingPongSince: number | null = null
  private snapshot: ConnectionSnapshot = {
    state: 'idle',
    attempt: 0,
    nextRetryMs: null,
    retryDeadlineAt: null,
    lastHelloAt: null,
    openedAt: null,
    generation: 0,
    pingMillis: null,
    lastError: null,
  }

  getSnapshot(): ConnectionSnapshot {
    return { ...this.snapshot }
  }

  onEvent(handler: EventHandler): () => void {
    this.handlers.add(handler)
    return () => this.handlers.delete(handler)
  }

  onState(handler: StateHandler): () => void {
    this.stateHandlers.add(handler)
    handler(this.getSnapshot())
    return () => this.stateHandlers.delete(handler)
  }

  connect(): void {
    this.manuallyClosed = false
    if (this.socket && (this.socket.readyState === WebSocket.OPEN || this.socket.readyState === WebSocket.CONNECTING)) {
      return
    }
    // A pending backoff timer would otherwise open a second socket whose
    // handlers are never detached.
    this.clearRetryTimer()
    this.openSocket()
  }

  disconnect(): void {
    this.manuallyClosed = true
    this.clearTimers()
    const socket = this.socket
    this.socket = null
    if (socket) {
      socket.onopen = null
      socket.onmessage = null
      socket.onerror = null
      socket.onclose = null
      if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
        socket.close()
      }
    }
    this.patch({ state: 'closed', nextRetryMs: null, retryDeadlineAt: null, attempt: this.attempt })
  }

  subscribe(channels: EventChannel[]): void {
    const added = channels.filter((channel) => !this.channels.has(channel))
    if (added.length === 0) return
    added.forEach((channel) => this.channels.add(channel))
    this.send({ action: 'subscribe', channels: added })
  }

  unsubscribe(channels: EventChannel[]): void {
    const removed = channels.filter((channel) => this.channels.has(channel))
    if (removed.length === 0) return
    removed.forEach((channel) => this.channels.delete(channel))
    this.send({ action: 'unsubscribe', channels: removed })
  }

  subscribedChannels(): EventChannel[] {
    return [...this.channels]
  }

  private openSocket(): void {
    this.patch({ state: this.attempt === 0 ? 'connecting' : 'reconnecting', attempt: this.attempt, nextRetryMs: null, retryDeadlineAt: null })

    let socket: WebSocket
    try {
      socket = new WebSocket(eventsUrl())
    } catch (error) {
      this.patch({ lastError: error instanceof Error ? error.message : '无法创建 WebSocket' })
      this.scheduleReconnect()
      return
    }
    this.socket = socket

    socket.onopen = () => {
      this.attempt = 0
      this.patch({
        state: 'open',
        attempt: 0,
        nextRetryMs: null,
        retryDeadlineAt: null,
        lastError: null,
        openedAt: Date.now(),
        generation: this.snapshot.generation + 1,
      })
      this.send({ action: 'subscribe', channels: [...this.channels] })
      this.startPing()
    }

    socket.onmessage = (message: MessageEvent<string>) => {
      // Any frame proves the socket is alive, not just a pong.
      const pingSentAt = this.awaitingPongSince
      this.awaitingPongSince = null
      let parsed: unknown
      try {
        parsed = JSON.parse(message.data)
      } catch {
        return
      }
      if (!parsed || typeof parsed !== 'object') return
      const candidate = parsed as { channel?: unknown; event?: unknown }
      if (typeof candidate.channel !== 'string' || typeof candidate.event !== 'string') return
      const event = parsed as ServerEvent
      if (event.channel === 'system' && event.event === 'hello') {
        this.patch({ lastHelloAt: Date.now() })
      }
      if (event.channel === 'system' && event.event === 'pong' && pingSentAt !== null) {
        this.patch({ pingMillis: Date.now() - pingSentAt })
      }
      for (const handler of [...this.handlers]) {
        handler(event)
      }
    }

    socket.onerror = () => {
      this.patch({ lastError: 'WebSocket 连接错误' })
    }

    socket.onclose = (event: CloseEvent) => {
      this.stopPing()
      this.socket = null
      if (this.manuallyClosed) {
        this.patch({ state: 'closed', nextRetryMs: null, retryDeadlineAt: null })
        return
      }
      this.patch({
        lastError:
          event.code === 1006 || event.code === 1000
            ? '与本地管理服务的连接已断开'
            : `连接关闭（code ${event.code}）`,
      })
      this.scheduleReconnect()
    }
  }

  private scheduleReconnect(): void {
    if (this.manuallyClosed) return
    this.attempt += 1
    const delay = backoffFor(this.attempt)
    this.patch({
      state: 'reconnecting',
      attempt: this.attempt,
      nextRetryMs: delay,
      retryDeadlineAt: Date.now() + delay,
    })
    this.retryTimer = window.setTimeout(() => {
      this.retryTimer = null
      if (!this.manuallyClosed) this.openSocket()
    }, delay)
  }

  /** Close a socket that stopped answering and fall back into the retry loop. */
  private forceReconnect(reason: string): void {
    this.stopPing()
    const socket = this.socket
    this.socket = null
    if (socket) {
      socket.onopen = null
      socket.onmessage = null
      socket.onerror = null
      socket.onclose = null
      if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
        socket.close()
      }
    }
    this.patch({ lastError: reason })
    this.scheduleReconnect()
  }

  private startPing(): void {
    this.stopPing()
    this.awaitingPongSince = null
    this.pingTimer = window.setInterval(() => {
      const pending = this.awaitingPongSince
      if (pending !== null) {
        // 上一轮 ping 之后一个帧都没到：保持起点不变，等真正的超时。
        // （每轮重置起点会让判定永远达不到 PONG_DEADLINE_MS。）
        if (Date.now() - pending > PONG_DEADLINE_MS) {
          this.forceReconnect('实时通道长时间无响应，正在重连')
        }
        return
      }
      this.awaitingPongSince = Date.now()
      if (!this.send({ action: 'ping' })) {
        this.awaitingPongSince = null
      }
    }, PING_INTERVAL_MS)
  }

  private stopPing(): void {
    if (this.pingTimer !== null) {
      window.clearInterval(this.pingTimer)
      this.pingTimer = null
    }
    this.awaitingPongSince = null
  }

  private clearRetryTimer(): void {
    if (this.retryTimer !== null) {
      window.clearTimeout(this.retryTimer)
      this.retryTimer = null
    }
  }

  private clearTimers(): void {
    this.stopPing()
    this.clearRetryTimer()
  }

  private send(command: ClientCommand): boolean {
    const socket = this.socket
    if (!socket || socket.readyState !== WebSocket.OPEN) return false
    socket.send(JSON.stringify(command))
    return true
  }

  private patch(partial: Partial<ConnectionSnapshot>): void {
    const next = { ...this.snapshot, ...partial }
    if (sameSnapshot(next, this.snapshot)) return
    this.snapshot = next
    const snapshot = this.getSnapshot()
    for (const handler of [...this.stateHandlers]) {
      handler(snapshot)
    }
  }
}

export const eventClient = new EventClient()
