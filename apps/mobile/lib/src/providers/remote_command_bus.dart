import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/ws_client.dart';
import '../data/repositories/remote_session_repository.dart';
import '../protocol/remote_protocol.dart';
import '../core/network/remote_command_exception.dart';
import 'dependencies.dart';

class RemoteCommandBus {
  RemoteCommandBus(this._session);

  final RemoteSessionRepository _session;

  bool get isReady => _session.isReady;

  /// 主通道推送的服务端事件流（`foreground_changed`、`launcher_catalog_ready` 等）。
  Stream<Map<String, dynamic>> get events => _session.events;

  WsConnectionState get connectionState => _session.isReady
      ? WsConnectionState.connected
      : WsConnectionState.disconnected;

  Future<bool> waitUntilReady() async {
    if (_session.isReady) return true;
    return _session.waitUntilReady();
  }

  Future<Map<String, dynamic>> request(
    Map<String, dynamic> payload, {
    Duration? timeout,
  }) async {
    if (!_session.isReady) {
      throw StateError('无法连接到 PC，请检查网络和连接设置后重试');
    }
    final response = await _session.requestRaw(payload, timeout: timeout);
    if (response['type'] == 'error') {
      final data = response['data'];
      final error = data is Map<String, dynamic> ? data : const <String, dynamic>{};
      throw RemoteCommandException(
        error['code'] as String? ?? 'REMOTE_COMMAND_FAILED',
        error['message'] as String? ?? 'PC 未能完成该操作',
      );
    }
    return response;
  }

  void send(Map<String, dynamic> payload) {
    if (!_session.isReady) throw StateError('无法连接到 PC，请检查网络和连接设置后重试');
    _session.send(RemoteCommandRequest(payload['type'] as String, data: payload));
  }

  Future<bool> sendBinary(List<int> bytes) =>
      _session.sendBinary(bytes);
}

final remoteCommandBusProvider = Provider<RemoteCommandBus>((ref) {
  return RemoteCommandBus(ref.watch(remoteSessionRepositoryProvider));
});
