import 'dart:async';

import '../../protocol/remote_protocol.dart';
import '../../services/remote_session.dart';

abstract interface class RemoteSessionRepository {
  Stream<RemoteSessionState> get states;
  Stream<Map<String, dynamic>> get events;
  Stream<RemoteSessionAuthFailure> get authenticationFailures;
  bool get isReady;
  bool get isTransportConnected;
  bool get isReconnectScheduled;
  String? get authenticatedToken;

  void configure(String baseUrl, String? token);
  Future<bool> connect();
  Future<Map<String, dynamic>> authenticateWithPairing(String pin);
  Future<bool> waitUntilReady({Duration timeout});
  void reconnectNow();
  Future<void> revokeCurrentToken();
  Future<RemoteResponse> request(RemoteRequest request);
  Future<Map<String, dynamic>> requestRaw(
    Map<String, dynamic> request, {
    Duration? timeout,
  });
  void send(RemoteRequest request);
  Future<bool> sendBinary(List<int> bytes);
  void disconnect();
}

final class QingLinkRemoteSessionRepository implements RemoteSessionRepository {
  QingLinkRemoteSessionRepository({RemoteSession? session})
    : _session = session ?? RemoteSession();

  final RemoteSession _session;

  @override
  Stream<RemoteSessionState> get states => _session.stateStream;

  @override
  Stream<Map<String, dynamic>> get events => _session.eventStream;

  @override
  Stream<RemoteSessionAuthFailure> get authenticationFailures =>
      _session.failureStream;

  @override
  bool get isReady => _session.isReady;
  @override
  bool get isTransportConnected => _session.isTransportConnected;
  @override
  bool get isReconnectScheduled => _session.isReconnectScheduled;
  @override
  String? get authenticatedToken => _session.authenticatedToken;

  @override
  void configure(String baseUrl, String? token) => _session.configure(baseUrl, token);

  @override
  Future<bool> connect() => _session.connect();

  @override
  Future<Map<String, dynamic>> authenticateWithPairing(String pin) =>
      _session.pair(pin);

  @override
  Future<bool> waitUntilReady({
    Duration timeout = const Duration(seconds: 12),
  }) async {
    if (_session.isReady) return true;
    try {
      await states
          .firstWhere((state) => state == RemoteSessionState.ready)
          .timeout(timeout);
    } on StateError {
      return _session.isReady;
    } on TimeoutException {
      return _session.isReady;
    }
    return _session.isReady;
  }

  @override
  void reconnectNow() => _session.reconnectNow();

  @override
  Future<void> revokeCurrentToken() => _session.revokeToken();

  @override
  Future<RemoteResponse> request(RemoteRequest request) async {
    return _session.requestTyped(request);
  }

  @override
  Future<Map<String, dynamic>> requestRaw(
    Map<String, dynamic> request, {
    Duration? timeout,
  }) => _session.request(request, timeout: timeout);

  @override
  void send(RemoteRequest request) => _session.send(request.toJson());

  @override
  Future<bool> sendBinary(List<int> bytes) => _session.sendBinaryFrame(bytes);

  @override
  void disconnect() => _session.disconnect();

  void dispose() => _session.dispose();
}
