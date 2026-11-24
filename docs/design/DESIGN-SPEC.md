# QingLink 界面设计交付说明

> 面向开发（含 AI 开发 agent）。读完这份文档 + 对照 `screens/` 里的图，就能开始实现，不需要再问设计稿细节。

---

## 0. 先看这里：这份交付包怎么用

```
docs/design/
├── DESIGN-SPEC.md          ← 本文件。token、尺寸、屏清单、代码映射
├── sheets-dark-light.pdf   ← 弹层 24 页（第 1~12 页暗色、13~24 页亮色，每页 2 组）
└── screens/
    ├── S01-home-dark.png   ← 单屏 2x 导出，文件名规则：S{两位序号}-{页面}-{dark|light}.png
    ├── S01-home-light.png
    └── ... 共 28 张（14 屏 × 2 主题）
```

**推荐工作方式**：改哪一屏，就先打开对应的那张 PNG（2x，尺寸 780×1688），再对照本文第 2 节的 token 和第 5 节的映射表，去改对应的 Dart 文件。不要凭记忆写色值。

**设计源文件（需要看层级/量间距时）**：`https://ardot.tencent.com/file/742668064059515`
画布分区：`(0,0)` 双主题规范 · `(1836,0)` 组件库 · `(0,3720)` 暗色 S1~S14 · `(0,5700)` 次级页暗色 · `(0,6900)` 亮色 · `(0,8950)` 次级页亮色 · `(3400,0)` 弹层暗色 · `(3400,4700)` 弹层亮色。
桌面管理界面的视觉基线就是代码本身（`apps/desktop/web-ui`），本文只保留 token 与尺寸约定。

---

## 1. 基准与硬约束

| 项 | 值 |
|---|---|
| 手机基准尺寸 | **390 × 844**（iPhone 14/15 逻辑像素），圆角 34 |
| 状态栏 | 高 62 |
| 顶栏 Nav | 高 56，左右内边距 18 |
| 页面 Body | 左右内边距 18 |
| 底部 TabBar | 高 95（含手势条），页面底部预留 96 |
| 管理端基准 | 1440 × 900，侧栏 232 |
| 最小点按区 | **44 × 44** |
| 主按钮高 | 52 |
| 列表行高 | 60（密集列表可 48，如按键表） |

**R1 — 高频操作贴底。** 主操作必须落在拇指自然弧内（纵向 y ≥ 0.6×H）。顶栏只承载"我在哪"和只读状态，不放主操作。
**R2 — 单手。** 所有高频入口在屏幕下半部；页面顶部不做需要精确点按的小目标。
**R3 — 图标不得用字母/汉字占位。** 程序图标用真实品牌 SVG，文件图标用真实文件类型图形。见第 6 节。

---

## 2. 颜色 token

设计稿取值（**这是唯一权威来源**，不要用旧代码里的值）：

| Token | 暗色 | 亮色 | 说明 |
|---|---|---|---|
| `bg.base` | `#0A0D0D` | `#F5F7F7` | 页面背景 |
| `bg.surface` | `#121414` | `#FFFFFF` | 卡片 |
| `bg.raised` | `#1A1C1C` | `#FAFCFC` | 卡片内的次级块、按钮底 |
| `bg.icon-btn` | `#141717` | `#EFF3F1` | 图标按钮底座 |
| `line` | `#212626` | `#E7ECEA` | 常规分隔线、卡片描边 |
| `line.strong` | `#303836` | `#D9DEDB` | 强调描边、输入框边框 |
| `text.high` | `#EDF2F0` | `#0D1212` | 标题、正文 |
| `text.mid` | `#8F9996` | `#5C6963` | 次要说明 |
| `text.low` | `#5E6B66` | `#8C9694` | 路径、时间戳、占位 |
| `brand.500` | `#36D9A1` | `#12A87A` | 品牌主色（亮色压深保证对比度） |
| `brand.tint` | `#0F2921` | `#E5F5F0` | 品牌色浅底（选中态、强调块） |
| `on-brand` | `#08120E` | `#FFFFFF` | 品牌色底上的文字 |
| `accent.info` | `#59C7E8` | `#0D94B8` | 信息、速度、音量 |
| `accent.violet` | `#A88CF5` | `#7D5CE0` | 快捷键、脚本类 |
| `state.warn` | `#F2B557` | `#B87503` | 进行中、警示 |
| `state.danger` | `#FF6B6E` | `#D93B3D` | 危险操作、离线 |
| `danger.tint` | `#2E1113` | `#FDECEC` | 危险按钮底座 |
| `scrim` | `rgba(0,0,0,.62)` | `rgba(13,23,18,.32)` | 弹层遮罩 |

### 代码落点

