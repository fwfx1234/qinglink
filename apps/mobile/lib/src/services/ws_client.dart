import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

enum WsConnectionState { disconnected, connecting, connected, reconnecting }

class WsClient {
  final String baseUrl;
  final String path;
  String token;
  final void Function(String)? onLog;
  final bool autoReconnect;
  final bool enableHealthChecks;

  WebSocketChannel? _channel;
  StreamSubscription? _subscription;
  Timer? _reconnectTimer;
  Timer? _healthTimer;
  Timer? _stableConnectionTimer;

  final _eventController = StreamController<Map<String, dynamic>>.broadcast();
  final _stateController = StreamController<WsConnectionState>.broadcast();
  final Map<String, _PendingRequest> _pendingRequests = {};

  WsConnectionState _state = WsConnectionState.disconnected;
  int _reconnectAttempt = 0;
  int _connectionGeneration = 0;
  int _nextRequestId = 0;
  bool _disposed = false;
  bool _manualDisconnect = false;
  bool _connectInFlight = false;
  bool _healthRequestInFlight = false;

  static const _maxBackoffSeconds = 30;
  static const _defaultRequestTimeout = Duration(seconds: 10);
  static const _defaultHealthInterval = Duration(seconds: 10);
  static const _defaultHealthTimeout = Duration(seconds: 5);
  static const _defaultStableConnectionWindow = Duration(seconds: 10);
  static const _initialReconnectDelay = Duration(milliseconds: 500);
  Duration _healthInterval;
  final Duration _healthTimeout;
  final Duration _stableConnectionWindow;

  Stream<Map<String, dynamic>> get eventStream => _eventController.stream;
  Stream<WsConnectionState> get stateStream => _stateController.stream;
  WsConnectionState get state => _state;

  WsClient({
    required this.baseUrl,
    required this.token,
    this.path = '/api/v2/events',
    this.onLog,
    this.autoReconnect = true,
    this.enableHealthChecks = true,
    Duration? healthInterval,
    Duration? healthTimeout,
    Duration? stableConnectionWindow,
  }) : _healthInterval = healthInterval ?? _defaultHealthInterval,
       _healthTimeout = healthTimeout ?? _defaultHealthTimeout,
       _stableConnectionWindow =
           stableConnectionWindow ?? _defaultStableConnectionWindow;

  void _log(String msg) {
    onLog?.call(msg);
  }

  Future<void> connect() async {
    if (_disposed || _connectInFlight) return;
    if (_state == WsConnectionState.connected) return;

    _manualDisconnect = false;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _connectInFlight = true;
    _setState(WsConnectionState.connecting);

    final wsScheme = baseUrl.startsWith('https') ? 'wss' : 'ws';
    final host = baseUrl.replaceFirst(RegExp(r'^https?://'), '');
    final normalizedPath = path.startsWith('/') ? path : '/$path';
    final uri = Uri.parse('$wsScheme://$host$normalizedPath');

    _log('WS connecting: $uri');

    _closeTransport();
    final generation = _connectionGeneration;

    try {
      final channel = WebSocketChannel.connect(uri);
      _channel = channel;
      _subscription = channel.stream.listen(
        (message) => _onMessage(message, generation),
        onError: (Object error, StackTrace stackTrace) {
          _onError(error, generation);
        },
        onDone: () => _onDone(generation),
        cancelOnError: false,
      );

      await channel.ready.timeout(const Duration(seconds: 10));
      if (_disposed || generation != _connectionGeneration) return;

      _setState(WsConnectionState.connected);
      if (enableHealthChecks) _startHealthChecks(generation);
      _stableConnectionTimer?.cancel();
      _stableConnectionTimer = Timer(_stableConnectionWindow, () {
        if (!_disposed && generation == _connectionGeneration) {
          _reconnectAttempt = 0;
        }
      });
      _log('WS connected');
    } catch (error) {
      _log('WS connect failed: $error');
      if (generation == _connectionGeneration && !_disposed) {
        _handleTransportFailure(error, generation);
      }
    } finally {
      _connectInFlight = false;
    }
  }

  void _onMessage(dynamic message, int generation) {
    if (_disposed || generation != _connectionGeneration) return;
    if (message is! String) {
      _log('WS ignored non-text message');
      return;
    }

    try {
      final data = jsonDecode(message) as Map<String, dynamic>;
      final type = data['type'] as String?;

      if (type == null) {
        _log('WS message missing type field');
        return;
      }

      if (_isEvent(type)) {
        _eventController.add(data);
      } else {
        _handleResponse(data);
      }
    } catch (error) {
      _log('WS message parse error: $error');
    }
  }

  bool _isEvent(String type) {
    switch (type) {
      case 'foreground_changed':
      case 'process_started':
      case 'process_ended':
      case 'system_suspend':
      case 'system_resume':
      case 'system_shutdown':
      case 'system_sleep':
      case 'session_closed':
        return true;
      default:
        return false;
    }
  }

  void _handleResponse(Map<String, dynamic> data) {
    final requestId = data['request_id'] as String?;
    if (requestId != null) {
      final pending = _pendingRequests.remove(requestId);
      if (pending != null && !pending.completer.isCompleted) {
        pending.completer.complete(data);
      }
    }
  }

