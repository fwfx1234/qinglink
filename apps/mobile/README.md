# 轻联 QingLink Mobile

轻联手机端用于发现并控制局域网内的 Windows PC。应用通过 UDP 广播发现电脑，以稳定 `pc_id` 绑定设备，并在 PC 地址变化时自动更新连接。

## 首次配对

1. 手机发现 QingLink PC 后调用 `POST /api/v2/pairing/requests`。
2. PC 上必须已运行并解锁桌面端：桌面端会弹出置顶的配对窗口显示 6 位匹配码（管理界面同时也显示），60 秒内有效，配对成功或超时后该窗口自动关闭；桌面端未运行或锁屏时该接口返回 423 `PAIRING_REQUIRES_DESKTOP`。
3. 手机确认匹配码并获得长期 token、PC id 和 MAC 地址。请求响应本身**不含** token：令牌只在确认阶段下发。
4. token 写入 Android Keystore / iOS Keychain 支持的安全存储，控制通道连接 `/api/v2/events` 和 `/api/v2/input`。

配对失败按 `error.code` 分支（如 `INVALID_PIN`、`PAIRING_BUSY`、`DEVICE_MISMATCH`），
映射文案集中在 `lib/src/core/errors/pairing_failures.dart`，不要把内部错误码直接展示给用户。

## 会话关闭

服务端下发 `{"type":"session_closed","data":{"code":...}}`：`REVOKED` 清除令牌重新配对，
`REPLACED` 提示已在其他客户端登录，`SERVICE_PAUSED` / `SERVICE_STOPPING` 保留令牌并退避重连，
PC 恢复后自动连回。完整表格见仓库根目录 `protocol/v2.md`。

协议详情见仓库根目录 `protocol/v2.md`。

## 应用库的三条约定

手机端不持有业务数据。下面几件事全部由服务端（`QingLinkService`）拥有，换手机、
换浏览器看到的是同一份：

- **目录分类**：`list_launcher_items` 的 `kind` 用 `app`（软件）、`game`（游戏）
  和 `favorite`（常用）。「库」页只暴露这三组加「最近 / 已屏蔽」。
- **常用与置顶**：`set_scanner_app_favorite` / `set_scanner_app_pinned` 把"我的库"
  写到 PC，手机端只读不存。旧版本存在 `SharedPreferences` 的 `quick_apps` 会在
  首次进「库」时自动迁移到服务端，随后本地副本被清空（见
  `lib/src/providers/favorites_controller.dart`）。
- **屏蔽**：`set_scanner_app_hidden` 把用不到的程序从手机列表里收起来（数据仍在 PC 上），
  `list_blocked_scanner_apps` 列出来供恢复。这是让列表变短的主要手段。
- **图标**：列表按需取图；`list_scanner_app_icons` 列出 PC 上可选的图标文件，
  `refresh_scanner_app_icon` 带或不带 `icon_path` 都能单独修好一个条目的图标
  （不带时按 Steam 官方封面 → 快捷方式图标 → 程序内嵌资源 → 同目录 PNG/ICO →
  系统默认图标 的顺序解析）；也可以浏览 PC 任意目录里的图片（见下文「换图标」）。
  服务端返回的图标恒为 128×128 正方形且素材铺满画布，
  所以手机端只要用固定尺寸的 `BoxFit.contain` 容器即可，不必再为不同条目调整尺寸。

扫描目录时 `scan_game_directory` / `add_scanned_games` 都接受 `kind`：目录扫描不再一律
当成游戏，服务端也不会为了确认选择而重扫一遍目录。

## 手机端与服务端的字段契约

下面几条是手机端与服务端最容易写错的地方，改动这两端时一起核对：

- **重命名**用 `rename_scanner_app` 的 `new_name` 字段（不是 `name`）；空名恢复扫描名。
- **窗口列表**用 `list_windows`（`force_refresh` 可绕过缓存），响应信封是
  `{"type":"window_list","data":[{...}]}`：`data` 是**数组**，不是 `{"windows":[...]}` 对象；
  数组元素的句柄字段名是 `id`。切换窗口再把该值作为 `focus_window` 的 `hwnd` 回传。
- **新增条目**（`add_scanner_app`）必须带 `category`：忽略大小写等于 `game` 归入游戏，
  其余归入软件。手机端跟随当前分区提交，避免刚加的程序落在另一个分区里。
