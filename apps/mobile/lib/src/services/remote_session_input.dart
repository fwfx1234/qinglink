import 'package:flutter/foundation.dart';

import 'remote_session_types.dart';
import 'ws_client.dart';

/// 二进制输入通道（/api/v2/input）的懒连接管理。
///
/// 输入通道按需建立：触控板首次产生位移时才连接，避免为不使用鼠标控制的
/// 会话保留第二个连接。会话状态、地址与令牌通过回调读取，不在这里维护。
class RemoteSessionInputChannel {
  /// [baseUrl] 返回当前服务地址；[authenticatedToken] 返回已认证令牌；
  /// [generation] 返回主通道代际号（用于使在途建连失效）；
  /// [isDisposed] 返回会话是否已释放；[isReady] 返回会话是否已就绪。
  RemoteSessionInputChannel({
    required RemoteSessionClientFactory clientFactory,
    required String? Function() baseUrl,
    required String? Function() authenticatedToken,
    required int Function() generation,
    required bool Function() isDisposed,
    required bool Function() isReady,
  }) {
    _clientFactory = clientFactory;
    _baseUrl = baseUrl;
    _authenticatedToken = authenticatedToken;
    _generation = generation;
    _isDisposed = isDisposed;
    _isReady = isReady;
  }

  late final RemoteSessionClientFactory _clientFactory;
  late final String? Function() _baseUrl;
  late final String? Function() _authenticatedToken;
  late final int Function() _generation;
  late final bool Function() _isDisposed;
  late final bool Function() _isReady;

  /// 二进制输入通道（/api/v2/input），懒连接，仅鼠标控制时使用。
  WsClient? _transport;

  /// 通过独立二进制输入通道发送鼠标移动帧。
  ///
  /// 输入通道（/api/v2/input）懒连接：触控板首次产生位移时才建立，
  /// 避免为不使用鼠标控制的会话保留第二个连接。返回 false 表示输入通道
  /// 暂不可用，调用方应降级为 JSON mouse_move。
  Future<bool> sendBinaryFrame(List<int> bytes) async {
    if (_isDisposed() || !_isReady() || _authenticatedToken() == null) {
      return false;
    }
    try {
      final input = await _ensureTransport();
      if (input == null) return false;
      input.sendBinary(bytes);
      return true;
    } catch (_) {
      discard();
      return false;
    }
  }

  Future<WsClient?> _ensureTransport() async {
    final existing = _transport;
    if (existing != null && existing.state == WsConnectionState.connected) {
      return existing;
    }
    discard();
    final baseUrl = _baseUrl();
    final token = _authenticatedToken();
    if (baseUrl == null || token == null || token.isEmpty) return null;
    final generation = _generation();
    final input = _clientFactory(
      baseUrl: baseUrl,
      path: '/api/v2/input',
      token: token,
      onLog: (message) {
        if (kDebugMode) debugPrint('[QingLink][InputWS] $message');
      },
    );
    _transport = input;
    try {
      await input.connect();
      if (_isDisposed() || generation != _generation() || input != _transport) {
        return null;
      }
      if (input.state != WsConnectionState.connected) return null;
      final response = await input.request({
        'type': 'authenticate',
        'method': 'token',
        'token': token,
      }, timeout: const Duration(seconds: 5));
      if (_isDisposed() || generation != _generation() || input != _transport) {
        return null;
      }
      return response['type'] == 'authenticated' ? input : null;
    } catch (_) {
      return null;
    }
  }

  /// 丢弃当前输入通道（主通道被替换或发送失败降级时调用）。
  void discard() {
    final input = _transport;
    _transport = null;
    input?.dispose();
  }
}
