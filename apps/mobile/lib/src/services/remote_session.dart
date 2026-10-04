/// 远程会话服务的桶文件。
///
/// 原 `remote_session.dart` 已按职责拆分到 `lib/src/services/` 下的独立库：
/// 编排与状态在 `remote_session_client.dart`，公开契约在
/// `remote_session_types.dart`。本文件只做 `export`，使既有的
/// `import '.../services/remote_session.dart';` 调用方无需任何改动。
library;

export 'remote_session_client.dart';
export 'remote_session_types.dart';
