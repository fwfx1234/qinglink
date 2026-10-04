# 轻联 QingLink Mobile

轻联手机端用于发现并控制局域网内的 Windows PC。应用通过 UDP 广播发现电脑，以稳定 `pc_id` 绑定设备，并在 PC 地址变化时自动更新连接。

## 首次配对

1. 手机发现 QingLink PC 后调用 `POST /api/v2/pairing/requests`。
2. PC 上必须已运行并解锁桌面端，Web 界面显示 6 位匹配码，60 秒内有效；桌面端未运行或锁屏时该接口返回 423 `PAIRING_REQUIRES_DESKTOP`。
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
  系统默认图标 的顺序解析）。服务端返回的图标恒为 128×128 正方形且素材铺满画布，
  所以手机端只要用固定尺寸的 `BoxFit.contain` 容器即可，不必再为不同条目调整尺寸。

扫描目录时 `scan_game_directory` / `add_scanned_games` 都接受 `kind`：目录扫描不再一律
当成游戏，服务端也不会为了确认选择而重扫一遍目录。

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
