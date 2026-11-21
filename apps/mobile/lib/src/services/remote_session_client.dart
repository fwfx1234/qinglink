import 'dart:async';

import '../protocol/remote_protocol.dart';
import 'remote_session_closure.dart';
import 'remote_session_commands.dart';
import 'remote_session_connection.dart';
import 'remote_session_events.dart';
import 'remote_session_input.dart';
import 'remote_session_state.dart';
import 'remote_session_types.dart';
import 'ws_client.dart';

/// Owns the WebSocket transports used by the app.
///
/// 主通道（/api/v2/events）承载认证、事件与请求；二进制输入通道
/// （/api/v2/input）用于高频鼠标位移，避免慢命令阻塞输入。
/// 输入通道按需懒连接，不影响仅浏览状态的会话。
///
/// 本类只做编排与状态维护，各职责在独立库中：
/// 连接与重连 [RemoteSessionConnection]、命令与回执
/// [RemoteSessionCommandChannel]、事件分发 [RemoteSessionEventRouter]、
/// 输入通道 [RemoteSessionInputChannel]、会话关闭语义 [RemoteSessionClosure]。
class RemoteSession {
  RemoteSession({RemoteSessionClientFactory? clientFactory}) {
    final factory = clientFactory ?? defaultRemoteSessionClientFactory;
    _stateStore = RemoteSessionStateStore();
    _events = RemoteSessionEventRouter(onSessionClosed: _handleServerClosure);
    _connection = RemoteSessionConnection(
      clientFactory: factory,
      isDisposed: () => _disposed,
      onDisconnected: _handleTransportDisconnected,
      onEvent: _events.dispatch,
      onReconnectDue: () => unawaited(connect()),
    );
    _commands = RemoteSessionCommandChannel(
      requireTransport: _requireTransport,
      isReady: () => isReady,
      state: () => _stateStore.state,
      onDisconnect: disconnect,
    );
    _inputChannel = RemoteSessionInputChannel(
      clientFactory: factory,
      baseUrl: () => _baseUrl,
      authenticatedToken: () => _authenticatedToken,
      generation: () => _connection.generation,
      isDisposed: () => _disposed,
      isReady: () => isReady,
    );
  }

  late final RemoteSessionStateStore _stateStore;
  late final RemoteSessionEventRouter _events;
  late final RemoteSessionConnection _connection;
  late final RemoteSessionCommandChannel _commands;
  late final RemoteSessionInputChannel _inputChannel;

  final RemoteSessionAuthHandshake _handshake =
      const RemoteSessionAuthHandshake();

  String? _baseUrl;
  String? _desiredToken;
  String? _authenticatedToken;
  Future<bool>? _connectInFlight;
  bool _wantConnection = false;
  bool _suspended = false;
  bool _disposed = false;

  /// 当前会话状态。
  RemoteSessionState get state => _stateStore.state;

  /// 会话状态变化流。
  Stream<RemoteSessionState> get stateStream => _stateStore.stateStream;

  /// 服务端事件流。
  Stream<Map<String, dynamic>> get eventStream => _events.eventStream;

  /// 会话级失败流。
  Stream<RemoteSessionAuthFailure> get failureStream =>
      _stateStore.failureStream;

  /// 已认证通过的令牌；未认证时为 null。
  String? get authenticatedToken => _authenticatedToken;

  /// 会话是否已就绪（可收发命令）。
  bool get isReady => _stateStore.state == RemoteSessionState.ready;

  /// 主通道是否已连接。
  bool get isTransportConnected =>
      _connection.transport?.state == WsConnectionState.connected;

  /// 是否已安排重连。
  bool get isReconnectScheduled => _connection.isReconnectScheduled;

  /// 更新服务地址与期望令牌；连接中或已连接时按需重建主通道。
  void configure(String baseUrl, String? token) {
    if (_baseUrl == baseUrl && _desiredToken == token) return;
    _baseUrl = baseUrl;
    _desiredToken = token;
    _authenticatedToken = null;
    _replaceTransport();
    if (_wantConnection && !_suspended) unawaited(connect());
  }

