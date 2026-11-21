/* QingLink 本地管理台 · 页面切换与演示数据
 *
 * 当前为界面阶段：数据为演示用的静态样例，接口接入后由 fetch 替换；
 * 结构与 docs/design/screens/A1~A4 一一对应。
 */

/* ---------------- 路由 ---------------- */
const VIEWS = ['overview', 'apps', 'logs', 'diagnostics', 'settings'];

function showView(name) {
  const target = VIEWS.includes(name) ? name : 'overview';
  document.querySelectorAll('.view').forEach((el) => {
    el.classList.toggle('active', el.id === `view-${target}`);
  });
  document.querySelectorAll('.nav-item').forEach((el) => {
    el.classList.toggle('active', el.dataset.view === target);
  });
}

window.addEventListener('hashchange', () => showView(location.hash.slice(1)));
showView(location.hash.slice(1) || 'overview');

/* ---------------- 提示条 ---------------- */
let toastTimer = null;
function toast(message) {
  const el = document.getElementById('toast');
  el.textContent = message;
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('show'), 2000);
}

document.addEventListener('click', (event) => {
  const trigger = event.target.closest('[data-toast]');
  if (trigger) toast(trigger.dataset.toast);
});

document.getElementById('start-pairing').addEventListener('click', () => {
  toast('已发起配对：请在手机上输入 6 位配对码');
});

/* ---------------- A2 应用目录 ---------------- */
const APPS = [
  { name: '赛博朋克 2077', kind: 'game', last: '今天 11:02', pinned: true, badge: '赛', tone: 'y' },
  { name: '原神', kind: 'game', last: '昨天 20:15', pinned: false, badge: '原', tone: 'g' },
  { name: 'Visual Studio Code', kind: 'app', last: '5 天前', pinned: false, badge: 'VS', tone: 'b' },
  { name: 'Steam', kind: 'app', last: '3 天前', pinned: false, badge: 'S', tone: 'n' },
  { name: '文件资源管理器', kind: 'app', last: '上周', pinned: true, badge: '文', tone: 'y' },
  { name: '浏览器', kind: 'app', last: '上周', pinned: false, badge: '浏', tone: 'b' },
  { name: 'Windows 终端', kind: 'app', last: '本月', pinned: false, badge: '>_', tone: 'n' },
  { name: 'Photoshop 2024', kind: 'app', last: '上月', pinned: false, badge: 'Ps', tone: 'b' },
  { name: 'OBS Studio', kind: 'app', last: '上月', pinned: false, badge: 'OBS', tone: 'v' },
  { name: '艾尔登法环', kind: 'game', last: '今天 09:12', pinned: false, badge: '艾', tone: 'y' },
  { name: '黑神话：悟空', kind: 'game', last: '昨天 22:40', pinned: false, badge: '黑', tone: 'y' },
  { name: '星露谷物语', kind: 'game', last: '3 天前', pinned: false, badge: '星', tone: 'g' },
  { name: '双人成行', kind: 'game', last: '上周', pinned: false, badge: '双', tone: 'v' },
  { name: '极限竞速：地平线 5', kind: 'game', last: '上周', pinned: false, badge: '极', tone: 'b' },
  { name: '空洞骑士', kind: 'game', last: '本月', pinned: false, badge: '空', tone: 'n' },
  { name: '文明 VI', kind: 'game', last: '上月', pinned: false, badge: '文', tone: 'g' },
  { name: 'Figma', kind: 'app', last: '今天 08:30', pinned: false, badge: 'F', tone: 'v' },
  { name: '微信', kind: 'app', last: '今天 08:12', pinned: true, badge: '微', tone: 'g' },
  { name: '网易云音乐', kind: 'app', last: '昨天 19:02', pinned: false, badge: '网', tone: 'r' },
  { name: '剪映专业版', kind: 'app', last: '2 天前', pinned: false, badge: '剪', tone: 'g' },
  { name: 'Office Word', kind: 'app', last: '3 天前', pinned: false, badge: 'W', tone: 'b' },
  { name: 'Notion', kind: 'app', last: '4 天前', pinned: false, badge: 'N', tone: 'n' },
  { name: 'Docker Desktop', kind: 'app', last: '上周', pinned: false, badge: 'D', tone: 'b' },
  { name: 'Postman', kind: 'app', last: '上月', pinned: false, badge: 'P', tone: 'r' },
];

const PAGE_SIZE = 7;
const appState = { keyword: '', kind: 'all', page: 1 };

function filteredApps() {
  const keyword = appState.keyword.trim().toLowerCase();
  return APPS.filter((app) => {
    const kindOk = appState.kind === 'all' || app.kind === appState.kind;
    const textOk = !keyword || app.name.toLowerCase().includes(keyword);
    return kindOk && textOk;
  });
}

function renderApps() {
  const list = filteredApps();
  const pages = Math.max(1, Math.ceil(list.length / PAGE_SIZE));
  appState.page = Math.min(appState.page, pages);
  const start = (appState.page - 1) * PAGE_SIZE;
  const rows = list.slice(start, start + PAGE_SIZE);

  const body = document.getElementById('apps-body');
  body.innerHTML = rows
    .map(
      (app) => `
      <tr>
        <td class="strong">
          <span class="app-cell">
            <span class="app-badge tone-${app.tone}">${app.badge}</span>
            <span>${app.name}</span>
          </span>
        </td>
        <td>${app.kind === 'game' ? '游戏' : '应用'}</td>
        <td>${app.last}</td>
        <td class="col-end">${app.pinned ? '<span class="pin">已固定</span>' : '<span class="dash">—</span>'}</td>
        <td class="col-end"><button class="btn link brand" type="button" data-toast="打开「${app.name}」的管理面板">管理</button></td>
      </tr>`,
    )
    .join('');

  document.getElementById('apps-count').textContent = `共 ${list.length} 个条目 · 每页 ${PAGE_SIZE} 条`;
  document.getElementById('apps-page').textContent = `第 ${appState.page} / ${pages} 页`;
  document.getElementById('apps-prev').disabled = appState.page <= 1;
  document.getElementById('apps-next').disabled = appState.page >= pages;
}