上表已全部落在 `apps/mobile/lib/src/theme/app_palette.dart` 的 `AppPalette.dark` / `.light`，
`AppColors.primary` 等门面值由 `app_theme.dart` 按实际亮度注入。改色值时两处同步：
本表与 `app_palette.dart`——这是唯一的替换口径，不存在"旧值待迁移"。

> 触控板渐变不在 token 体系内，同样在 `AppPalette`（`touchpadStart` / `touchpadEnd` / `touchpadBorder`）：
> 暗 `#1E3A2E → #101A16`、描边 `#2A4F3E`；亮 `#D3EFE2 → #EAF8F1`、描边 `#B1D9C8`。

### 桌面管理界面（桌面端 web-ui，1440×900）

文件：`apps/desktop/web-ui/src/styles/light.css` 与 `dark.css`。中性色与移动端同源（同样偏一点点绿），
但**品牌色拆成两个值**，这条规则必须遵守：

| Token | 暗色 | 亮色 | 说明 |
|---|---|---|---|
| `--surface-0/1/2/3` | `#0A0D0D` `#121414` `#1A1C1C` `#1F2222` | `#F5F7F7` `#FFFFFF` `#EFF3F1` `#E7ECEA` | 页面底 → 卡片 → 内嵌 → 悬浮 |
| `--border-1` / `-2` | `#212626` / `#303836` | `#E7ECEA` / `#C9D3CE` | 发丝描边 / 输入框可见边框 |
| `--text-1/2/3` | `#EDF2F0` `#9AA5A1` `#838E8A` | `#0D1212` `#4E5A55` `#5F6965` | 三级文本，全部 ≥4.5:1 |
| `--accent` | `#36D9A1` | `#0A7352` | **仅用于文字、图标、焦点** |
| `--accent-bright` | `#43E0AC` | `#12A87A` | **仅用于无文字装饰**：3px 强调条、状态点、品牌方块 |
| `--accent-strong` | `#5FE3B3` | `#095F44` | hover / active |
| `--accent-contrast` | `#08120E` | `#FFFFFF` | `--accent` 底上的前景 |
| `--accent-ink` | `#08120E` | `#0D1212` | `--accent-bright` 底上的前景（**不能用白字**） |
| `--accent-soft` / `-border` / `-ring` | `rgba(54,217,161,.14/.32/.30)` | `rgba(18,168,122,.10/.30/.24)` | 衬底 / 描边 / 焦点光环 |
| `--success` | `#3DDC84` | `#0A7352` | 比品牌绿偏黄绿 18°，避免与"品牌强调"同色 |
| `--warning` | `#F2B557` | `#8A5F00` | |
| `--error` | `#FF6B6E` | `#C62A2C` | 比移动端 `#D93B3D` 更深，为的是在自己的浅红衬底上达到 4.5:1 |
| `--primary-bg` / `-fg` | `#EDF2F0` / `#0A0D0D` | `#141917` / `#FFFFFF` | 实心主按钮仍用近黑近白，不用品牌色 |

**为什么要拆 `accent` 与 `accent-bright`**：品牌本体 `#12A87A` 在白底上只有 3.0:1，
当作链接、当前导航项、"查看更多信息"这类文字用会读不清。所以：

- 凡 `color:`（文字/图标）→ 用 `--accent`（压深后的同色相绿，亮色 5.9:1）
- 凡 `background:` 且元素内**不含文字**（强调条、状态点、7px 圆点、30px 品牌方块）→ 用 `--accent-bright`（鲜绿，非文字元素门槛只 3:1）
- 品牌方块上的前景必须是 `--accent-ink`（近黑），白字在鲜绿上只有 3.0:1

已验证：两套主题共 44 组「各级文字 × 各层背景」「语义色 × 自身衬底」组合，最低 4.73:1，
全部达到 WCAG AA。改动色值时需重跑这组验证，不要凭肉眼判断。

---

## 3. 字体

中文 `Noto Sans SC`（回退 `PingFang SC`），数字与拉丁 `Inter`。字号为逻辑像素。

| 级别 | 字号 / 行高 | 字重 | 用途 |
|---|---|---|---|
| H1 | 26 / 32 | SemiBold 600 | 页面标题 |
| H2 | 18 / 26 | SemiBold 600 | 分组标题 |
| Body | 14 / 22 | Medium 500 | 正文、卡片标题 |
| Meta | 12 / 18 | Regular 400 | 辅助说明、路径 |
| Caption | 10 / 14 | Regular 400 | 标签、角标 |

## 4. 间距 · 圆角

- **间距**：4 / 8 / 12 / 16 / 20 / 24 / 32（8pt 栅格）。组件内部用 `gap`，不用 `margin`。
- **圆角**：元素 10 · 卡片 14 · 大卡 18 · 弹层与面板 22 · 胶囊 999。
- **弹层**：顶部圆角 22，把手 36×4，内容左右 20，底部安全区额外 +16。

