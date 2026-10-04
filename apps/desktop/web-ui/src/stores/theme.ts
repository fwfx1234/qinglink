import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

/**
 * 主题：浅色（默认）/ 深色 / 跟随系统（§界面外观）。
 *
 * 未设置偏好时使用亮色，而不是跟随系统：这个界面默认应该亮，
 * 否则系统处于深色模式的机器上永远看不到亮色主题。
 *
 * 偏好只存在浏览器 localStorage 里，不经过桌面端接口：
 * - 管理界面是浏览器的页面，服务端不需要知道它长什么样；
 * - 首屏无需等待任何请求即可定主题（index.html 的内联脚本已做同一件事），
 *   否则首帧会闪一下另一个主题。
 */
export type ThemeMode = 'system' | 'light' | 'dark'
export type ResolvedTheme = 'light' | 'dark'

/** localStorage 键名；index.html 的内联脚本读同一个键。 */
export const STORAGE_KEY = 'qinglink.theme'

/** 未持久化任何偏好时的主题模式。 */
export const DEFAULT_MODE: ThemeMode = 'light'

const MODES: ThemeMode[] = ['system', 'light', 'dark']

/** 取系统偏好；无法探测时回退到亮色（默认主题）。 */
function systemTheme(): ResolvedTheme {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return 'light'
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
}

function storedMode(): ThemeMode {
  if (typeof window === 'undefined') return DEFAULT_MODE
  try {
    const value = window.localStorage.getItem(STORAGE_KEY)
    if (value && (MODES as string[]).includes(value)) return value as ThemeMode
  } catch {
    // 隐私模式或策略禁用 localStorage：当作未设置，使用默认亮色。
  }
  return DEFAULT_MODE
}

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>(storedMode())
  const system = ref<ResolvedTheme>(systemTheme())

  /** 实际生效的主题：`system` 模式下由系统偏好决定。 */
  const resolved = computed<ResolvedTheme>(() => (mode.value === 'system' ? system.value : mode.value))
  const isDark = computed(() => resolved.value === 'dark')

  /** 把当前状态写到 <html>，样式表据此切换令牌。 */
  function apply(): void {
    if (typeof document === 'undefined') return
    const root = document.documentElement
    root.dataset.theme = resolved.value
    root.dataset.themeMode = mode.value
  }

  function persist(): void {
    if (typeof window === 'undefined') return
    try {
      window.localStorage.setItem(STORAGE_KEY, mode.value)
    } catch {
      // 存不下不影响本次会话生效，只是刷新后回到默认。
    }
  }

  /** 在首次渲染前调用：同步系统偏好并写入 DOM。 */
  function start(): void {
    apply()
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return
    const query = window.matchMedia('(prefers-color-scheme: light)')
    query.addEventListener('change', (event) => {
      system.value = event.matches ? 'light' : 'dark'
      // 只有跟随系统时才影响实际主题，但 DOM 数据属性始终反映当前解析结果。
      apply()
    })
  }

  function setMode(next: ThemeMode): void {
    mode.value = next
    persist()
    apply()
  }

  /** 在浅色与深色之间切换，并把偏好固定下来（脱离跟随系统）。 */
  function toggle(): void {
    setMode(resolved.value === 'dark' ? 'light' : 'dark')
  }

  return { mode, system, resolved, isDark, start, setMode, toggle, apply }
})