- **游戏启动分辨率**：游戏条目菜单可设置主显示器的分辨率与刷新率；保存到 PC 目录，
  启动前应用、所有已配置游戏退出后恢复。并行启动时采用首个已配置游戏的模式，切换失败仍启动游戏。
- **扫描完成**由服务端主动推送 `{"type":"launcher_catalog_ready","kind":"app|game"}`；
  `LibraryController` 订阅该事件并按当前分区重拉一页，用户不必手动刷新。手机端不轮询扫描结果。
- **快捷键面板**通过 `key_event` 发送原始 HID 用法码与修饰键掩码（标签到用法码的翻译在
  `lib/src/services/shortcut_mapper.dart`）。服务端没有名为 `shortcut` 的命令，无法翻译的标签
  要给出可读提示，不能发一条服务端不认识的请求。
- **写操作命令**（重命名 / 删除 / 屏蔽 / 常用 / 置顶 / 新增）由 `LauncherController` 统一收口成
  "返回 `bool` 或 `null`、不抛异常"：这些动作都在弹窗关闭后的异步回调里执行，异常逃逸只会变成
  控制台里的未捕获错误，用户看到的是"点一下就报错"。
- **换图标只提供"选择"，不提供"填写"**：面板列出 PC 上检测到的候选图标文件，也可以浏览 PC
  任意目录里的图片（`png` / `jpg` / `bmp` / `ico`），两者都带缩略图——手机看不到 PC 的文件
  内容，缩略图由 `get_icon_preview` 按路径取（只解码、不落库，见 `protocol/v2.md`）。选中后
  仍用 `refresh_scanner_app_icon` 带 `icon_path` 落库。换完必须重新同步列表：`icon_source`
  在列表数据里，不刷新的话条目上显示的图标来源不会更新。变更标记由调用方持有
  （`showIconPickerSheet` 返回 `bool`），所以面板被拖拽关闭也照样触发这次刷新。
- **快捷键方案的按键通过软键盘点选**（`ShortcutMapper.keyGroups`），不是手打：标签写错了服务端
  只会回 `INVALID_REQUEST`，编辑器会在生成前组合并校验按键，避免写出服务端无法识别的标签。
- **首页应用顺序**：首页最多显示 12 张应用卡片。服务端最近启动记录中的第一项固定在首位，
  后面按置顶列表补齐并去重；只有首张卡片显示“上次使用”。
- **快捷键自由组合**：快捷键方案编辑器使用软键盘按钮组合修饰键和主键，例如依次选择 `Alt`
  与 `Z` 会生成 `Alt + Z`，支持多修饰键、字母、数字、符号、功能键、方向键和导航键。
  方案卡片里的快捷键标签可直接加入或移出首页；首页最多固定 12 个，按加入顺序保存到本机设置，
  点首页按钮会发送该组合键。删除方案中的组合时，只有其他方案也不再引用它才会从首页移除。
- **分辨率与刷新率**：控制中心通过 `get_displays` 读取**1080p 及以上的可选模式**——枚举到的
  真实分辨率（1080p、1440p、1600p、带鱼屏、4K 都算）全部保留，不再按固定白名单裁剪；同一
  分辨率下按整 Hz 归档刷新率（相差 1Hz 视为同一档，59/60、119/120 会合并），每档最多 4 个
  刷新率并列可选；当前模式不在列表里时服务端会把它最后追加进去，保证能高亮也能切回来。
  多显示器时先选屏再选模式；选择后用扁平的
  `display_id`、`width`、`height`、`refresh_millihz` 字段调用 `set_display_mode`，失败原因
  直接显示在提示里，列表读取失败时分组内会出现错误行与重试入口。
- **保持唤醒**：控制中心新增独立分组，`get_keep_awake` 读取 PC 上服务进程的开关，
  `set_keep_awake`（`enabled`）切换，成功后按回包的 `keep_awake_status` 更新；状态未读到时
  开关禁用，避免把「未知」当成「已关闭」。该开关由服务进程用一条常驻线程持有
  （`SetThreadExecutionState` 是 per-thread 状态），持久化在服务库并在启动时恢复；
  桌面端托盘/设置页经管理 IPC 读写同一开关，按快照同步，不再各自持有独立状态；
  协议见 `protocol/v2.md`「保持唤醒」。
- **触控板手势**：轻点 = 左键单击，长按 400ms 不动后松手 = 右键单击，长按后移动超过 3px
  进入拖拽（左键按下 → 移动 → 抬起）。**拖拽松手不再补发右键**：长按拖动的语义是"按住左键
  搬运"，补一次右键会让资源管理器在放下文件后弹出上下文菜单。
