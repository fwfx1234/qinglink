# 给开发 Agent 的启动指令

> 复制下面整段，粘到你的开发 agent（Cursor / Claude Code / WorkBuddy 等）对话框里，再指定要做的屏幕。

---

## 可直接粘贴的 Prompt

```
我要把 QingLink 移动端按新设计稿重做。设计交付包在 docs/design/。

开始前请按顺序做三件事：
1. 读 docs/design/DESIGN-SPEC.md 全文（token、尺寸、屏清单、代码映射、交互规则都在里面）
2. 打开 docs/design/screens/ 下对应的 PNG（2x，780×1688），逐块看，不要凭描述猜
3. 读要改的那几个 Dart 文件的现状，再动手

硬约束（违反任一条就是返工）：
- 色值一律取 DESIGN-SPEC.md 第 2 节的 token 表，不要用 app_theme.dart 里的旧值。AppPalette 需要按第 2 节的映射表替换，AppColors.primary/info/violet/warning/error 都要拆成随主题变化。
- 手机基准 390×844，页面左右 18，卡片内边距 16，最小点按区 44，主按钮高 52。
- 高频操作必须在拇指区（y ≥ 0.6×844 ≈ 506）。窗口控制（列表/最小化/最大化/关闭）在首页是常驻底部 Dock 条，在控制页在触控区下方——都不要搬回页面上部。
- 程序、进程、文件图标必须是真实图形，禁止"色块 + 一个汉字/字母"的占位。
- 亮暗两套都要有，层级关系完全一致。

本轮只做：[在这里填 S1 首页 / S4 控制页 / ...]

做完后自查：对照 PNG 逐块核对；跑 flutter analyze；把改了哪些文件、哪些设计细节没做到列出来。
```

---

## 分屏任务时可以这样指定

| 想做的范围 | 粘这段 |
|---|---|
| 第一屏 | `本轮只做 S1 首页（screens/home_tab.dart 及其子组件），暗色和亮色都要。` |
| 触控板 | `本轮只做 S4 控制页（screens/mouse_keyboard_screen.dart），注意窗口卡在触控区下方、底部三键是复制/粘贴/输入文字。` |
| 弹层 | `本轮只做 S2 库页相关的 4 个弹层（library_dialogs.dart、library_sheets.dart、library_launch_mode_sheet.dart、file_picker_sheet.dart）。弹层图在 docs/design/sheets-dark-light.pdf，第 1~12 页暗色、13~24 页亮色。` |
| 主题底座 | `本轮只改 theme/app_theme.dart，按 DESIGN-SPEC.md 第 2 节替换 AppPalette 和 AppColors，不动任何页面。` |

---

## 建议的推进顺序

1. **主题底座** — 先改 `app_theme.dart`，否则后面每屏都要返工改色。
2. **S1 首页** — 结构调整最大（新增底部 Dock 窗口条），先做能定下组件风格。
3. **S4 控制页** — 结构第二复杂（触控区撑满 + 窗口卡下移）。
4. **S2 / S5 / S6** — 列表类页面，套用首页定下的卡片和行样式。
5. **S8~S14** — 次级页，复用既有组件。
6. **弹层** — 最后统一做，26 组规格高度一致。
7. **A1~A4 管理端** — 当前 `apps/` 下没有管理端工程，需要先定技术栈。

---

## 常见坑（提前告诉 agent）

- `AppColors.primary` 现在是常量 `#54D6A6`，新稿亮色是 `#12A87A`、暗色是 `#36D9A1`，**必须改成从 `AppPalette` 取**，否则亮色主题对比度不够。
- 页面底部要给 TabBar 预留 96，否则最后一行被导航栏压住。
- 长文本（窗口标题、路径、进程名）统一 `maxLines: 1` + `overflow: ellipsis`，不要让它折行挤破布局。
- 触控板不要再放左键/右键/拖拽锁按钮，手势已经覆盖（轻点/长按 400ms/长按拖选）。
- 处理图标降级：真实图标取不到时才回退字母块，不要默认就用字母块。
