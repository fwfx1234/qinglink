import 'dart:async';

import 'package:flutter/foundation.dart';

import 'remote_session_types.dart';
import 'ws_client.dart';

/// 主通道（/api/v2/events）的连接与重连。
///
/// 负责按需创建 [WsClient]、订阅其状态/事件流、在替换连接时让代际号失效，
/// 以及指数退避重连的定时与间隔计算。会话状态、令牌与输入通道不在这里维护，
/// 通过回调交回 `RemoteSession` 编排。
class RemoteSessionConnection {
  /// [isDisposed] 返回会话是否已释放；[onDisconnected] 在当前传输断开且
  /// 会话仍希望保持连接时调用；[onEvent] 转发当前传输的一条事件；
  /// [onReconnectDue] 在退避定时器到点时调用。
  RemoteSessionConnection({
    required RemoteSessionClientFactory clientFactory,
    required bool Function() isDisposed,
    required void Function() onDisconnected,
    required void Function(Map<String, dynamic> event) onEvent,
    required void Function() onReconnectDue,
  }) {
    _clientFactory = clientFactory;
    _isDisposed = isDisposed;
    _onDisconnected = onDisconnected;
    _onEvent = onEvent;
    _onReconnectDue = onReconnectDue;
  }

  static const _initialReconnectDelay = Duration(milliseconds: 500);
  static const _maxReconnectDelaySeconds = 30;
  static const _fastReconnectInterval = Duration(seconds: 3);

  late final RemoteSessionClientFactory _clientFactory;
  late final bool Function() _isDisposed;
  late final void Function() _onDisconnected;
  late final void Function(Map<String, dynamic> event) _onEvent;
  late final void Function() _onReconnectDue;

  WsClient? _transport;
  StreamSubscription? _transportStateSubscription;
  StreamSubscription? _transportEventSubscription;
  int _generation = 0;
  Timer? _reconnectTimer;
  int _reconnectAttempt = 0;
  Duration? _fastReconnectIntervalOverride;

  /// 当前主通道；未建立或已替换时为 null。
  WsClient? get transport => _transport;

  /// 代际号：替换连接后自增，用于让在途回调失效（输入通道也依赖它）。
  int get generation => _generation;

  /// 是否已安排重连。
  bool get isReconnectScheduled => _reconnectTimer != null;

  /// 为 [baseUrl] 建立主通道并订阅其状态与事件。
  ///
  /// 调用方负责判断是否需要替换（先调用 [replace]），本方法只做创建与接线。
  void establish(String baseUrl) {
    final generation = ++_generation;
    final transport = _clientFactory(
      baseUrl: baseUrl,
      path: '/api/v2/events',
      token: '',
      onLog: (message) {
        if (kDebugMode) debugPrint('[QingLink][WS] $message');
      },
    );
    _transport = transport;
    _transportStateSubscription = transport.stateStream.listen((state) {
      if (_isDisposed() ||
          generation != _generation ||
          transport != _transport) {
        return;
      }
      if (state == WsConnectionState.disconnected) {
        _onDisconnected();
      }
    });
    _transportEventSubscription = transport.eventStream.listen((event) {
      if (!_isDisposed() &&
          generation == _generation &&
          transport == _transport) {
        _onEvent(event);
      }
    });
  }

  /// 丢弃当前主通道：代际号自增使在途回调失效，再取消订阅并释放连接。
  void replace() {
    _generation++;
    _transportStateSubscription?.cancel();
    _transportStateSubscription = null;
    _transportEventSubscription?.cancel();
    _transportEventSubscription = null;
    _transport?.dispose();
    _transport = null;
  }

  /// 转发动态心跳间隔到当前传输通道。
  void setHealthInterval(Duration interval) {
    _transport?.setHealthInterval(interval);
  }

  /// 安排一次退避重连；返回是否真的安排了定时器。
  ///
  /// 退避间隔：首次 500ms，之后 1s、2s、4s…最多 30s；离线/状态切换期间由
  /// [setFastReconnect] 切换为固定短间隔。[canSchedule] 与 [canFire] 由会话
  /// 提供，用于判断当下是否仍需要重连（已释放、已暂停、用户已断开等）。
  bool scheduleReconnect({
    required bool Function() canSchedule,
    required bool Function() canFire,
  }) {
    if (!canSchedule()) return false;
    final delay =
        _fastReconnectIntervalOverride ??
        (_reconnectAttempt == 0
            ? _initialReconnectDelay
            : Duration(
                seconds: (1 << (_reconnectAttempt - 1)).clamp(
                  1,
                  _maxReconnectDelaySeconds,
                ),
              ));
    _reconnectAttempt++;
    _reconnectTimer = Timer(delay, () {
      _reconnectTimer = null;
      if (canFire()) _onReconnectDue();
    });
    return true;
  }

  /// 取消待执行的重连定时器。
  void cancelReconnectTimer() {
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
  }

  /// 重置退避进度：连接成功、配对成功或收到可恢复的关闭事件时调用。
  void resetReconnectAttempt() {
    _reconnectAttempt = 0;
  }

  /// 离线/状态切换期间启用固定短间隔重连，恢复在线后关闭，加快状态感知。
  void setFastReconnect({required bool enabled}) {
    _fastReconnectIntervalOverride = enabled ? _fastReconnectInterval : null;
  }
}
