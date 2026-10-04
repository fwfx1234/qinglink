/** 可复用的超时看门狗：为「接口立即返回、真实结果靠事件回流」的操作兜底。 */

export class Watchdog {
  private readonly timers = new Map<string, number>()

  arm(key: string, timeoutMs: number, onTimeout: () => void): void {
    this.disarm(key)
    const timer = window.setTimeout(() => {
      this.timers.delete(key)
      onTimeout()
    }, timeoutMs)
    this.timers.set(key, timer)
  }

  disarm(key: string): void {
    const timer = this.timers.get(key)
    if (timer !== undefined) {
      window.clearTimeout(timer)
      this.timers.delete(key)
    }
  }

  has(key: string): boolean {
    return this.timers.has(key)
  }

  dispose(): void {
    for (const timer of this.timers.values()) {
      window.clearTimeout(timer)
    }
    this.timers.clear()
  }
}
