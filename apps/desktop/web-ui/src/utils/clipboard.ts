/**
 * 复制文本到剪贴板。
 *
 * 管理界面运行在 `http://127.0.0.1:3722`，属于安全上下文，`navigator.clipboard`
 * 可用；但某些内嵌内核或降级环境下它会被禁用，所以保留 `execCommand` 兜底，
 * 让「复制路径」在两种情况下都能给出确定的结果。
 */
export async function copyText(text: string): Promise<boolean> {
  if (!text) return false

  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      // 继续走下面的兜底路径。
    }
  }

  try {
    const area = document.createElement('textarea')
    area.value = text
    area.setAttribute('readonly', '')
    area.style.position = 'fixed'
    area.style.top = '-1000px'
    area.style.opacity = '0'
    document.body.appendChild(area)
    area.select()
    const copied = document.execCommand('copy')
    document.body.removeChild(area)
    return copied
  } catch {
    return false
  }
}
