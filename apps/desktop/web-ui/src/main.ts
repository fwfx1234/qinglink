import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { installEventWiring } from '@/api/wiring'
import { useConnectionStore } from '@/stores/connection'
import { useOverviewStore } from '@/stores/overview'
import { useThemeStore } from '@/stores/theme'
import './styles/tokens.css'
import './styles/light.css'
import './styles/dark.css'
import './styles/base.css'

const app = createApp(App)

app.use(createPinia())

// 主题必须在首次渲染前确定并写进 <html>，否则会先闪一帧默认主题。
// index.html 里的内联脚本已经做过一次（避免样式解析前的白屏），这里接管后续状态。
useThemeStore().start()

// 组件渲染异常不应让整页变白：记录到控制台，界面继续降级渲染
app.config.errorHandler = (error, _instance, info) => {
  console.error(`[QingLink] 组件渲染异常（${info}）`, error)
}

// 事件接线必须在首次渲染前完成，避免丢失首帧事件
installEventWiring()

app.use(router)

// §6.1 首屏：GET /overview + WS 订阅；日志仅在日志页可见时才拉取。
// 重连后的补拉统一由 api/wiring.ts 的 onOpen 负责，这里不重复挂钩子。
const overview = useOverviewStore()
void overview.refresh()

const connection = useConnectionStore()
connection.start()

app.mount('#app')