  Future<Map<String, dynamic>> request(
    Map<String, dynamic> payload, {
    Duration? timeout,
  }) async {
    if (_state != WsConnectionState.connected || _channel == null) {
      throw StateError('WebSocket not connected (state: $_state)');
    }

    final completer = Completer<Map<String, dynamic>>();
    final requestId = payload['request_id'] as String? ?? _newRequestId();
    final requestPayload = Map<String, dynamic>.from(payload)
      ..['request_id'] = requestId;
    final pending = _PendingRequest(completer);
    _pendingRequests[requestId] = pending;

    try {
      _channel!.sink.add(jsonEncode(requestPayload));
    } catch (error) {
      _pendingRequests.remove(requestId);
      rethrow;
    }

    final effectiveTimeout = timeout ?? _defaultRequestTimeout;
    final sentAt = DateTime.now();
    return completer.future.timeout(
      effectiveTimeout,
      onTimeout: () {
        _pendingRequests.remove(requestId);
        _log(
          'WS request timeout after ${DateTime.now().difference(sentAt).inMilliseconds}ms '
          '(limit ${effectiveTimeout.inMilliseconds}ms): ${payload['type']} request_id=$requestId',
        );
        throw TimeoutException('Request timeout: ${payload['type']}');
      },
    );
  }

  /// Sends a fire-and-forget message, used for high-frequency input events.
  void send(Map<String, dynamic> payload) {
    if (_state != WsConnectionState.connected || _channel == null) {
      throw StateError('WebSocket not connected (state: $_state)');
    }
    _channel!.sink.add(jsonEncode(payload));
  }

  /// Sends a binary frame without creating a request that expects a reply.
  void sendBinary(List<int> bytes) {
    if (_state != WsConnectionState.connected || _channel == null) {
      throw StateError('WebSocket not connected (state: $_state)');
    }
    _channel!.sink.add(bytes);
  }

  String _newRequestId() {
    _nextRequestId = (_nextRequestId + 1) & 0x7fffffff;
    return '${DateTime.now().microsecondsSinceEpoch.toRadixString(36)}-$_nextRequestId';
  }

  void _onError(Object error, int generation) {
    if (_disposed || generation != _connectionGeneration) return;
    _log('WS error: $error');
    _handleTransportFailure(error, generation);
  }

  void _onDone(int generation) {
    if (_disposed || generation != _connectionGeneration) return;
    _log('WS closed');
    _handleTransportFailure(StateError('WebSocket closed'), generation);
  }

  void _handleTransportFailure(Object error, int generation) {
    if (_disposed || generation != _connectionGeneration) return;
    _closeTransport();
    _failPending(error);
    _setState(WsConnectionState.disconnected);
    if (autoReconnect && !_manualDisconnect) _scheduleReconnect();
  }

  void _scheduleReconnect() {
    if (_disposed || _manualDisconnect) return;
    if (_state == WsConnectionState.reconnecting || _reconnectTimer != null) {
      return;
    }

    _setState(WsConnectionState.reconnecting);
    final delay = _computeReconnectDelay();
    _reconnectAttempt++;
    _log(
      'WS reconnect in ${delay.inMilliseconds}ms (attempt $_reconnectAttempt)',
    );

    _reconnectTimer = Timer(delay, () {
      _reconnectTimer = null;
      if (!_disposed && !_manualDisconnect) {
        unawaited(connect());
      }
    });
  }

  Duration _computeReconnectDelay() {
    if (_reconnectAttempt == 0) return _initialReconnectDelay;
    final secs = (1 << (_reconnectAttempt - 1)).clamp(1, _maxBackoffSeconds);
    return Duration(seconds: secs);
  }

  void _startHealthChecks(int generation) {
    _healthTimer?.cancel();
    _healthTimer = Timer.periodic(_healthInterval, (_) {
      unawaited(_checkHealth(generation));
    });
  }

  /// 动态调整健康检查（心跳）间隔；已连接且启用健康检查时立即生效。
  void setHealthInterval(Duration interval) {
    _healthInterval = interval;
    if (_state == WsConnectionState.connected && enableHealthChecks) {
      _startHealthChecks(_connectionGeneration);
    }
  }

  Future<void> _checkHealth(int generation) async {
    if (_disposed || generation != _connectionGeneration) return;
    if (_state != WsConnectionState.connected || _healthRequestInFlight) return;

    _healthRequestInFlight = true;
    try {
      final response = await request({'type': 'ping'}, timeout: _healthTimeout);
      if (response['type'] != 'pong') {
        throw StateError('Invalid WebSocket health response');
      }
    } catch (error) {
      if (generation == _connectionGeneration) {
        _log('WS health check failed: $error');
        _handleTransportFailure(error, generation);
      }
    } finally {
      _healthRequestInFlight = false;
    }
  }

  void _closeTransport() {
    _healthTimer?.cancel();
    _healthTimer = null;
    _healthRequestInFlight = false;
    _stableConnectionTimer?.cancel();
    _stableConnectionTimer = null;

    _connectionGeneration++;
    final subscription = _subscription;
    _subscription = null;
    if (subscription != null) unawaited(subscription.cancel());

    final channel = _channel;
    _channel = null;
    if (channel != null) unawaited(channel.sink.close());
  }

  void disconnect() {
    if (_disposed) return;
    _manualDisconnect = true;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _reconnectAttempt = 0;
    _closeTransport();
    _failPending(StateError('WebSocket disconnected'));
    _setState(WsConnectionState.disconnected);
  }

  void reconnectNow() {
    if (_disposed) return;
    _manualDisconnect = false;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _reconnectAttempt = 0;
    if (_state != WsConnectionState.connected) unawaited(connect());
  }

  void _failPending(Object error) {
    for (final pending in _pendingRequests.values) {
      if (!pending.completer.isCompleted) {
        pending.completer.completeError(error);
      }
    }
    _pendingRequests.clear();
  }

  void _setState(WsConnectionState state) {
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
    _failPending(StateError('WsClient disposed'));
  }
}

class _PendingRequest {
  final Completer<Map<String, dynamic>> completer;

  _PendingRequest(this.completer);
}
