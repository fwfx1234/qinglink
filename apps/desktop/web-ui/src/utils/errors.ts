/** 统一把未知异常收敛为可展示的文案与语义分类。 */

import { ApiError } from '@/api/client'
import type { ApiErrorCode } from '@/api/types'

export interface AppError {
  code: ApiErrorCode | 'UNKNOWN'
  message: string
  /** 服务未安装 / 未运行：应呈现引导态而非错误弹窗（§7.5） */
  serviceUnavailable: boolean
  /** 服务版本过旧：引导「修复服务（升级）」 */
  unsupported: boolean
  /** 桌面端身份未获服务授权：引导「修复服务」重新登记 */
  unauthorizedDesktop: boolean
  /** 状态冲突，例如扫描进行中（§4.20） */
  conflict: boolean
  /** 目标不存在，例如配对请求已过期（§4.13） */
  notFound: boolean
  /** 需要走「修复服务」的任一情况（协议过旧或身份未登记） */
  needsServiceRepair: boolean
}

export function toAppError(error: unknown): AppError {
  if (error instanceof ApiError) {
    const unauthorizedDesktop = error.code === 'UNAUTHORIZED_DESKTOP'
    return {
      code: error.code,
      message: error.message,
      serviceUnavailable: error.isServiceUnavailable,
      unsupported: error.isUnsupported,
      unauthorizedDesktop,
      conflict: error.code === 'CONFLICT',
      notFound: error.code === 'NOT_FOUND',
      needsServiceRepair: error.isUnsupported || unauthorizedDesktop,
    }
  }
  if (error instanceof Error) {
    return {
      code: 'UNKNOWN',
      message: error.message,
      serviceUnavailable: false,
      unsupported: false,
      unauthorizedDesktop: false,
      conflict: false,
      notFound: false,
      needsServiceRepair: false,
    }
  }
  return {
    code: 'UNKNOWN',
    message: '发生未知错误',
    serviceUnavailable: false,
    unsupported: false,
    unauthorizedDesktop: false,
    conflict: false,
    notFound: false,
    needsServiceRepair: false,
  }
}

export function errorMessage(error: unknown): string {
  return toAppError(error).message
}

export function isServiceUnavailable(error: unknown): boolean {
  return toAppError(error).serviceUnavailable
}
