import 'dart:async';

/// 心跳活性宽限期：PC 是否“在线”由最近一次成功会话决定，而不是 socket 瞬时状态。
///
/// 会话进入 ready（已认证）时调用 [markOnline]；离开 ready 时调用
/// [onDisconnected]：显式断开（登出/停止）立即离线，非显式断开进入宽限期，
/// 宽限期内恢复则保持在线，宽限到期仍未恢复才触发 [onOffline]。
/// App 后台时调用 [pause] 挂起判定，回前台后由连接状态重新驱动。
class OnlineGraceTracker {
  OnlineGraceTracker({
    required this.grace,
    required this.onOffline,
  });

  /// 从最后一次在线到判定离线的宽限时间。
  final Duration grace;
  final void Function() onOffline;

  bool _online = false;
  Timer? _timer;
  bool _paused = false;

  bool get isOnline => _online;

  void markOnline() {
    _online = true;
    _timer?.cancel();
    _timer = null;
  }

  void onDisconnected({required bool explicit}) {
    if (explicit) {
      _timer?.cancel();
      _timer = null;
      _online = false;
      return;
    }
    if (_paused || !_online || _timer != null) return;
    _timer = Timer(grace, () {
      _timer = null;
      if (_paused) return;
      _online = false;
      onOffline();
    });
  }

  void pause() {
    _paused = true;
    _timer?.cancel();
    _timer = null;
  }

  void resume() {
    _paused = false;
  }

  void dispose() {
    _timer?.cancel();
    _timer = null;
  }
}