  /// 建立连接并完成认证；已就绪时直接返回 true。
  Future<bool> connect() {
    if (_disposed || _suspended || _baseUrl == null) {
      return Future.value(false);
    }
    _wantConnection = true;
    final active = _connectInFlight;
    if (active != null) return active;
    if (_stateStore.state == RemoteSessionState.ready &&
        _authenticatedToken == _desiredToken) {
      return Future.value(true);
    }

    final future = _connectInternal();
    _connectInFlight = future;
    return future.whenComplete(() {
      if (identical(_connectInFlight, future)) _connectInFlight = null;
    });
  }

  Future<bool> _connectInternal() async {
    final baseUrl = _baseUrl;
    if (baseUrl == null || _disposed || _suspended) return false;

    _connection.cancelReconnectTimer();
    _ensureTransport(baseUrl);
    final transport = _connection.transport;
    if (transport == null) return false;

    try {
      if (transport.state != WsConnectionState.connected) {
        _stateStore.setState(RemoteSessionState.connecting);
        await transport.connect();
        if (transport.state != WsConnectionState.connected) {
          throw StateError('WebSocket connection failed');
        }
      }
      if (_disposed || _suspended || transport != _connection.transport) {
        return false;
      }

      _stateStore.setState(RemoteSessionState.authenticating);
      final token = _desiredToken;
      if (token != null && token.isNotEmpty) {
        await _handshake.authenticateToken(transport, token);
        _authenticatedToken = token;
      }
      if (transport == _connection.transport && !_disposed && !_suspended) {
        if (token != null && token.isNotEmpty) {
          _connection.resetReconnectAttempt();
        }
        _stateStore.setState(
          token == null || token.isEmpty
              ? RemoteSessionState.authenticating
              : RemoteSessionState.ready,
        );
        return true;
      }
    } on RemoteSessionAuthFailure catch (error) {
      _stateStore.emitFailureObject(error);
      _stopAfterAuthFailure();
      rethrow;
    } catch (_) {
      if (!_disposed && !_suspended && _wantConnection) {
        _stateStore.setState(RemoteSessionState.disconnected);
        _scheduleReconnect();
      }
    }
    return false;
  }

  /// 用配对码换取令牌并立即认证。
  Future<Map<String, dynamic>> pair(String pin, {String? deviceName}) async {
    final transport = _requireTransport();
    _stateStore.setState(RemoteSessionState.authenticating);
    final response = await transport.request(
      AuthenticatePairingRequest(pin, deviceName: deviceName).toJson(),
    );
    final token = _handshake.extractPairingToken(response);
    _desiredToken = token;
    _authenticatedToken = token;
    _connection.resetReconnectAttempt();
    _stateStore.setState(RemoteSessionState.ready);
    return response;
  }

  /// 发送请求并等待回执。
  Future<Map<String, dynamic>> request(
    Map<String, dynamic> payload, {
    Duration? timeout,
  }) => _commands.request(payload, timeout: timeout);

  /// Typed protocol entry point. Legacy map-based calls remain available
  /// while feature controllers migrate to [RemoteRequest].
  Future<RemoteResponse> requestTyped(
    RemoteRequest request, {
    Duration? timeout,
  }) => _commands.requestTyped(request, timeout: timeout);

  /// 发送不等待回执的命令。
  void send(Map<String, dynamic> payload) => _commands.send(payload);

  /// 吊销令牌并断开会话。
  Future<Map<String, dynamic>> revokeToken() => _commands.revokeToken();

  /// 通过独立二进制输入通道发送鼠标移动帧（见
  /// [RemoteSessionInputChannel.sendBinaryFrame]）。返回 false 表示输入通道
  /// 暂不可用，调用方应降级为 JSON mouse_move。
  Future<bool> sendBinaryFrame(List<int> bytes) =>
      _inputChannel.sendBinaryFrame(bytes);

