/**
 * 响应信封、错误码与分页（docs/web-api.md §2.1 / §2.2 / §2.3）。
 *
 * 契约按职责拆分后本模块位于最底层：其余契约模块只引用这里的 ApiEnvelope / Page，
 * 本模块不依赖任何其他契约模块。
 */

/* ------------------------------------------------------------------ */
/* §2.1 / §2.2 响应信封与错误码                                          */
/* ------------------------------------------------------------------ */

export type ApiErrorCode =
  | 'BAD_REQUEST'
  | 'SERVICE_UNAVAILABLE'
  | 'UNSUPPORTED'
  | 'UNAUTHORIZED_DESKTOP'
  | 'NOT_FOUND'
  | 'CONFLICT'
  | 'TIMEOUT'
  | 'IO_ERROR'
  | 'INTERNAL'

export interface ApiErrorBody {
  code: ApiErrorCode
  message: string
}

export interface ApiSuccessEnvelope<T> {
  ok: true
  data?: T | null
}

export interface ApiFailureEnvelope {
  ok: false
  error: ApiErrorBody
}

export type ApiEnvelope<T> = ApiSuccessEnvelope<T> | ApiFailureEnvelope

/* ------------------------------------------------------------------ */
/* §2.3 分页                                                            */
/* ------------------------------------------------------------------ */

export interface Page<T> {
  items: T[]
  offset: number
  limit: number
  total: number
  next_offset: number | null
}
