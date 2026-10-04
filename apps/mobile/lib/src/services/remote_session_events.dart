import 'dart:async';

/// 主通道事件的分发出口。
///
/// `session_closed` 需要先在会话层收尾（令牌、重连与状态），再原样广播给
/// 订阅者；其余事件直接广播。收尾动作通过构造函数注入，本类不持有会话状态。
class RemoteSessionEventRouter {
  /// [onSessionClosed] 在收到 `session_closed` 时调用，用于会话收尾。
  RemoteSessionEventRouter({
    required void Function(Map<String, dynamic> event) onSessionClosed,
  }) {
    _onSessionClosed = onSessionClosed;
  }

  late final void Function(Map<String, dynamic> event) _onSessionClosed;
  final _eventController = StreamController<Map<String, dynamic>>.broadcast();

  /// 会话事件流（`session_closed` 本身也会广播出去）。
  Stream<Map<String, dynamic>> get eventStream => _eventController.stream;

  /// 分发一条主通道事件。
  void dispatch(Map<String, dynamic> event) {
    if (event['type'] == 'session_closed') {
      _onSessionClosed(event);
    }
    _eventController.add(event);
  }

  /// 关闭事件流。
  void close() {
    _eventController.close();
  }
}
