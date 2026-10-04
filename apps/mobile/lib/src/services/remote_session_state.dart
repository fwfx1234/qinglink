import 'dart:async';

import 'remote_session_types.dart';

/// 会话状态与失败流的广播出口。
///
/// 状态的读写与流的关闭集中在一处，`RemoteSession` 只负责在正确的时机调用，
/// 从而使连接、命令、关闭等各职责不必各自持有 [StreamController]。
class RemoteSessionStateStore {
  final _stateController = StreamController<RemoteSessionState>.broadcast();
  final _failureController =
      StreamController<RemoteSessionAuthFailure>.broadcast();

  RemoteSessionState _state = RemoteSessionState.disconnected;

  /// 当前状态。
  RemoteSessionState get state => _state;

  /// 状态变化流。
  Stream<RemoteSessionState> get stateStream => _stateController.stream;

  /// 会话级失败流（认证被拒、令牌被吊销、电脑端断开等）。
  Stream<RemoteSessionAuthFailure> get failureStream =>
      _failureController.stream;

  /// 状态去重后再广播：相同状态不重复推送。
  void setState(RemoteSessionState state) {
    if (_state == state) return;
    _state = state;
    if (!_stateController.isClosed) _stateController.add(state);
  }

  /// 以错误码与文案广播一次会话级失败；失败流已关闭时忽略。
  void emitFailure(String code, String message) {
    emitFailureObject(RemoteSessionAuthFailure(code, message));
  }

  /// 广播一个失败对象；失败流已关闭时忽略。
  void emitFailureObject(RemoteSessionAuthFailure failure) {
    if (_failureController.isClosed) return;
    _failureController.add(failure);
  }

  /// 关闭状态流与失败流。
  void close() {
    _stateController.close();
    _failureController.close();
  }
}
