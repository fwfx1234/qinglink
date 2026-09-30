(() => {
  const $ = (selector, root = document) => root.querySelector(selector);
  const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

  const state = {
    screen: 'home',
    status: 'offline',
    activeLibrary: 'pinned',
    touchMode: 'pointer',
    inputMode: 'keys',
    customShortcuts: ['Ctrl + Enter', 'Esc'],
    activePreset: 'common',
    shortcutSets: [
      { id: 'common', name: '常用', description: 'Tab、方向键和复制粘贴', shortcuts: ['Tab', 'Esc', '↑', '↓', '←', '→', 'Ctrl + C', 'Ctrl + V'], builtin: true },
      { id: 'opencode', name: 'OpenCode', description: '会话、提交和中断', shortcuts: ['Ctrl + K', 'Ctrl + L', 'Ctrl + Enter', 'Esc'], builtin: true },
      { id: 'codex', name: 'Codex', description: '命令、提交和中断', shortcuts: ['Ctrl + K', 'Ctrl + Enter', 'Alt + Enter', 'Esc'], builtin: true },
    ],
    query: '',
    volume: 52,
    muted: false,
    pointerSpeed: 55,
    scrollDirection: 'standard',
    theme: 'dark',
    currentApp: null,
    lastApp: 'hades',
    autoUnlock: true,
    library: [
      { id: 'hades', name: 'Hades II', type: 'game', meta: 'Steam · 最近使用', icon: 'gamepad-2', tone: 'violet', pinned: true, running: false },
      { id: 'elden', name: 'Elden Ring', type: 'game', meta: 'Steam · 76 GB', icon: 'swords', tone: 'amber', pinned: false, running: false },
      { id: 'forza', name: 'Forza Horizon 5', type: 'game', meta: 'Xbox · 110 GB', icon: 'car-front', tone: 'cyan', pinned: false, running: false },
      { id: 'steam', name: 'Steam 大屏模式', type: 'game', meta: '快捷方式 · 最近使用', icon: 'gamepad-2', tone: 'blue', pinned: true, running: false },
      { id: 'spotify', name: 'Spotify', type: 'app', meta: '应用 · 最近使用', icon: 'music-2', tone: 'green', pinned: true, running: false },
      { id: 'chrome', name: 'Google Chrome', type: 'app', meta: '应用 · 12 个窗口', icon: 'globe-2', tone: 'cyan', pinned: false, running: false },
      { id: 'discord', name: 'Discord', type: 'app', meta: '应用 · 已安装', icon: 'messages-square', tone: 'violet', pinned: false, running: false },
    ],
    processes: [
      { id: 8240, name: 'Hades II', detail: 'hades2.exe · 前台', memory: '2.6 GB', icon: 'gamepad-2', tone: 'violet', foreground: true, critical: false },
      { id: 3112, name: 'Steam Client WebHelper', detail: 'steamwebhelper.exe', memory: '408 MB', icon: 'layers-2', tone: 'blue', foreground: false, critical: false },
      { id: 4596, name: 'Spotify', detail: 'Spotify.exe', memory: '188 MB', icon: 'music-2', tone: 'green', foreground: false, critical: false },
      { id: 912, name: 'Windows Explorer', detail: 'explorer.exe · 系统进程', memory: '124 MB', icon: 'folder-open', tone: 'amber', foreground: false, critical: true },
    ],
  };

  const statusMeta = {
    offline: { label: '已关机', dot: '', kicker: '客厅电脑', title: '唤醒电脑，开始游戏', description: '电脑目前处于睡眠或关机状态，唤醒后将自动建立连接。', latency: '上次在线 20:42', icon: 'monitor-up' },
    waking: { label: '正在唤醒', dot: 'warning', kicker: '连接准备中', title: '正在唤醒客厅电脑', description: '已发送局域网唤醒信号，正在等待电脑上线。', latency: '通常需要 5–15 秒', icon: 'radio-tower' },
    connecting: { label: '连接中', dot: 'warning', kicker: '正在建立安全连接', title: '电脑已上线', description: '正在连接 QingLink 服务并检查 Windows 状态。', latency: '局域网 · 低延迟', icon: 'plug-zap' },
    locked: { label: '需要解锁', dot: 'warning', kicker: 'Windows 锁屏', title: '解锁后即可使用', description: '电脑已上线，输入 PIN 或使用确认解锁继续。', latency: '已连接 · 12 ms', icon: 'lock-keyhole' },
    ready: { label: '可以使用', dot: 'online', kicker: '客厅电脑', title: '一切就绪，选择要做的事', description: '电脑已解锁。启动游戏、打开触控板，或继续上次使用。', latency: '已连接 · 12 ms', icon: 'monitor-check' },
    launching: { label: '启动中', dot: 'warning', kicker: '正在准备应用', title: '马上就好', description: '正在启动应用并等待窗口响应，请不要重复点击。', latency: '正在启动…', icon: 'loader-circle' },
    running: { label: '运行中', dot: 'online', kicker: '当前前台应用', title: '正在使用 {app}', description: '可以打开触控板、切换窗口或结束当前应用。', latency: '已连接 · 12 ms', icon: 'gamepad-2' },
    stopping: { label: '正在停止', dot: 'warning', kicker: '正在结束应用', title: '正在安全退出', description: '等待应用关闭并刷新当前进程状态。', latency: '处理中…', icon: 'square' },
    sleeping: { label: '正在睡眠', dot: 'warning', kicker: '电源操作', title: '电脑即将进入睡眠', description: '手机会保留配对信息，下次打开可直接唤醒。', latency: '正在断开连接…', icon: 'moon' },
    shutting_down: { label: '正在关机', dot: 'warning', kicker: '电源操作', title: '电脑即将关机', description: '所有正在运行的程序将被关闭，请稍候。', latency: '正在断开连接…', icon: 'power' },
    error: { label: '连接失败', dot: 'danger', kicker: '暂时无法连接', title: '没有连上电脑', description: '请确认手机和电脑连接在同一个 Wi‑Fi，然后重试。', latency: '连接超时', icon: 'wifi-off' },
  };

  function statusIsReady() {
    return ['ready', 'running'].includes(state.status);
  }

  function iconRefresh() {
    if (window.lucide) window.lucide.createIcons({ attrs: { 'stroke-width': 1.8 } });
  }

  function toast(message, kind = 'success') {
    const region = $('.toast-region');
    const item = document.createElement('div');
    item.className = `toast ${kind === 'error' ? 'error' : ''}`;
    item.innerHTML = `<i data-lucide="${kind === 'error' ? 'alert-circle' : 'check-circle-2'}"></i><span>${message}</span>`;
    region.appendChild(item);
    iconRefresh();
    window.setTimeout(() => item.remove(), 2800);
  }

  function setScreen(screen) {
    state.screen = screen;
    $$('.view').forEach((view) => view.classList.toggle('active', view.id === `${screen}-view`));
    $$('.nav-item').forEach((item) => item.classList.toggle('active', item.dataset.nav === (['processes', 'settings', 'shortcuts'].includes(screen) ? 'more' : screen)));
    window.scrollTo({ top: 0, behavior: 'smooth' });
    iconRefresh();
  }

  function setStatus(next, appId = state.currentApp) {
    state.status = next;
    if (appId) state.currentApp = appId;
    render();
  }

  function render() {
    renderStatus();
    renderPinned();
    renderLibrary();
    renderProcesses();
    renderShortcutSets();
    renderNav();
    renderVolume();
    updateAvailability();
    iconRefresh();
  }

  function renderStatus() {
    const meta = statusMeta[state.status];
    const current = state.library.find((item) => item.id === state.currentApp);
    const statusCard = $('[data-status-card]');
    statusCard.classList.toggle('compact', ['ready', 'running'].includes(state.status));
    statusCard.hidden = statusIsReady();
    const title = meta.title.replace('{app}', current?.name || '应用');
    const labels = $$('[data-status-label]');
    labels.forEach((node) => { node.textContent = meta.label; });
    $$('[data-status-dot]').forEach((node) => node.className = `status-dot ${meta.dot}`);
    const badgeDot = $('.status-badge .status-dot');
    if (badgeDot) badgeDot.className = `status-dot ${meta.dot}`;
    $('[data-status-kicker]').textContent = meta.kicker;
    $('[data-status-title]').textContent = title;
    $('[data-status-description]').textContent = meta.description;
    $('[data-latency]').textContent = meta.latency;
    $('[data-status-card] [data-lucide]')?.setAttribute('data-lucide', meta.icon);
    const progress = $('[data-status-progress]');
    const progressLabel = $('[data-progress-label]');
    const isBusy = ['waking', 'connecting', 'launching', 'stopping', 'sleeping', 'shutting_down'].includes(state.status);
    progress.hidden = !isBusy;
    if (progressLabel) progressLabel.textContent = ({ waking: '正在发送唤醒信号…', connecting: '正在建立连接…', launching: '正在等待应用窗口…', stopping: '正在结束应用…', sleeping: '正在进入睡眠…', shutting_down: '正在关闭电脑…' })[state.status] || '';
    const primary = $('[data-action="primary-status"]');
    const primaryLabel = primary.querySelector('span');
    const primaryIcon = primary.querySelector('[data-lucide]');
    const primaryMap = {
      offline: ['唤醒电脑', 'power'], waking: ['正在唤醒…', 'loader-circle'], connecting: ['连接中…', 'loader-circle'], locked: ['解锁 Windows', 'lock-open'], ready: ['打开触控板', 'mouse-pointer-2'], launching: ['启动中…', 'loader-circle'], running: ['继续控制', 'mouse-pointer-2'], stopping: ['正在停止…', 'loader-circle'], sleeping: ['正在睡眠…', 'moon'], shutting_down: ['正在关机…', 'power'], error: ['重新连接', 'rotate-ccw'],
    }[state.status];
    primaryLabel.textContent = primaryMap[0];
    primaryIcon.setAttribute('data-lucide', primaryMap[1]);
    primary.disabled = isBusy;
    $('[data-action="retry"]').hidden = state.status !== 'error';
  }

  function renderNav() {
    $$('.nav-item').forEach((item) => item.classList.toggle('active', item.dataset.nav === (['processes', 'settings', 'shortcuts'].includes(state.screen) ? 'more' : state.screen)));
  }

  function renderLibrary() {
    const list = $('#library-list');
    const query = state.query.toLowerCase();
    const recentIds = new Set(['hades', 'spotify', 'steam']);
    const items = state.library.filter((item) => {
      const matchesFilter = state.activeLibrary === 'all' || (state.activeLibrary === 'pinned' ? item.pinned : recentIds.has(item.id));
      return matchesFilter && (!query || `${item.name} ${item.meta}`.toLowerCase().includes(query));
    }).sort((a, b) => Number(b.pinned) - Number(a.pinned));
    list.innerHTML = items.map((item) => `
      <article class="library-item ${item.running ? 'running' : ''}">
        <div class="app-icon ${item.tone}"><i data-lucide="${item.icon}"></i></div>
        <div class="library-item-copy"><strong>${item.name}</strong><small>${item.running ? '运行中 · 点击停止' : item.meta}</small></div>
        <div class="item-actions"><button class="pin-item ${item.pinned ? 'pinned' : ''}" data-action="pin" data-id="${item.id}" aria-label="${item.pinned ? '取消置顶' : '置顶'} ${item.name}"><i data-lucide="pin"></i></button><button class="launch-item" data-action="launch" data-id="${item.id}" aria-label="${item.running ? '停止' : '启动'} ${item.name}"><i data-lucide="${item.running ? 'square' : 'play'}"></i></button></div>
      </article>`).join('');
    $('[data-empty-state]').hidden = items.length !== 0;
    $$('.filter-chip').forEach((chip) => chip.classList.toggle('active', chip.dataset.libraryFilter === state.activeLibrary));
  }

  function renderPinned() {
    const list = $('[data-pinned-list]');
    const recent = state.library.find((item) => item.id === state.lastApp);
    const pinned = state.library.filter((item) => item.pinned).sort((a, b) => Number(b.id === recent?.id) - Number(a.id === recent?.id));
    list.innerHTML = pinned.slice(0, 4).map((item, index) => `<button class="pinned-app ${item.running ? 'running' : ''}" data-action="launch" data-id="${item.id}" aria-label="${item.running ? '停止' : '启动'} ${item.name}"><span class="app-icon ${item.tone}"><i data-lucide="${item.icon}"></i></span><span class="pinned-app-copy"><strong>${item.name}</strong><small>${item.running ? '运行中' : index === 0 ? '上次使用' : item.type === 'game' ? '游戏' : '软件'}</small></span><i data-lucide="${item.running ? 'square' : 'play'}"></i></button>`).join('');
    $('[data-pinned-empty]').hidden = pinned.length !== 0;
  }

  function renderProcesses() {
    const query = ($('#process-search')?.value || '').toLowerCase();
    const list = $('#process-list');
    const items = state.processes.filter((process) => `${process.name} ${process.id} ${process.detail}`.toLowerCase().includes(query));
    list.innerHTML = items.map((process) => `
      <article class="process-item ${process.foreground ? 'foreground' : ''}">
        <span class="process-avatar ${process.tone}"><i data-lucide="${process.icon}"></i></span>
        <span class="process-item-copy"><strong>${process.name}</strong><small>${process.detail} · PID ${process.id} · ${process.memory}</small></span>
        ${process.critical ? '<span class="status-tag">系统</span>' : `<button class="kill-process" data-action="kill-process" data-id="${process.id}" aria-label="结束 ${process.name}"><i data-lucide="square"></i></button>`}
      </article>`).join('');
  }

  function renderShortcutSets() {
    const list = $('[data-shortcut-set-list]');
    if (!list) return;
    list.innerHTML = state.shortcutSets.map((set) => `<article class="shortcut-set-card ${set.id === state.activePreset ? 'active' : ''}"><button class="shortcut-set-select" data-action="select-shortcut-set" data-id="${set.id}"><span class="shortcut-set-icon"><i data-lucide="${set.id === 'codex' ? 'terminal' : 'command'}"></i></span><span class="shortcut-set-copy"><strong>${set.name}</strong><small>${set.description}</small></span>${set.id === state.activePreset ? '<span class="status-tag">使用中</span>' : ''}</button><button class="shortcut-set-edit" data-action="edit-shortcut-set" data-id="${set.id}" aria-label="编辑 ${set.name}"><i data-lucide="pencil"></i></button></article>`).join('');
  }

  function renderVolume() {
    $$('[data-volume-label]').forEach((node) => node.textContent = state.muted ? '静音' : `${state.volume}%`);
    const output = $('[data-volume-output]');
    if (output) output.textContent = state.muted ? '0' : state.volume;
    const slider = $('[data-volume-slider]');
    if (slider) slider.value = state.volume;
    $$('[data-action="mute"] svg').forEach((node) => node.setAttribute('data-lucide', state.muted ? 'volume-x' : 'volume-2'));
  }

  function updateAvailability() {
    const ready = statusIsReady();
    $$('[data-requires-ready]').forEach((element) => {
      element.disabled = !ready;
      element.title = ready ? '' : '电脑解锁后可用';
    });
  }

  function openModal(content) {
    const backdrop = $('#modal-backdrop');
    $('#modal-sheet').innerHTML = `<div class="modal-grabber"></div>${content}`;
    backdrop.hidden = false;
    iconRefresh();
  }

  function closeModal() { $('#modal-backdrop').hidden = true; $('#modal-sheet').innerHTML = ''; }

  function confirmPower(mode) {
    const copy = { sleep: ['让电脑进入睡眠？', '电脑会立即进入睡眠，当前应用保持打开。', '睡眠', 'moon'], restart: ['重启客厅电脑？', '正在运行的应用会被关闭，电脑上线后仍可继续连接。', '重启', 'rotate-cw'], shutdown: ['关闭客厅电脑？', '这会中断所有正在运行的游戏和软件，请确认已保存进度。', '关机', 'power'] }[mode];
    openModal(`<h3>${copy[0]}</h3><p>${copy[1]}</p><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button><button class="primary-button ${mode === 'shutdown' ? 'danger-button' : ''}" data-modal="power-confirm" data-mode="${mode}"><i data-lucide="${copy[3]}"></i>${copy[2]}</button></div>`);
  }

  function openVolumeModal() {
    openModal(`<h3>电脑音量</h3><p>滑动调节客厅电脑的系统音量。</p><div class="volume-control"><button class="round-action" data-modal="mute"><i data-lucide="${state.muted ? 'volume-x' : 'volume-2'}"></i></button><input type="range" min="0" max="100" value="${state.volume}" data-modal-volume aria-label="电脑音量"><output data-modal-output>${state.muted ? 0 : state.volume}</output></div><div class="modal-actions" style="margin-top:18px"><button class="secondary-button" data-modal="close">完成</button></div>`);
  }

  function openTouchpadSettings() {
    openModal(`<h3>触控板设置</h3><div class="touchpad-setting"><div class="settings-heading"><h3>映射速率</h3><output data-pointer-speed-output>${state.pointerSpeed}</output></div><input class="settings-range" type="range" min="10" max="100" value="${state.pointerSpeed}" data-pointer-speed aria-label="映射速率"><small>调节手机滑动对应的鼠标移动距离</small></div><div class="touchpad-setting"><div class="settings-heading"><h3>滚动方向</h3></div><div class="direction-options"><label class="direction-option"><input type="radio" name="scroll-direction" value="standard" ${state.scrollDirection === 'standard' ? 'checked' : ''}><span><strong>标准</strong><small>向上滑动，页面向上</small></span></label><label class="direction-option"><input type="radio" name="scroll-direction" value="natural" ${state.scrollDirection === 'natural' ? 'checked' : ''}><span><strong>自然</strong><small>向上滑动，内容向下</small></span></label></div></div><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button><button class="primary-button" data-modal="save-touchpad-settings"><i data-lucide="check"></i>保存</button></div>`);
  }

  function openWindowModal() {
    openModal(`<h3>已打开窗口</h3><p>点击窗口切换到前台，也可以直接关闭。</p><div class="choice-list"><div class="choice-row"><button class="choice-button" data-modal="window" data-window="hades"><i data-lucide="gamepad-2"></i><span>Hades II</span><small>前台 · PID 8240</small></button><button class="choice-close" data-modal="close-window" data-window-name="Hades II" aria-label="关闭 Hades II" title="关闭"><i data-lucide="x"></i></button></div><div class="choice-row"><button class="choice-button" data-modal="window" data-window="spotify"><i data-lucide="music-2"></i><span>Spotify</span><small>后台 · PID 4596</small></button><button class="choice-close" data-modal="close-window" data-window-name="Spotify" aria-label="关闭 Spotify" title="关闭"><i data-lucide="x"></i></button></div><div class="choice-row"><button class="choice-button" data-modal="window" data-window="chrome"><i data-lucide="globe-2"></i><span>Google Chrome</span><small>后台 · 12 个窗口</small></button><button class="choice-close" data-modal="close-window" data-window-name="Google Chrome" aria-label="关闭 Google Chrome" title="关闭"><i data-lucide="x"></i></button></div></div><div class="modal-actions"><button class="secondary-button" data-modal="close">完成</button></div>`);
  }

  function openAddModal() {
    openModal(`<h3>添加到库</h3><p>把电脑上的游戏或软件加入 QingLink，之后可以一键启动。</p><form class="modal-form" id="add-form"><label class="form-label">名称<input name="name" required placeholder="例如：Moonlighter" /></label><label class="form-label">类型<select name="type"><option value="game">游戏</option><option value="app">软件</option></select></label><label class="form-label">启动路径或协议<input name="path" required placeholder="C:\\Games\\Moonlighter\\game.exe" /></label><div class="modal-actions"><button class="secondary-button" type="button" data-modal="close">取消</button><button class="primary-button" type="submit"><i data-lucide="plus"></i>添加并测试</button></div></form>`);
  }

  function openTextModal() {
    const options = state.shortcutSets.map((set) => `<option value="${set.id}" ${set.id === state.activePreset ? 'selected' : ''}>${set.name} 快捷键</option>`).join('');
    openModal(`<div class="ime-sheet"><div class="shortcut-presets"><select class="preset-select" data-shortcut-preset aria-label="快捷键方案">${options}</select><button class="edit-shortcuts" data-modal="edit-shortcuts">管理</button></div><div class="shortcut-dock" data-shortcut-dock></div><textarea class="ime-input-capture" data-modal-text aria-label="输入法"></textarea></div>`);
    renderShortcutDock(state.activePreset);
    window.setTimeout(() => $('[data-modal-text]')?.focus(), 50);
  }

  function renderShortcutDock(preset) {
    const shortcuts = state.shortcutSets.find((set) => set.id === preset)?.shortcuts || state.customShortcuts;
    const dock = $('[data-shortcut-dock]');
    if (dock) dock.innerHTML = shortcuts.map((shortcut) => `<button class="shortcut-chip" data-modal="send-shortcut" data-shortcut="${shortcut}">${shortcut}</button>`).join('');
    iconRefresh();
  }

  function openShortcutEditor() {
    openModal(`<h3>自定义快捷键</h3><p>每行一个快捷键，例如：Ctrl + Enter</p><div class="text-input-panel"><textarea data-shortcut-editor rows="5">${state.customShortcuts.join('\n')}</textarea><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button><button class="primary-button" data-modal="save-shortcuts"><i data-lucide="check"></i>保存</button></div></div>`);
  }

  function openShortcutSetEditor(id) {
    const set = state.shortcutSets.find((item) => item.id === id);
    if (!set) return;
    openModal(`<h3>编辑 ${set.name}</h3><p>每行一个快捷键，保存后会显示在输入面板上方。</p><div class="text-input-panel"><textarea data-shortcut-set-editor rows="5">${set.shortcuts.join('\n')}</textarea><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button><button class="primary-button" data-modal="save-shortcut-set" data-id="${id}"><i data-lucide="check"></i>保存</button></div></div>`);
  }

  function openAddShortcutSet() {
    openModal(`<h3>新增快捷键方案</h3><p>建立一套适合自己工作流的快捷键。</p><div class="modal-form"><label class="form-label">方案名称<input data-new-set-name placeholder="例如：我的编程快捷键" /></label><label class="form-label">快捷键列表<textarea data-new-set-shortcuts rows="4" placeholder="Ctrl + Enter\nEsc"></textarea></label><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button><button class="primary-button" data-modal="save-new-set"><i data-lucide="plus"></i>添加方案</button></div></div>`);
  }

  function launchItem(id) {
    const item = state.library.find((entry) => entry.id === id);
    if (!item) return;
    if (!statusIsReady()) { toast(state.status === 'locked' ? '请先解锁 Windows' : '电脑上线并解锁后才能启动', 'error'); return; }
    if (item.running) {
      stopItem(item);
      return;
    }
    state.lastApp = item.id;
    item.running = true;
    setStatus('launching', item.id);
    window.setTimeout(() => { if (state.status === 'launching') { setStatus('running', item.id); toast(`${item.name} 已启动`); } }, 1100);
  }

  function stopItem(item) {
    openModal(`<h3>停止 ${item.name}？</h3><p>会结束当前应用进程，未保存的数据可能丢失。</p><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button><button class="primary-button danger-button" data-modal="stop-confirm" data-id="${item.id}"><i data-lucide="square"></i>停止应用</button></div>`);
  }

  function executePower(mode) {
    closeModal();
    state.library.forEach((item) => { item.running = false; });
    const next = mode === 'shutdown' ? 'shutting_down' : 'sleeping';
    setStatus(next);
    window.setTimeout(() => { setStatus('offline'); toast(mode === 'shutdown' ? '电脑已关机' : '电脑已进入睡眠'); }, 1500);
  }

  function wake() {
    if (!['offline', 'error'].includes(state.status)) return;
    setStatus('waking');
    window.setTimeout(() => setStatus('connecting'), 900);
    window.setTimeout(() => { setStatus('locked'); toast('电脑已上线，请解锁'); }, 1900);
  }

  function unlock() {
    if (state.status !== 'locked') return;
    setStatus('connecting');
    window.setTimeout(() => { setStatus('ready'); toast('Windows 已解锁，可以使用'); }, 850);
  }

  function handlePrimary() {
    if (state.status === 'offline' || state.status === 'error') return wake();
    if (state.status === 'locked') return unlock();
    if (state.status === 'ready') return setScreen('control');
    if (state.status === 'running') return setScreen('control');
  }

  function handleAction(action, element) {
    const id = element.dataset.id;
    if (action === 'theme') { state.theme = state.theme === 'dark' ? 'light' : 'dark'; document.body.classList.toggle('light', state.theme === 'light'); toast(state.theme === 'light' ? '已切换浅色模式' : '已切换深色模式'); return; }
    if (action === 'refresh' || action === 'retry') { toast('正在刷新电脑状态…'); if (state.status === 'error') setStatus('offline'); return; }
    if (action === 'primary-status') return handlePrimary();
    if (action === 'wake') return wake();
    if (action === 'control') { if (!statusIsReady()) return toast('请先唤醒并解锁电脑', 'error'); return setScreen('control'); }
    if (action === 'settings') return setScreen('settings');
    if (action === 'processes') return setScreen('processes');
    if (action === 'shortcuts') return setScreen('shortcuts');
    if (action === 'add-shortcut-set') return openAddShortcutSet();
    if (action === 'select-shortcut-set') { state.activePreset = id; renderShortcutSets(); toast(`${state.shortcutSets.find((set) => set.id === id)?.name || '方案'} 已启用`); return; }
    if (action === 'edit-shortcut-set') return openShortcutSetEditor(id);
    if (action === 'windows') return openWindowModal();
    if (action === 'volume') { if (!statusIsReady()) return toast('请先唤醒并解锁电脑', 'error'); return openVolumeModal(); }
    if (action === 'touchpad-settings') return openTouchpadSettings();
    if (action === 'continue') { const item = state.library.find((entry) => entry.id === state.lastApp); return item && launchItem(item.id); }
    if (action === 'open-text') return openTextModal();
    if (action === 'send-text') { const input = $('[data-text-panel] textarea'); if (!input.value.trim()) return toast('先输入要发送的文字', 'error'); toast(`已发送 ${input.value.trim().length} 个字符`); input.value = ''; return; }
    if (action === 'launch') return launchItem(id);
    if (action === 'pin') { const item = state.library.find((entry) => entry.id === id); if (item) { item.pinned = !item.pinned; renderPinned(); renderLibrary(); toast(item.pinned ? '已置顶到首页' : '已取消置顶'); } return; }
    if (action === 'add-item') return openAddModal();
    if (action === 'scan') { const banner = $('[data-scan-banner]'); banner.hidden = false; window.setTimeout(() => { banner.hidden = true; toast('扫描完成，未发现新的程序'); }, 1500); return; }
    if (action === 'clear-search') { state.query = ''; $('#library-search').value = ''; renderLibrary(); element.hidden = true; return; }
    if (action === 'power') return confirmPower(element.dataset.power);
    if (action === 'mute') { state.muted = !state.muted; renderVolume(); toast(state.muted ? '电脑已静音' : `音量已恢复到 ${state.volume}%`); return; }
    if (action === 'audio-output') return openModal(`<h3>音频输出</h3><p>选择电脑声音播放到哪个设备。</p><div class="choice-list"><button class="choice-button" data-modal="audio"><i data-lucide="tv"></i><span>客厅电视扬声器</span><small>默认</small></button><button class="choice-button" data-modal="audio"><i data-lucide="headphones"></i><span>蓝牙耳机</span><small>已断开</small></button></div><div class="modal-actions"><button class="secondary-button" data-modal="close">完成</button></div>`);
    if (action === 'resolution') return openModal(`<h3>分辨率与刷新率</h3><p>切换时屏幕可能短暂黑屏，手机连接不会中断。</p><div class="choice-list"><button class="choice-button" data-modal="resolution"><i data-lucide="check"></i><span>3840 × 2160 · 60 Hz</span><small>当前</small></button><button class="choice-button" data-modal="resolution"><i data-lucide="monitor"></i><span>2560 × 1440 · 120 Hz</span><small>推荐游戏</small></button><button class="choice-button" data-modal="resolution"><i data-lucide="monitor"></i><span>1920 × 1080 · 60 Hz</span><small>兼容模式</small></button></div><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button></div>`);
    if (action === 'pairing') return openModal(`<h3>配对与连接</h3><p>客厅电脑已安全配对。打开 QingLink 时会尝试连接，唤醒电脑请使用首页或设置里的 Wake-on-LAN。</p><div class="choice-list"><button class="choice-button" data-modal="close"><i data-lucide="qr-code"></i><span>重新配对这台电脑</span><small>需要电脑确认</small></button><button class="choice-button" data-modal="close"><i data-lucide="trash-2"></i><span>移除配对信息</span><small>危险操作</small></button></div><div class="modal-actions"><button class="secondary-button" data-modal="close">完成</button></div>`);
    if (action === 'diagnostics') return openModal(`<h3>连接诊断</h3><p>当前连接状态：${statusMeta[state.status].label}。这里可以模拟常见状态，用于演示错误反馈。</p><div class="choice-list"><button class="choice-button" data-modal="simulate-error"><i data-lucide="wifi-off"></i><span>模拟连接失败</span><small>进入重试状态</small></button><button class="choice-button" data-modal="simulate-ready"><i data-lucide="check-circle-2"></i><span>恢复为可使用</span><small>跳过等待流程</small></button></div><div class="modal-actions"><button class="secondary-button" data-modal="close">完成</button></div>`);
    if (action === 'refresh-processes') { renderProcesses(); toast('进程列表已刷新'); return; }
    if (action === 'kill-process') { const process = state.processes.find((entry) => String(entry.id) === String(id)); if (!process) return; if (process.critical) return toast('系统进程无法结束', 'error'); return openModal(`<h3>结束 ${process.name}？</h3><p>PID ${process.id} 将被强制结束，未保存的数据可能丢失。</p><div class="modal-actions"><button class="secondary-button" data-modal="close">取消</button><button class="primary-button danger-button" data-modal="kill-confirm" data-id="${process.id}"><i data-lucide="square"></i>结束进程</button></div>`); }
    if (action === 'close-window') return toast('已发送关闭当前窗口指令');
    if (action === 'window-minimize') return toast('当前窗口已最小化');
    if (action === 'window-maximize') return toast('当前窗口已最大化');
    if (action === 'touchpad-help') return toast('触控板移动鼠标，右侧滚动条控制页面');
    if (action === 'device') return toast('当前仅配对了客厅电脑');
  }

  document.addEventListener('click', (event) => {
    const nav = event.target.closest('[data-nav]');
    if (nav) { setScreen(nav.dataset.nav); return; }
    const filter = event.target.closest('[data-library-filter]');
    if (filter) { state.activeLibrary = filter.dataset.libraryFilter; renderLibrary(); return; }
    const touchMode = event.target.closest('[data-touch-mode]');
    if (touchMode) { state.touchMode = touchMode.dataset.touchMode; $$('.mode-chip').forEach((item) => item.classList.toggle('active', item === touchMode)); $('[data-touchpad]').classList.toggle('scroll-mode', state.touchMode === 'scroll'); $('.touchpad-hint').textContent = state.touchMode === 'scroll' ? '上下滑动滚动页面' : '在这里滑动'; return; }
    const inputMode = event.target.closest('[data-input-mode]');
    if (inputMode) { state.inputMode = inputMode.dataset.inputMode; $$('.input-mode-chip').forEach((item) => item.classList.toggle('active', item === inputMode)); $('[data-keys-panel]').hidden = state.inputMode !== 'keys'; $('[data-text-panel]').hidden = state.inputMode !== 'text'; return; }
    const key = event.target.closest('[data-key]');
    if (key) { toast(`已发送 ${key.textContent.trim()} 快捷键`); return; }
    const action = event.target.closest('[data-action]');
    if (action) { handleAction(action.dataset.action, action); return; }
    const modalAction = event.target.closest('[data-modal]');
    if (modalAction) {
      const kind = modalAction.dataset.modal;
      if (kind === 'close') return closeModal();
      if (kind === 'power-confirm') return executePower(modalAction.dataset.mode);
      if (kind === 'stop-confirm') { const item = state.library.find((entry) => entry.id === modalAction.dataset.id); if (item) { item.running = false; state.currentApp = null; closeModal(); setStatus('ready'); toast(`${item.name} 已停止`); } return; }
      if (kind === 'kill-confirm') { const index = state.processes.findIndex((entry) => String(entry.id) === String(modalAction.dataset.id)); if (index > -1) { const [removed] = state.processes.splice(index, 1); closeModal(); renderProcesses(); toast(`${removed.name} 已结束`); } return; }
      if (kind === 'window') { state.currentApp = modalAction.dataset.window; closeModal(); toast('已切换到前台窗口'); return; }
      if (kind === 'close-window') { closeModal(); toast(`${modalAction.dataset.windowName} 已关闭`); return; }
      if (kind === 'audio' || kind === 'resolution') { closeModal(); toast(kind === 'audio' ? '音频输出已切换' : '显示模式已切换'); return; }
      if (kind === 'simulate-error') { closeModal(); setStatus('error'); toast('已模拟连接失败', 'error'); return; }
      if (kind === 'simulate-ready') { closeModal(); setStatus('ready'); toast('连接已恢复'); return; }
      if (kind === 'mute') { state.muted = !state.muted; renderVolume(); const output = $('[data-modal-output]'); if (output) output.textContent = state.muted ? 0 : state.volume; return; }
      if (kind === 'send-modal-text') { const input = $('[data-modal-text]'); if (!input?.value.trim()) return toast('先输入文字', 'error'); closeModal(); toast(`已发送 ${input.value.trim().length} 个字符`); return; }
      if (kind === 'send-shortcut') { toast(`已发送 ${modalAction.dataset.shortcut}`); return; }
      if (kind === 'edit-shortcuts') { closeModal(); setScreen('shortcuts'); return; }
      if (kind === 'save-shortcuts') { const editor = $('[data-shortcut-editor]'); state.customShortcuts = editor.value.split(/\n|,/).map((item) => item.trim()).filter(Boolean).slice(0, 6); closeModal(); toast('自定义快捷键已保存'); return; }
      if (kind === 'save-shortcut-set') { const editor = $('[data-shortcut-set-editor]'); const set = state.shortcutSets.find((item) => item.id === modalAction.dataset.id); if (set && editor) { set.shortcuts = editor.value.split(/\n|,/).map((item) => item.trim()).filter(Boolean).slice(0, 8); } closeModal(); renderShortcutSets(); toast('快捷键方案已保存'); return; }
      if (kind === 'save-new-set') { const name = $('[data-new-set-name]')?.value.trim(); const shortcuts = $('[data-new-set-shortcuts]')?.value.split(/\n|,/).map((item) => item.trim()).filter(Boolean).slice(0, 8); if (!name || !shortcuts?.length) return toast('请填写方案名称和快捷键', 'error'); const id = `custom-${Date.now()}`; state.shortcutSets.push({ id, name, description: '自定义工作流', shortcuts, builtin: false }); state.activePreset = id; closeModal(); setScreen('shortcuts'); render(); toast(`${name} 已添加`); return; }
      if (kind === 'save-touchpad-settings') { const speed = $('[data-pointer-speed]'); const direction = $('[name="scroll-direction"]:checked'); state.pointerSpeed = Number(speed?.value || state.pointerSpeed); state.scrollDirection = direction?.value || state.scrollDirection; closeModal(); toast(`触控板设置已保存 · ${state.scrollDirection === 'natural' ? '自然滚动' : '标准滚动'}`); return; }
    }
  });

  $('#modal-backdrop').addEventListener('click', (event) => { if (event.target.id === 'modal-backdrop') closeModal(); });
  $('#library-search').addEventListener('input', (event) => { state.query = event.target.value.trim(); $('[data-action="clear-search"]').hidden = !state.query; renderLibrary(); });
  $('#process-search').addEventListener('input', renderProcesses);
  $('[data-volume-slider]').addEventListener('input', (event) => { state.volume = Number(event.target.value); state.muted = false; renderVolume(); });
  $('[data-scroll-rail]').addEventListener('input', (event) => { toast(`滚动到 ${event.target.value}%`); });
  document.addEventListener('input', (event) => { if (event.target.matches('[data-modal-volume]')) { state.volume = Number(event.target.value); state.muted = false; const output = $('[data-modal-output]'); if (output) output.textContent = state.volume; renderVolume(); } });
  document.addEventListener('change', (event) => { if (event.target.matches('[data-setting]')) { state[event.target.dataset.setting] = event.target.checked; toast(event.target.checked ? '偏好已开启' : '偏好已关闭'); } });
  $('#modal-sheet').addEventListener('submit', (event) => { if (event.target.id !== 'add-form') return; event.preventDefault(); const form = new FormData(event.target); const name = String(form.get('name')).trim(); const type = String(form.get('type')); const path = String(form.get('path')).trim(); if (!name || !path) return; const id = `manual-${Date.now()}`; state.library.push({ id, name, type, meta: `手动添加 · ${path.split('\\').pop()}`, icon: type === 'game' ? 'gamepad-2' : 'app-window', tone: type === 'game' ? 'violet' : 'slate', pinned: false, running: false }); state.activeLibrary = 'all'; closeModal(); setScreen('library'); render(); toast(`${name} 已添加到库`); });
  document.addEventListener('change', (event) => { if (event.target.matches('[data-shortcut-preset]')) { state.activePreset = event.target.value; renderShortcutDock(state.activePreset); renderShortcutSets(); } });
  document.addEventListener('input', (event) => { if (event.target.matches('[data-pointer-speed]')) { const output = $('[data-pointer-speed-output]'); if (output) output.textContent = event.target.value; } });
  document.addEventListener('input', (event) => { if (!event.target.matches('[data-modal-text]')) return; event.target.dataset.lastValue = event.target.value; });
  document.addEventListener('keydown', (event) => { if (!event.target.matches('[data-modal-text]')) return; if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); toast('已发送回车'); return; } if (event.key === 'Backspace') { event.preventDefault(); const input = event.target; input.value = input.value.slice(0, -1); input.dispatchEvent(new Event('input', { bubbles: true })); toast('已发送删除'); } });

  const touchpad = $('[data-touchpad]');
  touchpad.addEventListener('pointerdown', () => touchpad.classList.add('active'));
  touchpad.addEventListener('pointerup', () => { touchpad.classList.remove('active'); toast('鼠标移动已发送'); });
  touchpad.addEventListener('pointerleave', () => touchpad.classList.remove('active'));
  touchpad.addEventListener('pointermove', (event) => { if (!touchpad.classList.contains('active')) return; const rect = touchpad.getBoundingClientRect(); const cursor = $('.touchpad-cursor', touchpad); cursor.style.left = `${Math.max(8, Math.min(88, ((event.clientX - rect.left) / rect.width) * 100))}%`; cursor.style.top = `${Math.max(8, Math.min(82, ((event.clientY - rect.top) / rect.height) * 100))}%`; });

  render();
})();