---

## 5. 屏幕清单与代码映射

### 移动端（14 屏 × 2 主题）

| 屏 | 设计稿名称 | 图片文件 | 对应代码（相对 `apps/mobile/lib/src/`） |
|---|---|---|---|
| S1 | 首页 | `S01-home-{dark,light}.png` | `screens/home_tab.dart`、`screens/home_status_card.dart`、`screens/home_quick_actions.dart`、`screens/home_common_apps.dart`、`widgets/app_remote_controls.dart`、`widgets/active_window.dart` |
| S2 | 库 | `S02-library-{dark,light}.png` | `screens/library_tab.dart`、`screens/library_tiles.dart`、`screens/library_sheets.dart`、`screens/library_dialogs.dart` |
| S3 | 控制中心 | `S03-control-center-{dark,light}.png` | `screens/control_center_screen.dart`、`screens/control_center/display_section.dart` |
| S4 | 控制（触控板） | `S04-touchpad-{dark,light}.png` | `screens/mouse_keyboard_screen.dart`、`screens/mouse_keyboard/touchpad_surface.dart`、`screens/mouse_keyboard/clipboard_action_buttons.dart`、`screens/mouse_keyboard/keyboard_entry_button.dart` |
| S5 | 更多 | `S05-more-{dark,light}.png` | `screens/more_tab.dart` |
| S6 | 进程管理 | `S06-processes-{dark,light}.png` | `screens/processes_tab.dart`、`widgets/process_icon.dart` |
| S7 | 配对与连接 | `S07-pairing-{dark,light}.png` | `screens/pairing_screen.dart`、`screens/home_connection.dart` |
| S8 | 设置 | `S08-settings-{dark,light}.png` | `screens/settings_tab.dart`、`screens/settings_input_section.dart` |
| S9 | 快捷键方案 | `S09-shortcuts-{dark,light}.png` | `screens/shortcuts_screen.dart`、`screens/shortcut_preset_sheet.dart` |
| S10 | 电视遥控 | `S10-tv-remote-{dark,light}.png` | `screens/ir/tv_remote_screen.dart`、`screens/ir/tv_quick_panel.dart` |
| S11 | 遥控器管理 | `S11-remote-manage-{dark,light}.png` | `screens/ir/ir_remote_manage_screen.dart` |
| S12 | 扫描结果 | `S12-scan-result-{dark,light}.png` | `screens/scan_folder_screen.dart` |
| S13 | 按键与信号 | `S13-keys-signals-{dark,light}.png` | `screens/ir/ir_command_screen.dart` |
| S14 | 控制流程 | `S14-flows-{dark,light}.png` | `screens/ir/ir_macro_screen.dart` |

### 管理端（桌面端 web-ui，1440 × 900）

管理界面不做设计稿交付，**视觉基线就是代码本身**：`apps/desktop/web-ui` 的 6 个路由页
（概览 / 应用 / 进程 / 日志 / 设置 / 配对），token 见上一节。侧栏 232、最小点按区 44、
主按钮高 52 这三条与移动端共用。

### 关键结构：S1 首页（自上而下）

状态栏 62 → Nav 56 → Body（设备状态卡 · 快捷操作 4 宫格 · 常用应用 3×4 网格）→ **Dock 窗口条 64** → TabBar 95。

> Dock 窗口条是这一版的核心改动：窗口控制（列表/最小化/最大化/关闭）是高频操作，被从设备卡里提出来做成常驻底部条，落在拇指区（y≈690–742）。设备状态卡回归纯展示。

### 关键结构：S4 控制页（自上而下）

状态栏 62 → Nav 56 → 连接状态条 32 → **触控区（撑满剩余，约 522）** → 当前窗口卡 64（含 4 个窗口按钮）→ 底部三键 46（复制 / 粘贴 / 输入文字）。窗口控制同样落在拇指区（y≈716）。

---

## 6. 弹层清单（暗/亮各一套）

查看位置：`sheets-dark-light.pdf`（第 1~12 页暗色、13~24 页亮色，每页 2 组），或设计稿 `(3400,0)` / `(3400,4700)`。

