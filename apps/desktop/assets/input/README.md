# 安全输入发布资产

发布打包必须在构建桌面端之前放置以下 x64 Windows 服务：

```text
QingLinkService.exe
```

该服务在活动的 Windows 会话中启动自己的 LocalSystem 输入工作者，因此不需要键盘或鼠标内核驱动。推荐使用 Authenticode 以获得受信任的 UAC 发布者身份，但构建不强制要求。
