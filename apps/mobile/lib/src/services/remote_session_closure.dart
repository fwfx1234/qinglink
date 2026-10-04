/// 服务端 `session_closed` 事件的语义。
///
/// 只做判定与文案，不触碰会话状态：由 `RemoteSession` 依据
/// [stopsReconnecting] 决定是停止重连（配对已失效），还是保留配对意图并继续
/// 退避重试。
class RemoteSessionClosure {
  const RemoteSessionClosure._(
    this.stopsReconnecting,
    this.failureCode,
    this.failureMessage,
  );

  /// 解析一条 `session_closed` 事件。
  factory RemoteSessionClosure.fromEvent(Map<String, dynamic> event) {
    final data = event['data'] as Map<String, dynamic>?;
    final code = data?['code'] as String? ?? 'SESSION_CLOSED';
    if (code == 'REVOKED') {
      return const RemoteSessionClosure._(
        true,
        'INVALID_TOKEN',
        'Token has been revoked',
      );
    }
    if (code == 'REPLACED') {
      return const RemoteSessionClosure._(
        true,
        'REPLACED',
        'Session was replaced',
      );
    }
    // The PC withdrew its listener (pause / service restart) or sent an
    // unknown reason. The pairing is still valid, so keep `_wantConnection` and
    // retry with backoff; the session recovers once the PC is back.
    return RemoteSessionClosure._(false, code, _closureMessage(code));
  }

  /// 为 true 表示配对已失效，必须停止重连。
  final bool stopsReconnecting;

  /// 上报到失败流的错误码。
  final String failureCode;

  /// 上报到失败流（并面向用户展示）的文案。
  final String failureMessage;

  /// User-facing text for a `session_closed` reason the server can send.
  static String _closureMessage(String code) {
    switch (code) {
      case 'SERVICE_PAUSED':
        return '电脑已暂停远程控制';
      case 'SERVICE_STOPPING':
        return '电脑端服务正在重启';
    }
    return '电脑端连接已断开';
  }
}
