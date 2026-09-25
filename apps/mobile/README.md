# 轻联 QingLink Mobile

轻联手机端用于发现并控制局域网内的 Windows PC。应用通过 UDP 广播发现电脑，以稳定 `pc_id` 绑定设备，并在 PC 地址变化时自动更新连接。

## 首次配对

1. 手机发现 QingLink PC 后调用 `POST /api/v2/pairing/requests`。
2. PC 窗口显示 6 位匹配码，60 秒内有效。
3. 手机确认匹配码并获得长期 token、PC id 和 MAC 地址。
4. token 写入 Android Keystore / iOS Keychain 支持的安全存储，控制通道连接 `/api/v2/events` 和 `/api/v2/input`。

协议详情见仓库根目录 `protocol/v2.md`。

## 开发

```bash
flutter pub get
flutter analyze
flutter test
flutter run
```
