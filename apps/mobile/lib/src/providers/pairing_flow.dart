import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/errors/pairing_failures.dart';
import '../data/repositories/pairing_repository.dart';
import '../models/auth.dart';
import '../services/qinglink_pairing.dart';
import '../services/remote_session.dart';
import '../utils/wake_on_lan.dart';
import 'auth_state_writer.dart';
import 'dependencies.dart';
import 'discovery_coordinator.dart';
import 'session_connector.dart';
import 'settings_provider.dart';

/// 配对请求的发起、确认与取消。
///
/// 只负责 HTTP 配对状态机与配对结果的持久化；配对成功后的控制连接由
/// [SessionConnector] 建立。
class PairingFlow {
  PairingFlow(
    this._ref,
    this._state,
    this._discovery,
    this._session, {
    PairingRepository? pairing,
  }) : _pairing = pairing ?? _ref.read(pairingRepositoryProvider);

  final Ref _ref;
  final AuthStateWriter _state;
  final DiscoveryCoordinator _discovery;
  final SessionConnector _session;
  final PairingRepository _pairing;
  String? _requestId;
  String? _deviceId;

  /// 发起配对请求；已有进行中的请求时直接返回。
  Future<void> begin() async {
    if (_requestId != null) return;
    await _ref.read(settingsProvider.notifier).loaded;
    var config = _ref.read(settingsProvider);
    if (config.baseUrl == null) {
      await _discovery.refresh();
      config = _ref.read(settingsProvider);
    }
    final baseUrl = config.baseUrl;
    if (baseUrl == null) {
      throw const QingLinkPairingException('NO_PC', '请先选择 QingLink 电脑');
    }
    if (kDebugMode) {
      debugPrint('[QingLink][Pairing] request start base=$baseUrl');
    }
    final prefs = await _ref.read(settingsProvider.notifier).preferences;
    final deviceId =
        prefs.getString('qinglink_device_id') ??
        'mobile-${DateTime.now().millisecondsSinceEpoch}-${Random().nextInt(1 << 32)}';
    await prefs.setString('qinglink_device_id', deviceId);
    _deviceId = deviceId;
    try {
      _requestId = await _pairing.request(
        baseUrl: baseUrl,
        deviceId: deviceId,
        deviceName: '轻联手机',
      );
    } on QingLinkPairingException catch (failure) {
      // The request endpoint now authorizes nothing by itself: it fails closed
      // while the desktop agent is absent, locked or already has a pending
      // request. Surface the mapped text and keep the throw for the callers.
      _state.update(
        _state.state.copyWith(
          status: AuthStatus.unauthenticated,
          errorMessage: pairingFailureText(failure.code, failure.message),
        ),
      );
      rethrow;
    }
    if (kDebugMode) {
      debugPrint('[QingLink][Pairing] request created id=$_requestId');
    }
    _state.update(
      _state.state.copyWith(
        // Pairing requests use HTTP and remain valid even when the optional
        // WS session handshake is unavailable.
        connection: AppConnectionState.connected,
        status: AuthStatus.unauthenticated,
        clearError: true,
      ),
    );
  }

  /// 提交 PIN 完成配对，并使用新令牌建立控制连接。
  Future<void> confirm(String pin) async {
    final config = _ref.read(settingsProvider);
    final requestId = _requestId;
    final deviceId = _deviceId;
    if (config.baseUrl == null || requestId == null || deviceId == null) {
      _state.update(
        _state.state.copyWith(
          status: AuthStatus.unauthenticated,
          errorMessage: '请先发起配对请求',
        ),
      );
      return;
    }

    _state.update(
      _state.state.copyWith(
        status: AuthStatus.authenticating,
        clearError: true,
      ),
    );
    try {
      final response = await _pairing.confirm(
        baseUrl: config.baseUrl!,
        requestId: requestId,
        pin: pin,
        deviceId: deviceId,
        deviceName: '轻联手机',
      );
      final token = await _persistResponse(response, config);
      await _session.connectAfterPairing(config.baseUrl!, token);
      _requestId = null;
    } on RemoteSessionAuthFailure catch (failure) {
      if (kDebugMode) {
        debugPrint('[QingLink][Pairing] auth failure code=${failure.code}');
      }
      _fail(pairingFailureText(failure.code, failure.message));
    } on QingLinkPairingException catch (failure) {
      if (kDebugMode) {
        debugPrint('[QingLink][Pairing] request failure code=${failure.code}');
      }
      _fail(pairingFailureText(failure.code, failure.message));
    } catch (_) {
      _fail('配对超时，请重试');
    }
  }

  /// 取消进行中的配对请求。
  Future<void> cancel() async {
    final requestId = _requestId;
    final baseUrl = _ref.read(settingsProvider).baseUrl;
    _requestId = null;
    if (requestId != null && baseUrl != null) {
      try {
        await _pairing.cancel(baseUrl: baseUrl, requestId: requestId);
      } catch (error, stackTrace) {
        if (kDebugMode) {
          debugPrint('[QingLink] cancel pairing failed: $error\n$stackTrace');
        }
      }
    }
    if (!_state.state.isAuthenticated) {
      _state.update(_state.state.copyWith(status: AuthStatus.unauthenticated));
    }
  }

  /// 持久化配对响应，返回校验过的令牌。
  Future<String> _persistResponse(
    Map<String, dynamic> response,
    ConnectionConfig config,
  ) async {
    final token = response['token'] as String?;
    final pcId = response['pc_id'] as String?;
    final pcName = response['pc_name'] as String?;
    final mac = response['mac_address'] as String?;
    final wakeNetworks =
        (response['wake_networks'] as List<dynamic>? ?? const [])
            .whereType<Map>()
            .map((e) => WakeNetwork.fromJson(Map<String, dynamic>.from(e)))
            .toList(growable: false);
    if (token == null || token.isEmpty) {
      throw const RemoteSessionAuthFailure(
        'INVALID_AUTH_RESPONSE',
        '配对响应缺少 Token',
      );
    }
    final settings = _ref.read(settingsProvider.notifier);
    await settings.saveToken(token);
    if (pcId != null && pcId.isNotEmpty) {
      await settings.savePcIdentity(pcId);
    }
    final pairedPcId = pcId ?? config.pcId;
    if (pairedPcId != null &&
        pairedPcId.isNotEmpty &&
        config.pcIpAddress != null &&
        config.pcIpAddress!.isNotEmpty) {
      await settings.applyDiscoveredEndpoint(
        pcId: pairedPcId,
        pcName: pcName,
        address: config.pcIpAddress!,
        port: config.pcPort,
      );
    }
    if (mac != null && mac.isNotEmpty) {
      await settings.updateMacAddress(mac);
    }
    if (wakeNetworks.isNotEmpty) {
      await settings.updateWakeNetworks(wakeNetworks);
    }
    return token;
  }

  /// 回到未认证状态并给出失败提示。
  void _fail(String message) {
    _state.update(
      _state.state.copyWith(
        status: AuthStatus.unauthenticated,
        errorMessage: message,
      ),
    );
  }
}