- **复制 / 粘贴 / 全选**：控制页底部与键盘面板底部各有一组按钮，作用对象是 PC 当前选中的内容
  （发送 `Ctrl + C` / `Ctrl + V` / `Ctrl + A`），不做手机与 PC 之间的剪贴板同步。
- **输入框回填**：面板**先滑出**，同时并行用 `get_foreground` 的 `focus_text` 读取 PC 焦点
  输入框已有文字，回来后作为手机输入框的基线（文本通常比面板晚几十到几百毫秒出现；用户若已
  抢先输入，迟到的原文直接丢弃、不覆盖）。服务端只认编辑框的 `ValuePattern`，密码框、只读
  输入框、只暴露文本树的自绘控件与占位提示都按空基线处理；Chromium 系（Chrome / Edge /
  Electron / WebView2）额外按结构判断——带子元素的 `Edit` 是富文本编辑器，`Value` 里可能混着
  占位提示，一律不回填，而纯 `<input>` / `<textarea>`（地址栏、搜索框、普通表单）照旧回填。
  每次打开面板都重新请求，请求失败一律当空，不复用上一次的结果，面板上会有灰字提示。尾部
  追加/退格按差异发送；在中间改字会退化为"Ctrl+A 全选后整体重写"，代价是 PC 侧撤销栈被清空。
- **PC 侧鼠标状态**：服务端推送 `input_state` 事件，触控板左上角显示"左键按住"之类的角标；
  进入拖拽时若 PC 侧左键已按住，不再重复下发按下。
- **自动唤起键盘**：PC 焦点第一次落进输入框时，服务端推送 `focus_input`，控制页收到后直接打开
  键盘面板，把手机当输入工具用。服务端只在"进入"的边沿推送，所以客户端不做去重：焦点离开
  不收起，同一个输入框里继续点也不会重弹；面板已经开着时忽略该事件；手机退到后台或锁屏时
  不响应。自动弹出会抢走用户正在进行的触控操作，因此**设置页「输入」分组提供开关**
  （`auto_open_keyboard`，默认开启）；关掉后 `focus_input` 事件一律丢弃，键盘仍可点控制页的
  键盘按钮手动打开。**焦点在弹层入场动画结束后才请求**（首帧请求会让 Android 丢掉这次输入法 show），
  Android 上 400ms / 800ms 仍未看到输入法时最多补发两次 `TextInput.show`；打开面板时照旧用
  `focus_text` 建基线（密码框拿到空基线，打字仍能正常发送）。

## 设置

S8 是**手机端唯一的设置界面**，不再有第二处开关：

- 分组顺序：电源 → 解锁与唤醒（PIN、唤醒后自动解锁、唤醒电脑）→ 输入（自动弹出键盘、
  自然滚动、鼠标速度）→ 电视联动（跟随电脑开机 / 关机）→ 连接信息（重新发现、设备、
  地址、版本）→ 断开当前连接 / 清除所有设置。
- 入口只有两个，都指向同一页：控制页顶栏齿轮（`mouse_keyboard/connection_status_bar.dart`
  的 `onOpenSettings` → `AppRoutes.settings`）与「更多」页的「设置」行。齿轮以前弹的是只含
  「鼠标速度 + 自然滚动」的局部面板，导致「自动弹出键盘」这类开关被藏在另一条路径上。
- 鼠标速度原先只存在于控制页的局部面板；现在并入设置页的「输入」分组，控制页仍通过
  `settingsProvider` 的监听同步灵敏度，改完回到控制页即刻生效，那个局部面板已删除。
- 新增开关只加在 `screens/settings_input_section.dart` / `screens/settings_tv_section.dart`，
  不要在其他页面复制一份状态。

## 检查更新

设置页「连接信息」分组提供**检查更新**入口（`AppRoutes.update` →
`screens/update_screen.dart`），这个页面不要求已配对。版本号不再写死在设置页，
由 `appVersionProvider`（`providers/app_version_provider.dart`）通过
`package_info_plus` 从平台读取。

应用启动约 3 秒后会**自动检查一次**（`main.dart` 用外部 `ProviderContainer` 在首帧后
触发），设置页的「检查更新」是随时可重复执行的手动入口。**发现新版本立即后台下载**，
下载完成后停在「安装」等待用户操作；安装会中断当前使用，所以始终手动触发。

更新来源是本项目 GitHub Releases，只认 `mobile-v` 前缀的 tag：

