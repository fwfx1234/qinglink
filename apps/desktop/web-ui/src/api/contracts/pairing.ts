/**
 * 配对请求与取消（docs/web-api.md §4.2 的 pairing 字段、§4.12 / §4.13）。
 *
 * PairingRequest 既内嵌在 overview 与配对信封里，也用于 §5 的 pairing 事件，
 * 因此单独成模块，供 service.ts / events.ts 引用。
 */

/* ------------------------------------------------------------------ */
/* §4.12 / §4.13 配对                                                   */
/* ------------------------------------------------------------------ */

export interface PairingRequest {
  request_id: string
  device_id: string
  device_name: string
  pin: string
  expires_at_unix: number
}

export interface PairingEnvelope {
  pairing: PairingRequest | null
}

export interface CancelPairingResult {
  cancelled: boolean
}
