/**
 * QingLink 桌面管理接口契约类型。
 * 唯一权威来源：docs/web-api.md（本文件与之一一对应，不得自行发明字段）。
 *
 * 本文件是桶文件：契约按职责拆在 ./contracts/ 下，这里只做类型转出，因此
 * 既有的 `import type { X } from '@/api/types'` / `from './types'` 无需改动。
 * 类型定义一律写在 ./contracts/ 下，本文件不放任何定义。
 *
 *   envelope.ts    §2.1 - §2.3   响应信封、错误码、分页
 *   service.ts     §4.1 - §4.9   服务状态与能力、设置、桌面端动作结果
 *   devices.ts     §4.10 / §4.11 设备列表与撤销
 *   pairing.ts     §4.12 / §4.13 配对请求与取消
 *   catalog.ts     §4.14 - §4.20、§4.29 - §4.33 应用目录与图标
 *   logs.ts        §4.21 - §4.26 日志
 *   diagnostics.ts §4.27 / §4.28 诊断报告
 *   update.ts      §4.37 - §4.40 程序更新
 *   events.ts      §5           WebSocket 事件
 */

export type * from './contracts/envelope'
export type * from './contracts/service'
export type * from './contracts/devices'
export type * from './contracts/pairing'
export type * from './contracts/catalog'
export type * from './contracts/logs'
export type * from './contracts/diagnostics'
export type * from './contracts/update'
export type * from './contracts/events'
