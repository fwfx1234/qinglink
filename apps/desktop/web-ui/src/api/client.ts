/**
 * 统一 fetch 封装：
 *  - 解析 §2.1 响应信封（ok 字段优先，不看 HTTP 状态码）
 *  - 把 §2.2 错误码映射成带 code 的 ApiError
 *  - 30s 超时（AbortController），超时映射为 TIMEOUT
 *  - 区分「无信封的原始响应」（图标 PNG / 日志文件下载）
 */

import type { ApiEnvelope, ApiErrorCode, ApiErrorBody } from './types'

export const API_BASE = '/api/admin/v1'

const DEFAULT_TIMEOUT_MS = 30_000

export class ApiError extends Error {
  readonly code: ApiErrorCode
  readonly status: number | null

  constructor(code: ApiErrorCode, message: string, status: number | null = null) {
    super(message)
    this.name = 'ApiError'
    this.code = code
    this.status = status
  }

  /** 服务未安装 / 未运行 —— 界面应展示引导态而非错误弹窗（§7.5） */
  get isServiceUnavailable(): boolean {
    return this.code === 'SERVICE_UNAVAILABLE'
  }

  /** 服务版本过旧 —— 引导「修复服务（升级）」 */
  get isUnsupported(): boolean {
    return this.code === 'UNSUPPORTED'
  }

  /** 桌面端身份未获服务授权 —— 引导「修复服务」重新登记 */
  get isUnauthorizedDesktop(): boolean {
    return this.code === 'UNAUTHORIZED_DESKTOP'
  }
}

export interface RequestOptions {
  method?: 'GET' | 'POST' | 'DELETE'
  query?: Record<string, string | number | boolean | null | undefined>
  body?: unknown
  timeoutMs?: number
  signal?: AbortSignal
}

export function buildQuery(
  query?: Record<string, string | number | boolean | null | undefined>,
): string {
  if (!query) return ''
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(query)) {
    if (value === null || value === undefined || value === '') continue
    params.set(key, String(value))
  }
  const text = params.toString()
  return text ? `?${text}` : ''
}

/** GET 接口的 id 走 query string，且必须 encodeURIComponent（§2.4） */
export function apiPath(path: string, query?: Record<string, string | number | boolean | null | undefined>): string {
  return `${API_BASE}${path}${buildQuery(query)}`
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError'
}

/**
 * 框架层拒绝（axum 的 405/415/422，以及 body 解析失败）**不套响应信封**，
 * 也不带错误码。这里按状态码给出可读结论，避免把内容协商失败报成
 * “响应不是合法 JSON”，那只会掩盖真正的问题。
 */
function frameworkFailure(status: number): ApiError {
  switch (status) {
    case 400:
      return new ApiError('BAD_REQUEST', '请求体不是合法 JSON', status)
    case 405:
      return new ApiError('BAD_REQUEST', '该接口不支持此请求方法', status)
    case 415:
      return new ApiError('BAD_REQUEST', '请求体需要 Content-Type: application/json', status)
    case 422:
      return new ApiError('BAD_REQUEST', '请求参数缺失或类型不合法', status)
    default:
      return new ApiError('INTERNAL', `请求失败（HTTP ${status}）`, status)
  }
}

async function parseFailure(response: Response): Promise<ApiError> {
  let body: unknown = null
  try {
    body = await response.json()
  } catch {
    body = null
  }
  if (body && typeof body === 'object' && 'error' in body) {
    const envelope = body as { error?: Partial<ApiErrorBody> }
    const code = envelope.error?.code
    const message = envelope.error?.message
    if (typeof code === 'string' && typeof message === 'string') {
      return new ApiError(code as ApiErrorCode, message, response.status)
    }
  }
  return frameworkFailure(response.status)
}

/** 发起请求并解包信封，返回 data。业务失败抛 ApiError。 */
export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', query, body, timeoutMs = DEFAULT_TIMEOUT_MS, signal } = options
  const controller = new AbortController()
  const timer = window.setTimeout(() => controller.abort(), timeoutMs)
  const onExternalAbort = () => controller.abort()
  signal?.addEventListener('abort', onExternalAbort)

  const headers: Record<string, string> = { Accept: 'application/json' }
  const init: RequestInit = { method, headers, signal: controller.signal }
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
    init.body = JSON.stringify(body)
  }

  let response: Response
  try {
    response = await fetch(apiPath(path, query), init)
  } catch (error) {
    if (isAbortError(error)) {
      throw new ApiError('TIMEOUT', '请求超时，请重试')
    }
    throw new ApiError('SERVICE_UNAVAILABLE', '无法连接本地管理服务，请确认 QingLink 正在运行')
  } finally {
    window.clearTimeout(timer)
    signal?.removeEventListener('abort', onExternalAbort)
  }

  let envelope: ApiEnvelope<T>
  try {
    envelope = (await response.json()) as ApiEnvelope<T>
  } catch {
    throw frameworkFailure(response.status)
  }

  if (!envelope || typeof envelope !== 'object' || !('ok' in envelope)) {
    throw new ApiError('INTERNAL', `响应信封缺少 ok 字段（HTTP ${response.status}）`, response.status)
  }
  if (envelope.ok === false) {
    throw await parseFailure(response)
  }
  return (envelope.data ?? null) as T
}

/** 下载类请求：返回 Blob，失败时仍按信封抛 ApiError。 */
export async function requestBlob(
  path: string,
  query?: Record<string, string | number | boolean | null | undefined>,
): Promise<Blob> {
  let response: Response
  try {
    response = await fetch(apiPath(path, query), { headers: { Accept: '*/*' } })
  } catch {
    throw new ApiError('SERVICE_UNAVAILABLE', '无法连接本地管理服务，请确认 QingLink 正在运行')
  }
  if (!response.ok) {
    throw await parseFailure(response)
  }
  return response.blob()
}
