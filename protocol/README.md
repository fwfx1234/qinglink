# QingLink 协议 v2

`qinglink-protocol` 是 Rust 线型 DTO 的唯一真相来源。本目录中的 JSON Schema
是跨语言验证契约。对应的纯传输层 Dart DTO 位于
`apps/mobile/lib/src/protocol/qinglink_protocol.dart`；两种实现都按 `protocol/fixtures/`
里的同一组夹具、在 `apps/mobile/test` 的契约测试中做检查。

协议共享有意仅限于传输 DTO。领域对象和 Flutter 视图模型保持平台原生，而
HTTP/WS 载荷由 `protocol/fixtures/` 的夹具与同目录的 JSON Schema 互相校核。
**本仓库目前没有 CI 流水线**：这些校核靠 `apps/mobile/test/qinglink_contract_test.dart`
与手工执行 `cargo fmt --check` / `cargo check` / `flutter test` 完成，改动协议时
必须自己跑，不要假定有流水线兜底。协议共享使移动应用独立于 Rust 运行时，
并使 IP 变更和重连可观测。

移动端游戏目录的 `launch_mode` 是可选的显示模式对象，包含 `width`、`height` 与
`refresh_millihz`。配置由 `update_scanner_app` 保存，清除时使用
`clear_launch_mode: true`；启动和恢复策略见 [`v2.md`](v2.md)「游戏启动显示模式」。

显示设置是本协议的另一组消息：`get_displays` 返回每台显示器的**常用模式候选**
（≤4K 的 6 档分辨率 × 60/90/120/144Hz，外加当前模式兜底，服务端不返回冷门档位），
`set_display_mode` 用扁平的 `display_id` / `width` / `height` / `refresh_millihz` 四字段切换。
形状见 `qinglink-v2-ws.schema.json` 的 `displayInfo` / `displaysResponse`，语义与错误码见
[`v2.md`](v2.md)「显示设置」。

保持唤醒是第三组系统消息：`get_keep_awake` 读取服务进程开关，`set_keep_awake`（`enabled`）
切换，两者都返回 `keep_awake_status`（`data.enabled`）。桌面端托盘与设置页经桌面管理 IPC
的 `set_keep_awake` 命令读写**同一个**开关，管理快照的 `keep_awake` 字段把状态推回桌面端。
`qinglink-v2-ws.schema.json` 描述的是通用信封（`additionalProperties: true`），新增消息
沿用既有 `request` / `response` 形状，无需扩展 `$defs`。per-thread 约束、错误码与
持久化语义见 [`v2.md`](v2.md)「保持唤醒」。

`GET /api/v2/health` 的 `app_version` 是服务端产品版本号（`0.1.0+13` 形式），与桌面端
界面展示的"服务端版本"同源，也与 Windows 文件属性里的版本一致。它是可选字段：旧服务
不返回，客户端按"版本未知"处理。版本号的构成与递增规则见
[`docs/development.md`](../docs/development.md) §2.4。

上行请求类型有独立的索引：[`requests.md`](requests.md) 按功能域列出全部 95 个
`WsClientRequest` 变体（线格式 `type`、载荷字段、响应类型、实际错误码），用于反查
"某个请求类型是否已登记"。[`v2.md`](v2.md) 按主题讲**语义**（配对、显示设置、保持唤醒…），
两者互补：索引回答"有没有"，`v2.md` 回答"什么意思"。新增或删除协议变体时，
`requests.md` 必须与变体定义、路由表同批更新。