| 弹层 | 对应代码（相对 `apps/mobile/lib/src/`） |
|---|---|
| 显示模式 | `screens/control_center/display_mode_sheet.dart` |
| 音频输出 | `screens/control_center/display_section.dart` |
| 结束进程 | `screens/processes_tab.dart` |
| 快捷键方案表单 | `screens/shortcut_preset_sheet.dart` |
| 启动参数 | `screens/library_sheets.dart` |
| 删除程序 | `screens/library_dialogs.dart` |
| 启动分辨率 | `screens/library_launch_mode_sheet.dart` |
| 选择程序（文件浏览器） | `widgets/file_picker_sheet.dart` |
| 添加按键 | `screens/ir/ir_command_dialog.dart` |
| 新增控制流程 | `screens/ir/ir_macro_dialog.dart` |
| 电视快速面板 | `screens/ir/tv_quick_panel.dart` |
| 窗口列表 | `utils/remote_actions.dart` |
| 鼠标设置 | `screens/settings_input_section.dart`（从控制页 `ConnectionStatusBar.onOpenSettings` 跳设置页） |
| Windows 登录密码 | `screens/settings_tab.dart` |
| 新增遥控器 / 删除遥控器 | `screens/ir/ir_remote_manage_screen.dart` |
| 从常用移除 / 添加到库 / 重命名 | `screens/library_dialogs.dart` |
| 图标选择 | `widgets/icon_picker_sheet.dart` |
| 键盘输入 | `screens/mouse_keyboard/keyboard_input_sheet.dart` |
| 解锁 PIN | `screens/pairing_screen.dart`（配对流程内联，无独立弹层） |

**弹层通用规格**：把手 36×4 → 标题 14 SemiBold + 说明 12 → 选项行 56 → 危险项置底 → 安全区 +16。关闭方式：下拉 / 点遮罩 / 系统返回。破坏性操作必须二次确认，默认焦点在「取消」。

---

## 7. 图标实现要求

**这条最容易做错，务必遵守。**

1. **程序图标用真实品牌图形**，禁止"品牌色底 + 一个汉字/字母"的占位。已覆盖：赛博朋克 2077（黄 C）、原神（金四芒星）、Steam（圆环+活塞）、VS Code（蓝折角）、资源管理器（黄文件夹）、Chrome（三色圆+蓝心）、终端（框+`>_`）、Photoshop（深蓝底蓝框 Ps）、剪映（绿剪刀）、网易云（红音符）、OBS（同心环）、微信（双气泡）。
2. **文件浏览器**：文件夹用真实文件夹形状（`#E8B44A`），exe 用黄色程序块（`#FCEE0A` + 黑色 C），不要用通用方块。
3. **进程页**同样需要真实程序图标，通过 `widgets/process_icon.dart` 从 exe 路径取图标，取不到时才降级到首字母块。
4. 图标绘制避免 SVG 解析陷阱：`path` 不能以前导小数点开头（`.113` 会被判为 degenerate）；父容器是 flex 时子节点 `x/y` 会失效，必须用 `primaryAxisAlignItems/counterAxisAlignItems: CENTER` 居中。

---

## 8. 交互硬规则（10 条）

1. **主操作贴底** — 主按钮永远在拇指弧内，顶栏只放"我在哪"与只读状态。
2. **状态先行** — 离线 / 锁定 / UAC 三种中断用整卡表达，一屏只出现一张状态卡。
3. **即时反馈** — 命令 120ms 内出按下态；成功轻提示 2s；失败给原因并可重试，不停留在转圈。
4. **防误触** — 关机 / 结束进程 / 删除条目必须确认；危险按钮与主按钮不同排、不同色。
5. **乐观更新** — 收藏、置顶、开关先改 UI，失败回滚并说明原因。
6. **手势** — 列表项长按出菜单；弹层下拉关闭；长列表滚到底自动加载下一页。
7. **输入** — 控制页弹层常驻退格 / 删除 / 回车；PC 焦点进入输入框时自动唤起键盘。
8. **空态即引导** — 没数据时给"下一步做什么"的按钮，不放只有插画的空页。
9. **主题** — 跟随系统 / 浅色 / 深色三档，切换即时生效；两套主题层级关系完全一致。
10. **可用性** — 点按区 ≥ 44，正文对比度 ≥ 4.5:1，动效 160–220ms ease-out，尊重"减少动态"。

### 触控板手势（S4）

轻点 = 左键；双指滑动 = 滚动；长按 400ms = 右键；长按后移动 = 拖选。**不要**在底部再放左键/右键/拖拽锁按钮——手势已覆盖，重复摆放会挤掉触控面积。

---

## 9. 验收清单

- [ ] 亮暗两套都实现，切换不重载、不闪白
- [ ] 所有色值取自第 2 节，且与 `app_palette.dart` 一致
- [ ] 每个屏幕的图片逐张对照过，无遗漏区块
- [ ] 弹层全部实现，含空态与危险确认
- [ ] 程序/进程/文件图标为真实图形，无字母占位
- [ ] 最小点按区 44，长文本 `maxLines` 处理不截断
- [ ] 高频操作（窗口控制、主按钮）在 y ≥ 0.6H
