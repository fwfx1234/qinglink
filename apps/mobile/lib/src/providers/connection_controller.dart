import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/errors/pairing_failures.dart';
import '../data/repositories/device_discovery_repository.dart';
import '../data/repositories/pairing_repository.dart';
import '../models/auth.dart';
import '../services/qinglink_pairing.dart';
import 'auth_state_writer.dart';
import 'discovery_coordinator.dart';
import 'pairing_flow.dart';
import 'session_connector.dart';
import 'settings_provider.dart';

/// Coordinates discovery, pairing and authenticated session transitions.
///
/// 本类只做生命周期接线与公开 API 转发：会话由 [SessionConnector]、配对由
/// [PairingFlow]、设备发现由 [DiscoveryCoordinator] 承担。
class ConnectionController extends StateNotifier<AuthState> {
  final Ref _ref;
  late final AuthStateWriter _state;
  late final DiscoveryCoordinator _discovery;
  late final SessionConnector _session;
  late final PairingFlow _pairing;
  bool _disposed = false;

  ConnectionController(
    this._ref, {
    DeviceDiscoveryRepository? discovery,
    PairingRepository? pairing,
  }) : super(const AuthState()) {
    _state = AuthStateWriter(
      read: () => state,
      write: _setState,
      isDisposed: () => _disposed,
    );
    _discovery = DiscoveryCoordinator(_ref, _state, discovery: discovery);
    _session = SessionConnector(_ref, _state, _discovery);
    _pairing = PairingFlow(
      _ref,
      _state,
      _discovery,
      _session,
      pairing: pairing,
    );
    unawaited(_initialize());
  }

  /// 安全写入状态，避免 notifier dispose 后触发断言
  void _setState(AuthState newState) {
    if (!_disposed) state = newState;
  }

  Future<void> _initialize() async {
    await _ref.read(settingsProvider.notifier).loaded;
    if (_disposed) return;
    final hasCachedEndpoint = _ref.read(settingsProvider).baseUrl != null;
    await _discovery.refresh(updateEndpoint: !hasCachedEndpoint);
    if (_disposed) return;
    _session.synchronizeConfig(_ref.read(settingsProvider));
    _discovery.startPeriodicRefresh();
  }

  /// 立即扫描一次局域网设备。
  Future<void> refreshDiscovery() => _discovery.refresh();

  /// 选中一台 PC：已配对则直接连接，否则立即发起配对。
  Future<bool> selectDevice(DiscoveredPc device) async {
    if (kDebugMode) {
      debugPrint(
        '[QingLink] selecting PC ${device.host}:${device.port} id=${device.id}',
      );
    }
    await _discovery.applyEndpoint(device);
    final token = _ref.read(settingsProvider).savedToken;
    if (token != null && token.isNotEmpty) {
      final connected = await connectOnly();
      if (kDebugMode) {
        debugPrint('[QingLink] select result connected=$connected');
      }
      return connected;
    }
    if (kDebugMode) {
      debugPrint('[QingLink] no token, starting pairing directly');
    }
    _setState(
      state.copyWith(
        connection: AppConnectionState.connecting,
        status: AuthStatus.unauthenticated,
        clearError: true,
      ),
    );
    try {
      await beginPairing();
      if (kDebugMode) {
        debugPrint('[QingLink][Pairing] started after selecting PC');
      }
      return true;
    } catch (error) {
      if (kDebugMode) debugPrint('[QingLink][Pairing] start failed: $error');
      _setState(
        state.copyWith(
          status: AuthStatus.unauthenticated,
          errorMessage:
              state.errorMessage ??
              '配对失败：${error is QingLinkPairingException ? pairingFailureText(error.code, error.message) : error}',
        ),
      );
      return false;
    }
  }

  /// 建立控制连接；重复调用复用进行中的连接。
  Future<bool> connectOnly() => _session.connectOnly();

  Future<bool> waitUntilReady({
    Duration timeout = const Duration(seconds: 12),
  }) {
    return _session.waitUntilReady(timeout: timeout);
  }

  /// 发起配对请求。
  Future<void> beginPairing() => _pairing.begin();

  /// 提交 PIN 完成配对。
  Future<void> pair(String pin) => _pairing.confirm(pin);

  /// 取消进行中的配对请求。
  Future<void> cancelPairing() => _pairing.cancel();

  /// 撤销令牌、断开连接并清空本地配对信息。
  Future<void> logout() => _session.logout();

  @override
  void dispose() {
    _disposed = true;
    _discovery.dispose();
    _session.dispose();
    super.dispose();
  }
}

final connectionProvider = StateNotifierProvider<ConnectionController, AuthState>(
  (ref) => ConnectionController(ref),
);