- tag 格式 `mobile-v<语义版本>+<构建号>`，例如 `mobile-v1.0.0+6`；语义版本部分
  必须与 `pubspec.yaml` 的 `version` 前半段一致，CI 会校验，不一致直接失败。
- 构建号参与版本比较：先比 `major.minor.patch`，相等再比 `+` 后的构建号。
  只接受 tag 版本**严格新于**当前版本且带 APK 资产的正式 release（跳过 draft
  和 prerelease）。
- 版本基础设施见根目录 `docs/development.md` 的「发布流水线」。

**Android** 是完整链路：检查 → 后台下载 → 校验 → 手动安装。

- CI 产出两个资产：`QingLink-mobile-<版本>-android.apk` 与同名校验清单
  `QingLink-mobile-<版本>-android.apk.sha256`（GNU sha256sum 行格式）。
  清单文件名跟着 APK 走是刻意的：旧版客户端只会请求「APK 地址 + `.sha256`」，
  换成 `SHA256SUMS` 会让它们 404、彻底断掉升级路径。新客户端两种命名都认
  （优先 `SHA256SUMS`，回落同名 `.sha256`）。
  下载前先解析清单取 APK 的期望哈希，下载完成后逐块计算 SHA256，
  不一致就报错且不保留安装入口。
- APK 落在应用私有目录 `updates/`，用 `open_filex` 拉起系统安装器；`open_filex`
  自带 FileProvider 覆盖该目录，不需要再自定义 FileProvider。
- `AndroidManifest.xml` 声明 `REQUEST_INSTALL_PACKAGES`，Android 8+ 首次安装仍需用户在
  系统弹窗里允许「安装未知应用」；权限被拒时界面给出提示，不自动重试。
- 下载不是自动安装：完成后停在「安装」按钮，由用户决定何时打断当前操作去安装。

### 发布签名

Android 只允许「同包名 + 同签名」的 APK 覆盖安装。签名一变，已安装的用户就只能
卸载重装，本地数据全丢；而 CI 每次都跑在全新的 runner 上，AGP 默认的 debug 密钥
是现生成的（证书 DN 为 `CN=Android Debug`），等于**发一次版换一次签名**——
这正是 `1.0.0+6` 及更早版本在应用内更新时报「签名冲突」的原因。

因此固定一份发布密钥，CI 与本地共用：

- `android/keystore/qinglink-release.jks`（PKCS12，RSA 4096，有效期 10000 天，
  alias `qinglink`）与 `android/key.properties` **都随源码提交**。
- `android/app/build.gradle.kts` 读 `key.properties` 生成 `release` 签名配置，
  `buildTypes.release` 用它；只有配置缺失时才退回 debug（仅供本地试验）。
- CI 打包后调用 `apksigner verify --print-certs` 硬校验证书 SHA-256，指纹不符
  直接失败，避免某天悄悄退回 debug 签名。

当前指纹（证书 SHA-256）：

```
93:17:DC:EC:24:EE:30:C4:40:F3:BB:27:50:5A:70:E0:0C:46:B0:5F:CA:71:C9:6D:8A:02:6F:AF:A9:D3:08:D7
```

密钥是**刻意公开**的：它丢失意味着所有老用户永久无法在应用内更新，代价远大于
公开的风险；更新渠道本身仍受仓库权限与 Release 内 SHA256 清单保护。真要轮换
（例如上架应用商店改用 Play App Signing）就等于让老用户卸载重装一次，需同步改
CI 里的期望指纹。

`1.0.0+7` 是第一个使用固定签名的版本，从更早的版本升级必须先卸载一次；
此后各版本可正常覆盖升级。

**iOS** 只做检查和跳转：GitHub Release 无法用于 iOS 侧载，release 里也没有 iOS 安装包，
所以检测到新版本后只展示版本号、更新说明和「前往发布页」按钮，不提供下载与安装。
当前自动流水线也只产出 Android APK；iOS 包仍按原有方式单独发布。

相关实现：`models/update_info.dart`（版本解析与 release 模型）、
`data/repositories/update_repository.dart`（GitHub API、流式下载、SHA256 校验）、
`providers/update_provider.dart`（状态机与安装动作）。

## 进程页图标

进程页（`lib/src/screens/processes_tab.dart`）的图标**按行懒取**，不随进程列表返回：

- 每行通过 `get_process_icon`（按 PID）请求，`ProcessIconController`
  （`lib/src/providers/process_icon_controller.dart`）按小写 exe 路径缓存：同一个程序
  的多个 PID 只请求一次、也只解码一次。
