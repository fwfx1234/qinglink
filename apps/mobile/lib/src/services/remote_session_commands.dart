import '../protocol/remote_protocol.dart';
import 'remote_session_types.dart';
import 'ws_client.dart';

/// 主通道上的命令发送与回执。
///
/// 就绪判定、传输获取与断开动作都由 `RemoteSession` 通过回调提供，
/// 本类只负责协议交互本身。
class RemoteSessionCommandChannel {
  /// [requireTransport] 返回已连接的主通道（否则抛出状态错误）；
  /// [isReady] 返回会话是否可收发；[state] 返回当前会话状态，仅用于错误文案；
  /// [onDisconnect] 在令牌吊销成功后断开会话。
  RemoteSessionCommandChannel({
    required WsClient Function() requireTransport,
    required bool Function() isReady,
    required RemoteSessionState Function() state,
    required void Function() onDisconnect,
  }) {
    _requireTransport = requireTransport;
    _isReady = isReady;
    _state = state;
    _onDisconnect = onDisconnect;
  }

  late final WsClient Function() _requireTransport;
  late final bool Function() _isReady;
  late final RemoteSessionState Function() _state;
  late final void Function() _onDisconnect;

  /// 发送请求并等待回执。
  Future<Map<String, dynamic>> request(
    Map<String, dynamic> payload, {
    Duration? timeout,
  }) async {
    if (!_isReady()) {
      throw StateError('Remote session is not ready (state: ${_state()})');
    }
    return _requireTransport().request(payload, timeout: timeout);
  }

  /// Typed protocol entry point. Legacy map-based calls remain available
  /// while feature controllers migrate to [RemoteRequest].
  Future<RemoteResponse> requestTyped(
    RemoteRequest request, {
    Duration? timeout,
  }) async {
    final response = RemoteResponse.fromJson(
      await this.request(request.toJson(), timeout: timeout),
    );
    if (response.isError) throw failureFromResponse(response);
    return response;
  }

  /// 发送不等待回执的命令。
  void send(Map<String, dynamic> payload) {
    if (!_isReady()) {
      throw StateError('Remote session is not ready (state: ${_state()})');
    }
    _requireTransport().send(payload);
  }

  /// Typed fire-and-forget entry point for input events.
  void sendTyped(RemoteRequest request) => send(request.toJson());

  /// 吊销当前令牌，随后断开会话。
  Future<Map<String, dynamic>> revokeToken() async {
    final response = await request({'type': 'revoke_token'});
    _onDisconnect();
    return response;
  }
}

/// 认证握手与回执校验（配对与令牌两种方式共用）。
class RemoteSessionAuthHandshake {
  /// 无状态，可复用同一实例。
  const RemoteSessionAuthHandshake();

  /// 用令牌认证；回执不是 `authenticated` 时抛出 [RemoteSessionAuthFailure]。
  Future<void> authenticateToken(WsClient transport, String token) async {
    final response = await transport.request(
      AuthenticateTokenRequest(token).toJson(),
    );
    ensureAuthenticated(response);
  }

  /// 校验认证回执；错误码取服务端给出的 code，缺省为 `AUTH_FAILED`。
  void ensureAuthenticated(Map<String, dynamic> response) {
    if (response['type'] == 'authenticated') return;
    final data = response['data'] as Map<String, dynamic>?;
    final code = data?['code'] as String? ?? 'AUTH_FAILED';
    final message = data?['message'] as String? ?? 'Authentication failed';
    throw RemoteSessionAuthFailure(code, message);
  }

  /// 校验配对回执并取出令牌；回执缺少令牌时抛出 `INVALID_AUTH_RESPONSE`。
  String extractPairingToken(Map<String, dynamic> response) {
    ensureAuthenticated(response);
    final token =
        (response['data'] as Map<String, dynamic>?)?['token'] as String?;
    if (token == null || token.isEmpty) {
      throw const RemoteSessionAuthFailure(
        'INVALID_AUTH_RESPONSE',
        'Authentication response did not contain a token',
      );
    }
    return token;
  }
}
