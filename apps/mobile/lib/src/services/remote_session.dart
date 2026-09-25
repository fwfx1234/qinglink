import 'dart:async';
import 'package:flutter/foundation.dart';

import 'ws_client.dart';

enum RemoteSessionState {
  disconnected,
  connecting,
  authenticating,
  ready,
  suspended,
  closing,
}

class RemoteSessionAuthFailure implements Exception {
  final String code;
  final String message;

  const RemoteSessionAuthFailure(this.code, this.message);

  @override
  String toString() => '$code: $message';
}

typedef RemoteSessionClientFactory =
    WsClient Function({
      required String baseUrl,
      required String path,
      required String token,
      void Function(String)? onLog,
    });

/// Owns the WebSocket transports used by the app.
///
/// 主通道（/api/v2/events）承载认证、事件与请求；二进制输入通道
/// （/api/v2/input）用于高频鼠标位移，避免慢命令阻塞输入。
/// 输入通道按需懒连接，不影响仅浏览状态的会话。
class RemoteSession {
  final RemoteSessionClientFactory _clientFactory;

  WsClient? _transport;

  /// 二进制输入通道（/api/v2/input），懒连接，仅鼠标控制时使用。
  WsClient? _inputTransport;
  StreamSubscription? _transportStateSubscription;
  StreamSubscription? _transportEventSubscription;
  final _stateController = StreamController<RemoteSessionState>.broadcast();
  final _eventController = StreamController<Map<String, dynamic>>.broadcast();
  final _failureController =
      StreamController<RemoteSessionAuthFailure>.broadcast();

  RemoteSessionState _state = RemoteSessionState.disconnected;
  String? _baseUrl;
  String? _desiredToken;
  String? _authenticatedToken;
  Future<bool>? _connectInFlight;
  Timer? _reconnectTimer;
  int _reconnectAttempt = 0;
  int _generation = 0;
  bool _wantConnection = false;
  bool _suspended = false;
  bool _disposed = false;

  static const _initialReconnectDelay = Duration(milliseconds: 500);
  static const _maxReconnectDelaySeconds = 30;
  static const _fastReconnectInterval = Duration(seconds: 3);
  Duration? _fastReconnectIntervalOverride;

  RemoteSession({RemoteSessionClientFactory? clientFactory})
    : _clientFactory = clientFactory ?? _defaultClientFactory;