- 一次最多 2 个在途请求、最多排队 64 行：滑动时 build 出的行很多，放开并发会让服务端
  的图标解析占住输入通道。
- 服务端只返回程序自带的内嵌资源或同目录素材；解析不到的进程保留占位图标
  （`ProcessIcon` 退化成 `TileIcon`），不当成错误提示给用户。
- 服务端明确拒绝（旧版本没有这条命令）会记一条"这个路径没有图标"；连接层异常（断线、
  超时）不记，等行重建时自动重试。下拉刷新会清掉失败记录再试一次。

协议字段与 `source` 取值见仓库根目录 `protocol/v2.md` 的「进程图标」。

## 工程结构

`apps/mobile` 是一个标准 Flutter 工程，有几处需要说明：

- `android/`、`ios/` 是**交付目标**。
- `windows/` 是 `flutter create` 生成的桌面脚手架，**只用于在 PC 上快速预览界面，不是
  交付目标**；不要往里面加平台特有逻辑。
- `design/` 是设计资源（`icon/` 下的 SVG 源与预览图、`prototype/` 的 HTML 原型），
  **不参与构建**。应用图标按 `design/icon/*.svg` 导出到各平台的尺寸目录；历史上那个
  依赖本机 `sharp` 路径的生成脚本已经删除。
- `test/` 是既有测试文件，按 [`AGENTS.md`](../../AGENTS.md) §4 保留；但交付门槛是
  `flutter analyze`，不是测试套件。

## 开发

```bash
flutter pub get
flutter analyze   # 静态检查：本仓库的交付门槛
flutter run
flutter test      # 既有测试，可选；AGENTS.md §4 不要求运行测试套件
```

## 界面设计系统

界面按 `docs/design/DESIGN-SPEC.md` 实现，色值、字号、尺寸的**唯一权威来源是该文档第 1~4 节**，
代码侧只在下面几个文件里落成 token，业务代码不要写死色值和字号。

| 内容 | 位置 | 说明 |
| --- | --- | --- |
| 语义色板 | `lib/src/theme/app_palette.dart` | `AppPalette.dark` / `.light`：背景、表面、描边、文字、品牌色、状态色、弹层遮罩等 |
| 当前主题色门面 | `lib/src/theme/app_theme.dart` | `AppColors.primary` 等；由 `MaterialApp.builder` 按实际亮度注入，因此随明暗切换 |
| 字号体系 | `lib/src/widgets/app_text_styles.dart` | H1 26 / H2 18 / Body 14 / Meta 12 / Caption 10 |
| 圆角·间距·尺寸 | `lib/src/theme/app_theme.dart` | `AppRadius` / `AppSpacing` / `AppMetrics`（页面 16、点按区 44、主按钮 48、列表行 56） |

几条必须遵守的落地规则：

- **不要新增 `const` 里引用主题色的写法**。`AppColors.xxx` 是随主题变化的 getter，不是编译期常量；
  需要 `const` 的地方用 `context.palette.xxx` 或写字面色值。
- **窗口控制贴底**。首页是常驻底部 Dock 条（`WindowDockSection`），控制页在触控区下方
  （`ActiveWindowSection`），两者共用 `lib/src/widgets/window_controls.dart` 的按钮组；
  不要把「列表/最小化/最大化/关闭」搬回页面上部。
- **顶栏只放“我在哪”**。`HomeTopBar` 只渲染「轻联 QingLink」+ 页面标题 + 主题切换，
  主操作（添加、扫描、启动）一律落在拇指区。
- **图标用真实图形**。程序图标走 `AppIcon` / `ProcessIcon` 按需从 PC 取，取不到才退回图标底衬，
  不要用“色块 + 一个字母”占位。
- **长文本一律 `maxLines: 1` + `overflow: ellipsis`**（窗口标题、路径、进程名）。
- **轻提示走顶部覆盖层**。所有 `show*SnackBar` 扩展方法最终调用 `widgets/top_toast.dart` 的
  `showTopToast`：提示钉在安全区顶部、高度随内容增长，同一时刻只保留一条。不要再给
  `SnackBar` 配一个"接近屏高"的底部 margin 把它顶到上方——提示一旦换行就会超出屏幕、
  内容被裁掉（历史缺陷）。

### 屏幕实现状态

S1~S14 已按设计稿逐张实现（暗/亮两套走同一套 token，切换不重建页面）：

