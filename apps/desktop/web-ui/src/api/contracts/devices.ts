/**
 * 设备列表与撤销（docs/web-api.md §4.10 / §4.11）。
 */

import type { Page } from './envelope'

/* ------------------------------------------------------------------ */
/* §4.10 / §4.11 设备                                                   */
/* ------------------------------------------------------------------ */

/**
 * 后端实际形状：`id` 已经带 `paired:` / `legacy:` 前缀，必须原样回传撤销。
 * （docs 早期写作 device_id + kind，以后端实现为准。）
 */
export interface DeviceItem {
  id: string
  device_name: string
}

export type DevicePage = Page<DeviceItem>

export interface RevokeResult {
  revoked: boolean
}
