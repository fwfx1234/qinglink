# 安全输入发布资产

本目录存放桌面端要内嵌的 x64 Windows 服务：

```text
QingLinkService.exe
```

**这个文件是构建产物，不入库**（`.gitignore` 里的 `/apps/desktop/assets/input/*.exe`）：它的
源代码在 `services/qinglink-service`。构建桌面端之前必须先把它生成出来，否则桌面端的"安装服务"
会直接提示资源缺失（`build.rs` 本身对此是静默的，只有设置 `QINGLINK_REQUIRE_INPUT_ASSETS=1`
才会让构建失败）：

```powershell
# 仓库根目录
cargo build -p qinglink-service --bin QingLinkService
Copy-Item .\target\debug\QingLinkService.exe .\apps\desktop\assets\input\QingLinkService.exe -Force
```

注意 `build.rs` 把字节和它的 SHA256 一起内嵌，所以**改了 `services/` 之后必须重新执行上面两步
再重建桌面端**；顺序与刷新时机见 [`docs/development.md`](../../../../docs/development.md)
第 5 节「改了服务端代码」。

该服务在活动的 Windows 会话中启动自己的 LocalSystem 输入工作者，因此不需要键盘或鼠标内核驱动。推荐使用 Authenticode 以获得受信任的 UAC 发布者身份，但构建与安装都不强制要求。

桌面端安装该服务时默认**不**校验 Authenticode 签名，因此本机自建的未签名产物可以直接安装。需要强制签名时，在启动桌面端之前设置 `QINGLINK_REQUIRE_SIGNED_INPUT=1`：由 UAC 提权的安装助手会额外校验内嵌服务资源与桌面端自身都有有效的 Authenticode 签名，任一不满足即拒绝安装。无论是否强制签名，内嵌资源的 SHA256 始终核对。
