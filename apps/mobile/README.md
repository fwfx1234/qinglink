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
- **分辨率与刷新率**：控制中心先通过 `get_displays` 读取显示器和可用模式，选择后用扁平的
  `display_id`、`width`、`height`、`refresh_millihz` 字段调用 `set_display_mode`；服务端返回的
  Windows 显示 API 错误会显示在失败提示中。

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
