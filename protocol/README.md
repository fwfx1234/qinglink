# QingLink 协议 v2

`qinglink-protocol` 是 Rust 线型 DTO 的唯一真相来源。本目录中的 JSON Schema
是跨语言验证契约。对应的纯传输层 Dart DTO 位于
`apps/mobile/lib/src/protocol/qinglink_protocol.dart`；两种实现都按 `protocol/fixtures/`
里的同一组夹具、在 `apps/mobile/test` 的契约测试中做检查。

协议共享有意仅限于传输 DTO。领域对象和 Flutter 视图模型保持平台原生，而
HTTP/WS 载荷由 CI 中的夹具和 Schema 验证。这使得移动应用独立于 Rust
运行时，并使 IP 变更和重连可观测。

移动端游戏目录的 `launch_mode` 是可选的显示模式对象，包含 `width`、`height` 与
`refresh_millihz`。配置由 `update_scanner_app` 保存，清除时使用
`clear_launch_mode: true`；启动和恢复策略见 [`v2.md`](v2.md)「游戏启动显示模式」。