  /// 进入挂起态：释放传输并停止重连，但在恢复后仍会重连。
  void suspend() {
    if (_disposed) return;
    _suspended = true;
    _wantConnection = true;
    _connection.cancelReconnectTimer();
    _replaceTransport();
    _stateStore.setState(RemoteSessionState.suspended);
  }

  /// 退出挂起态并按需立即重连。
  void resume() {
    if (_disposed) return;
    _suspended = false;
    if (_wantConnection && _baseUrl != null) unawaited(connect());
  }

  /// 立即重连（重置退避进度）。
  void reconnectNow() {
    if (_disposed || _suspended || _baseUrl == null) return;
    _wantConnection = true;
    _connection.resetReconnectAttempt();
    _connection.cancelReconnectTimer();
    if (_stateStore.state != RemoteSessionState.ready &&
        _connectInFlight == null) {
      unawaited(connect());
    }
  }

  /// 离线/状态切换期间启用固定短间隔重连，恢复在线后关闭，加快状态感知。
  void setFastReconnect({required bool enabled}) {
    _connection.setFastReconnect(enabled: enabled);
  }

  /// 转发动态心跳间隔到当前传输通道。
  void setHealthInterval(Duration interval) {
    _connection.setHealthInterval(interval);
  }

  /// 主动断开：清空连接意图并释放传输。
  void disconnect() {
    if (_disposed) return;
    _wantConnection = false;
    _suspended = false;
    _connection.cancelReconnectTimer();
    _stateStore.setState(RemoteSessionState.closing);
    _replaceTransport();
    _stateStore.setState(RemoteSessionState.disconnected);
  }

  void _ensureTransport(String baseUrl) {
    if (_connection.transport != null && _baseUrl == baseUrl) return;
    _replaceTransport();
    _connection.establish(baseUrl);
  }

  WsClient _requireTransport() {
    final transport = _connection.transport;
    if (transport == null || transport.state != WsConnectionState.connected) {
      throw StateError('WebSocket transport is not connected');
    }
    return transport;
  }

  void _replaceTransport() {
    _connection.replace();
    _inputChannel.discard();
    _authenticatedToken = null;
  }

  /// 当前传输断开且会话仍希望保持连接时的收尾：清令牌、置状态、安排重连。
  void _handleTransportDisconnected() {
    if (!_wantConnection || _suspended) return;
    _authenticatedToken = null;
    _stateStore.setState(RemoteSessionState.disconnected);
    _scheduleReconnect();
  }

  void _scheduleReconnect() {
    final scheduled = _connection.scheduleReconnect(
      canSchedule: () =>
          !_disposed &&
          _wantConnection &&
          !_suspended &&
          _baseUrl != null &&
          !_connection.isReconnectScheduled &&
          _connectInFlight == null,
      canFire: () => !_disposed && !_suspended && _wantConnection,
    );
    if (scheduled) _stateStore.setState(RemoteSessionState.connecting);
  }

  void _stopAfterAuthFailure() {
    _wantConnection = false;
    _connection.cancelReconnectTimer();
    _replaceTransport();
    _stateStore.setState(RemoteSessionState.disconnected);
  }

  void _handleServerClosure(Map<String, dynamic> event) {
    final closure = RemoteSessionClosure.fromEvent(event);
    if (closure.stopsReconnecting) {
      _wantConnection = false;
      _connection.cancelReconnectTimer();
      _stateStore.emitFailure(closure.failureCode, closure.failureMessage);
      _replaceTransport();
      _stateStore.setState(RemoteSessionState.disconnected);
      return;
    }
    _connection.resetReconnectAttempt();
    _replaceTransport();
    _stateStore.setState(RemoteSessionState.disconnected);
    _stateStore.emitFailure(closure.failureCode, closure.failureMessage);
    _scheduleReconnect();
  }

  /// 释放会话：断连并关闭所有对外流。
  void dispose() {
    if (_disposed) return;
    disconnect();
    _disposed = true;
    _events.close();
    _stateStore.close();
  }
}
