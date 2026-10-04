import '../models/auth.dart';

/// 认证状态的受控读写入口。
///
/// `ConnectionController` 持有连接状态；按职责拆出的协作对象（会话、配对、
/// 设备发现）通过本对象读写状态，从而共用同一套释放保护，避免各自重复判断
/// 控制器是否已释放。
class AuthStateWriter {
  /// [read] 读取当前状态，[write] 写入新状态，[isDisposed] 判断控制器是否已释放。
  AuthStateWriter({
    required this._read,
    required this._write,
    required this._isDisposed,
  });

  final AuthState Function() _read;
  final void Function(AuthState newState) _write;
  final bool Function() _isDisposed;

  /// 当前认证状态快照。
  AuthState get state => _read();

  /// 控制器是否已释放。
  bool get isDisposed => _isDisposed();

  /// 写入新状态；控制器已释放时忽略。
  void update(AuthState newState) {
    if (_isDisposed()) return;
    _write(newState);
  }
}
