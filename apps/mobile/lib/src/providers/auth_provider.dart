/// 连接与认证 provider 的桶文件。
///
/// 原单文件已按职责拆分到 `lib/src/providers/` 下的独立库：
/// - `connection_controller.dart`：`ConnectionController` 生命周期与公开 API；
/// - `session_connector.dart`：会话建立、重连与令牌失效恢复；
/// - `pairing_flow.dart`：配对请求的发起、确认与取消；
/// - `discovery_coordinator.dart`：局域网发现与端点应用；
/// - `auth_failure_policy.dart`：认证失败错误码分类与提示文案；
/// - `auth_state_transitions.dart`：重连成功后的认证状态恢复；
/// - `auth_state_writer.dart`：协作对象共用的状态读写入口。
///
/// 本文件只做 `export`，既有 `import '.../providers/auth_provider.dart';`
/// 调用方无需任何改动。
library;

export 'auth_state_transitions.dart';
export 'connection_controller.dart';