document.getElementById('app-search').addEventListener('input', (event) => {
  appState.keyword = event.target.value;
  appState.page = 1;
  renderApps();
});

document.getElementById('app-filter').addEventListener('click', (event) => {
  const button = event.target.closest('.seg');
  if (!button) return;
  appState.kind = button.dataset.kind;
  appState.page = 1;
  document.querySelectorAll('#app-filter .seg').forEach((el) => {
    el.classList.toggle('active', el === button);
  });
  renderApps();
});

document.getElementById('apps-prev').addEventListener('click', () => {
  appState.page = Math.max(1, appState.page - 1);
  renderApps();
});

document.getElementById('apps-next').addEventListener('click', () => {
  appState.page += 1;
  renderApps();
});

/* ---------------- A3 运行日志 ---------------- */
const LOGS = [
  ['09:41:02', 'INFO', '服务启动完成，监听 0.0.0.0:3721'],
  ['09:41:02', 'INFO', '协议版本 v2，WebSocket 通道就绪'],
  ['09:41:07', 'INFO', '设备握手 device=pixel8 proto=v2 rtt=12ms'],
  ['09:41:07', 'INFO', '令牌校验通过，会话建立 sid=8f2a41'],
  ['09:42:15', 'WARN', '应用目录扫描超时（D:\\Games），已回退缓存结果'],
  ['09:43:01', 'INFO', '图标重扫完成：12 项成功，2 项使用 shell 默认图标'],
  ['09:44:38', 'INFO', '启动 赛博朋克 2077，显示模式切换为 2560×1440@165'],
  ['09:51:20', 'INFO', '游戏进程已退出，恢复原显示模式 1920×1080@60'],
  ['09:58:04', 'WARN', '锁屏状态下收到配对请求，已按安全策略拒绝'],
  ['10:02:11', 'INFO', '心跳正常 device=pixel8 rtt=12ms'],
  ['10:07:45', 'ERROR', '输入通道断开：connection reset by peer'],
  ['10:07:46', 'INFO', '输入通道重连成功，按键注入已恢复'],
  ['10:08:12', 'INFO', '设备 小米 14 心跳超时 3 次，恢复后重新握手'],
  ['10:12:30', 'INFO', '目录扫描：D:\\SteamLibrary 命中 8 个可执行文件'],
  ['10:15:02', 'WARN', '图标提取失败：fortnite.exe 无嵌入图标，已使用默认图标'],
  ['10:21:47', 'INFO', '手机端请求「锁定」，会话 sid=8f2a41'],
  ['10:30:11', 'INFO', '显示器切换到 1920×1080@60，恢复耗时 1.8s'],
  ['10:44:03', 'INFO', '日志轮转：qinglink-2026-11-01-old.log 已归档'],
];

const logState = { keyword: '', level: 'all', live: true };

function renderLogs() {
  const keyword = logState.keyword.trim().toLowerCase();
  const body = document.getElementById('log-body');
  body.innerHTML = LOGS.filter(([ts, level, message]) => {
    const levelOk = logState.level === 'all' || logState.level === level;
    const textOk = !keyword || message.toLowerCase().includes(keyword);
    return levelOk && textOk;
  })
    .map(
      ([ts, level, message]) => `
      <div class="log-line">
        <span class="ts">${ts}</span>
        <span class="lv ${level}">${level}</span>
        <span class="msg">${message}</span>
      </div>`,
    )
    .join('');

  if (logState.live) body.scrollTop = body.scrollHeight;
}

document.getElementById('log-search').addEventListener('input', (event) => {
  logState.keyword = event.target.value;
  renderLogs();
});

document.getElementById('log-filter').addEventListener('click', (event) => {
  const button = event.target.closest('.seg');
  if (!button) return;
  logState.level = button.dataset.level;
  document.querySelectorAll('#log-filter .seg').forEach((el) => {
    el.classList.toggle('active', el === button);
  });
  renderLogs();
});

document.getElementById('log-live').addEventListener('click', (event) => {
  logState.live = !logState.live;
  event.currentTarget.classList.toggle('paused', !logState.live);
  event.currentTarget.innerHTML = logState.live
    ? '<i class="dot"></i>实时 · 自动滚动'
    : '<i class="dot"></i>已暂停';
  renderLogs();
});

/* 演示用：每 4 秒追一条新日志，验证自动滚动与过滤。 */
let seq = 0;
setInterval(() => {
  if (!logState.live) return;
  seq += 1;
  const now = new Date();
  const ts = now.toTimeString().slice(0, 8);
  const samples = [
    ['INFO', '心跳正常 device=pixel8 rtt=12ms'],
    ['INFO', '命令队列空闲，等待下一个输入'],
    ['WARN', '手机端上一次按键未确认，已重发'],
  ];
  const [level, message] = samples[seq % samples.length];
  LOGS.push([ts, level, message]);
  if (LOGS.length > 60) LOGS.shift();
  renderLogs();
}, 4000);

/* ---------------- 初始化 ---------------- */
renderApps();
renderLogs();