| 屏 | 页面 | 关键文件 |
| --- | --- | --- |
| S1 | 首页 | `screens/home_*.dart`、`widgets/window_controls.dart` |
| S2 | 库 | `screens/library_*.dart` |
| S3 | 控制中心 | `screens/control_center_screen.dart`、`screens/control_center/*` |
| S4 | 控制（触控板） | `screens/mouse_keyboard*` |
| S5 | 更多 | `screens/more_tab.dart` |
| S6 | 进程管理 | `screens/processes_tab.dart` |
| S7 | 配对与连接 | `screens/pairing_screen.dart`（全屏页，非弹窗） |
| S8 | 设置 | `screens/settings_tab.dart`、`screens/settings_*.dart` |
| S9 | 快捷键方案 | `screens/shortcuts_screen.dart` |
| S10 | 电视遥控 | `screens/ir/tv_remote_screen.dart`、`ir/ir_remote_panel.dart` |
| S11 | 遥控器管理 | `screens/ir/ir_remote_manage_screen.dart` |
| S12 | 扫描结果 | `screens/scan_folder_screen.dart` |
| S13 | 按键与信号 | `screens/ir/ir_command_screen.dart` |
| S14 | 控制流程 | `screens/ir/ir_macro_screen.dart` |

- **次级页顶栏**统一为「返回键（44 tonal）+ 左对齐 H1 标题 + 右侧一个主操作」，
  主操作是品牌色文字按钮或 `＋新增` 实心小按钮，不再用居中 AppBar。
- **弹层**一律走 `widgets/app_sheet.dart` 的 `showAppSheet`：圆角 22、把手 36×4、
  左右 20、底部安全区 +16、遮罩 `scrim`；内容用 `SheetHeader` / `SheetOption` 拼，
  不要各自调用 `showModalBottomSheet`。
- 管理端四屏（A1~A4）是独立静态工作台，见 `apps/admin/`（`index.html` 直接打开即可）。

## 尺寸密度与真机核对（2026-11）

设计稿画布是 **390dp 宽**（iPhone 类机型），而目标安卓机逻辑宽度多为 **360dp**，
同一套 dp 值在 360dp 屏上会被整体放大 ~8%，观感偏「大」。因此全站尺寸在 SPEC 基础上
统一收了一档（≈0.92）：H1 26→24、H2 18→16、Body/Card/Row 14→13、Meta/Label 12→11、
行高 60→56、列表图标 44→40、快捷瓦片 48→44、常用应用图标 56→48、底部导航 68→62、
窗口键 40×36→36×32。点按区下限仍是 `AppMetrics.minTap = 44`，不要再往下压。

**量尺寸别靠眼睛**：`.shots/_measure.py` 会打印截图的横向边界 / 内容行带（`bands` 与
`edges` 两种模式），配合 `_crop.py` 放大局部，能直接比对设计稿与真机的 dp 值。首页就是
靠它定位到三处「多出来」的留白：状态卡 78→56（内边距 16→8、图标 44→40）、快捷网格行距
90→76（`mainAxisExtent` 76→70、间距 14→6）、常用应用行距 100→84（extent 84→74、
间距 16→10）。**设计稿的行距是硬指标，别用 `SizedBox` 随手估。**

同类问题还有两处结构性偏差，已按设计稿改正：

- 进程列表原本「每行一张卡片 + 6dp 间隔」，设计稿是「每个分组一张整卡、行间 1dp 分隔线」，
  现在按 `_ProcessGroup` 分组渲染。
- 电视遥控的方向盘原为固定 280，占屏宽 84%（设计稿约 50%），摇杆 72×184（设计稿 60×84）。
  现在方向盘取 `min(可用短边, 屏宽×0.56, 200)`，摇杆 60×84、电源键 60、静音键 52。

两个真机踩到的坑，记在这里避免重犯：

- **`BoxDecoration` 同时给 `border` 和 `borderRadius` 时，子内容会整块不绘制。**
  表现是列表首/末行变成一条空白卡片（高度对、内容全无）。库列表的「一张卡片多行」因此
  改成：`DecoratedBox` 只画直角描边，圆角由外层 `ClipRRect` 裁（见 `library_tiles.dart`）。
- **飞书/系统通知会盖住顶栏**。真机截图对不上顶栏时，先确认不是「已连接到无线调试」这类
  抬头通知，必要时临时 `adb shell settings put global heads_up_notifications_enabled 0`，
  拍完记得改回 1。
