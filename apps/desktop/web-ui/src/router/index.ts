import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { ROUTE_NAMES } from '@/utils/constants'
import OverviewView from '@/views/OverviewView.vue'
import AppsView from '@/views/AppsView.vue'
import LogsView from '@/views/LogsView.vue'
import DiagnosticsView from '@/views/DiagnosticsView.vue'
import SettingsView from '@/views/SettingsView.vue'

/**
 * 使用 hash 路由：构建产物由 Rust 端本地 Web 服务按文件提供，
 * hash 模式下深链接无需服务端 SPA fallback，刷新任意页面都能正常加载。
 */
const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: ROUTE_NAMES.overview,
    component: OverviewView,
    meta: { title: '概览', description: '服务连接、待确认配对、目录统计与已配对设备' },
  },
  {
    path: '/apps',
    name: ROUTE_NAMES.apps,
    component: AppsView,
    meta: { title: '应用与游戏目录', description: '搜索、重命名、隐藏与移除本机条目' },
  },
  {
    path: '/logs',
    name: ROUTE_NAMES.logs,
    component: LogsView,
    meta: { title: '日志', description: '实时日志流、级别与时间过滤、清空与下载' },
  },
  {
    path: '/settings',
    name: ROUTE_NAMES.settings,
    component: SettingsView,
    meta: { title: '设置', description: '桌面端行为开关与进程控制' },
  },
  {
    path: '/diagnostics',
    name: ROUTE_NAMES.diagnostics,
    component: DiagnosticsView,
    meta: { title: '服务诊断', description: '链路状态、协议版本、自检报告与诊断包导出' },
  },
  // 旧版把设备管理单独成页；深链接仍要能用。
  { path: '/devices', redirect: { name: ROUTE_NAMES.overview } },
  { path: '/:pathMatch(.*)*', redirect: { name: ROUTE_NAMES.overview } },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
