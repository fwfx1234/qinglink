import 'ws_client.dart';

/// 远程会话的连接状态。
enum RemoteSessionState {
  disconnected,
  connecting,
  authenticating,
  ready,
  suspended,
  closing,
}

/// 认证失败（含服务端拒绝与会话被顶替）时抛出的失败。
class RemoteSessionAuthFailure implements Exception {
  /// 服务端返回的错误码，或客户端归一后的错误码。
  final String code;

  /// 面向用户或日志的说明。
  final String message;

  const RemoteSessionAuthFailure(this.code, this.message);

  @override
  String toString() => '$code: $message';
}

/// 构造 [WsClient] 的工厂，便于测试注入假实现。
typedef RemoteSessionClientFactory =
    WsClient Function({
      required String baseUrl,
      required String path,
      required String token,
      void Function(String)? onLog,
    });

/// 默认客户端工厂：主通道（/api/v2/events）与输入通道
/// （/api/v2/input）共用的构造方式。
WsClient defaultRemoteSessionClientFactory({
  required String baseUrl,
  required String path,
  required String token,
  void Function(String)? onLog,
}) {
  return WsClient(
    baseUrl: baseUrl,
    path: path,
    token: token,
    onLog: onLog,
    autoReconnect: false,
    // 服务端每 15 秒心跳一次，客户端健康检查作为对端失联的兜底。
    enableHealthChecks: true,
  );
}