  static WsClient _defaultClientFactory({
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

  RemoteSessionState get state => _state;
  Stream<RemoteSessionState> get stateStream => _stateController.stream;
  Stream<Map<String, dynamic>> get eventStream => _eventController.stream;
  Stream<RemoteSessionAuthFailure> get failureStream =>
      _failureController.stream;
  String? get authenticatedToken => _authenticatedToken;
  bool get isReady => _state == RemoteSessionState.ready;
  bool get isTransportConnected =>
      _transport?.state == WsConnectionState.connected;
  bool get isReconnectScheduled => _reconnectTimer != null;

  void configure(String baseUrl, String? token) {
    if (_baseUrl == baseUrl && _desiredToken == token) return;
    _baseUrl = baseUrl;
    _desiredToken = token;
    _authenticatedToken = null;
    _replaceTransport();
    if (_wantConnection && !_suspended) unawaited(connect());
  }

  Future<bool> connect() {
    if (_disposed || _suspended || _baseUrl == null) {
      return Future.value(false);
    }
    _wantConnection = true;
    final active = _connectInFlight;
    if (active != null) return active;
    if (_state == RemoteSessionState.ready &&
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

    _cancelReconnectTimer();
    _ensureTransport(baseUrl);
    final transport = _transport;
    if (transport == null) return false;

    try {
      if (transport.state != WsConnectionState.connected) {
        _setState(RemoteSessionState.connecting);
        await transport.connect();
        if (transport.state != WsConnectionState.connected) {
          throw StateError('WebSocket connection failed');
        }
      }
      if (_disposed || _suspended || transport != _transport) return false;

      _setState(RemoteSessionState.authenticating);
      final token = _desiredToken;
      if (token != null && token.isNotEmpty) {
        await _authenticateToken(token, transport);
      }
      if (transport == _transport && !_disposed && !_suspended) {
        if (token != null && token.isNotEmpty) _reconnectAttempt = 0;
        _setState(
          token == null || token.isEmpty
              ? RemoteSessionState.authenticating
              : RemoteSessionState.ready,
        );
        return true;
      }
    } on RemoteSessionAuthFailure catch (error) {
      if (!_failureController.isClosed) _failureController.add(error);
      _stopAfterAuthFailure();
      rethrow;
    } catch (_) {
      if (!_disposed && !_suspended && _wantConnection) {
        _setState(RemoteSessionState.disconnected);
        _scheduleReconnect();
      }
    }
    return false;
  }

  Future<Map<String, dynamic>> pair(String pin, {String? deviceName}) async {
    final transport = _requireTransport();
    _setState(RemoteSessionState.authenticating);
    final response = await transport.request({
      'type': 'authenticate',
      'method': 'pair',
      'pin': pin,
      if (deviceName != null && deviceName.isNotEmpty)
        'device_name': deviceName,
    });
    _ensureAuthResponse(response);
    final token =
        (response['data'] as Map<String, dynamic>?)?['token'] as String?;
    if (token == null || token.isEmpty) {
      throw const RemoteSessionAuthFailure(
        'INVALID_AUTH_RESPONSE',
        'Authentication response did not contain a token',
      );
    }
    _desiredToken = token;
    _authenticatedToken = token;
    _reconnectAttempt = 0;
    _setState(RemoteSessionState.ready);
    return response;
  }

  Future<void> _authenticateToken(String token, WsClient transport) async {
    final response = await transport.request({
      'type': 'authenticate',
      'method': 'token',
      'token': token,
    });
    _ensureAuthResponse(response);
    _authenticatedToken = token;
  }

  void _ensureAuthResponse(Map<String, dynamic> response) {
    if (response['type'] == 'authenticated') return;
    final data = response['data'] as Map<String, dynamic>?;
    final code = data?['code'] as String? ?? 'AUTH_FAILED';
    final message = data?['message'] as String? ?? 'Authentication failed';
    throw RemoteSessionAuthFailure(code, message);
  }

  Future<Map<String, dynamic>> request(
    Map<String, dynamic> payload, {
    Duration? timeout,
  }) async {
    if (!isReady) {
      throw StateError('Remote session is not ready (state: $_state)');
    }
    return _requireTransport().request(payload, timeout: timeout);
  }

  void send(Map<String, dynamic> payload) {
    if (!isReady) {
      throw StateError('Remote session is not ready (state: $_state)');
    }
    _requireTransport().send(payload);
  }

  Future<Map<String, dynamic>> revokeToken() async {
    final response = await request({'type': 'revoke_token'});
    disconnect();
    return response;
  }

  /// 通过独立二进制输入通道发送鼠标移动帧。
  ///
  /// 输入通道（/api/v2/input）懒连接：触控板首次产生位移时才建立，
  /// 避免为不使用鼠标控制的会话保留第二个连接。返回 false 表示输入通道
  /// 暂不可用，调用方应降级为 JSON mouse_move。
  Future<bool> sendBinaryFrame(List<int> bytes) async {
    if (_disposed || !isReady || _authenticatedToken == null) {
      return false;
    }
    try {
      final input = await _ensureInputTransport();
      if (input == null) return false;
      input.sendBinary(bytes);
      return true;
    } catch (_) {
      _discardInputTransport();
      return false;
    }
  }

  Future<WsClient?> _ensureInputTransport() async {
    final existing = _inputTransport;
    if (existing != null && existing.state == WsConnectionState.connected) {
      return existing;
    }
    _discardInputTransport();
    final baseUrl = _baseUrl;
    final token = _authenticatedToken;
    if (baseUrl == null || token == null || token.isEmpty) return null;
    final generation = _generation;
    final input = _clientFactory(
      baseUrl: baseUrl,
      path: '/api/v2/input',
      token: token,
      onLog: (message) {
        if (kDebugMode) debugPrint('[QingLink][InputWS] $message');
      },
    );
    _inputTransport = input;
    try {
      await input.connect();
      if (_disposed || generation != _generation || input != _inputTransport) {
        return null;
      }
      if (input.state != WsConnectionState.connected) return null;
      final response = await input.request({
        'type': 'authenticate',
        'method': 'token',
        'token': token,
      }, timeout: const Duration(seconds: 5));
      if (_disposed || generation != _generation || input != _inputTransport) {
        return null;
      }
      return response['type'] == 'authenticated' ? input : null;
    } catch (_) {
      return null;
    }
  }

  void _discardInputTransport() {
    final input = _inputTransport;
    _inputTransport = null;
    input?.dispose();
  }

  void suspend() {
    if (_disposed) return;
    _suspended = true;
    _wantConnection = true;
    _cancelReconnectTimer();
    _replaceTransport();
    _setState(RemoteSessionState.suspended);
  }

  void resume() {
    if (_disposed) return;
    _suspended = false;
    if (_wantConnection && _baseUrl != null) unawaited(connect());
  }

  void reconnectNow() {
    if (_disposed || _suspended || _baseUrl == null) return;
    _wantConnection = true;
    _reconnectAttempt = 0;
    _cancelReconnectTimer();
    if (_state != RemoteSessionState.ready && _connectInFlight == null) {
      unawaited(connect());
    }
  }

  /// 离线/状态切换期间启用固定短间隔重连，恢复在线后关闭，加快状态感知。
  void setFastReconnect({required bool enabled}) {
    _fastReconnectIntervalOverride = enabled ? _fastReconnectInterval : null;
  }

  /// 转发动态心跳间隔到当前传输通道。
  void setHealthInterval(Duration interval) {
    _transport?.setHealthInterval(interval);
  }

  void disconnect() {
    if (_disposed) return;
    _wantConnection = false;
    _suspended = false;
    _cancelReconnectTimer();
    _setState(RemoteSessionState.closing);
    _replaceTransport();
    _setState(RemoteSessionState.disconnected);
  }

  void _ensureTransport(String baseUrl) {
    if (_transport != null && _baseUrl == baseUrl) return;
    _replaceTransport();
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
      if (_disposed || generation != _generation || transport != _transport) {
        return;
      }
      if (state == WsConnectionState.disconnected &&
          _wantConnection &&
          !_suspended) {
        _authenticatedToken = null;
        _setState(RemoteSessionState.disconnected);
        _scheduleReconnect();
      }
    });
    _transportEventSubscription = transport.eventStream.listen((event) {
      if (!_disposed && generation == _generation && transport == _transport) {
        if (event['type'] == 'session_closed') {
          _handleServerClosure(event);
        }
        _eventController.add(event);
      }
    });
  }

  WsClient _requireTransport() {
    final transport = _transport;
    if (transport == null || transport.state != WsConnectionState.connected) {
      throw StateError('WebSocket transport is not connected');
    }
    return transport;
  }

  void _replaceTransport() {
    _generation++;
    _transportStateSubscription?.cancel();
    _transportStateSubscription = null;
    _transportEventSubscription?.cancel();
    _transportEventSubscription = null;
    _transport?.dispose();
    _transport = null;
    _discardInputTransport();
    _authenticatedToken = null;
  }

  void _scheduleReconnect() {
    if (_disposed || !_wantConnection || _suspended || _baseUrl == null) return;
    if (_reconnectTimer != null || _connectInFlight != null) return;
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
      if (!_disposed && !_suspended && _wantConnection) unawaited(connect());
    });
    _setState(RemoteSessionState.connecting);
  }

  void _cancelReconnectTimer() {
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
  }

  void _stopAfterAuthFailure() {
    _wantConnection = false;
    _cancelReconnectTimer();
    _replaceTransport();
    _setState(RemoteSessionState.disconnected);
  }

  void _handleServerClosure(Map<String, dynamic> event) {
    final data = event['data'] as Map<String, dynamic>?;
    final code = data?['code'] as String? ?? 'SESSION_CLOSED';
    _wantConnection = false;
    _cancelReconnectTimer();
    if (!_failureController.isClosed) {
      _failureController.add(
        RemoteSessionAuthFailure(
          code == 'REVOKED' ? 'INVALID_TOKEN' : code,
          code == 'REVOKED' ? 'Token has been revoked' : 'Session was replaced',
        ),
      );
    }
    _replaceTransport();
    _setState(RemoteSessionState.disconnected);
  }

  void _setState(RemoteSessionState state) {
    if (_state == state) return;
    _state = state;
    if (!_stateController.isClosed) _stateController.add(state);
  }

  void dispose() {
    if (_disposed) return;
    disconnect();
    _disposed = true;
    _eventController.close();
    _stateController.close();
    _failureController.close();
  }
}
