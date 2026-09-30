import 'dart:async';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../data/repositories/device_discovery_repository.dart';
import '../data/repositories/pairing_repository.dart';
import '../services/remote_session.dart';
import '../services/qinglink_pairing.dart';
import '../services/ws_client.dart';
import '../utils/wake_on_lan.dart';
import 'settings_provider.dart';
import 'dependencies.dart';

enum VerifyResult { success, tokenExpired, networkError }

/// Restores the authenticated state after a transport reconnects and the
/// server has accepted the saved token.
AuthState restoreAuthenticatedStateAfterReconnect(
  AuthState current,
  String? savedToken,
) {
  final token = current.token ?? savedToken;
  if (token == null || token.isEmpty) {
    return current.copyWith(connection: AppConnectionState.connected);
  }
  return AuthState(
    connection: AppConnectionState.connected,
    status: AuthStatus.authenticated,
    token: token,
    errorMessage: null,
  );
}

/// Coordinates discovery, pairing and authenticated session transitions.
class ConnectionController extends StateNotifier<AuthState> {
  final Ref _ref;
  final DeviceDiscoveryRepository _discovery;
  final PairingRepository _pairing;
  bool _disposed = false;
  Future<bool>? _connecting;
  StreamSubscription? _sessionFailureSubscription;
  StreamSubscription? _sessionStateSubscription;
  Timer? _discoveryTimer;
  String? _pairingRequestId;
  String? _pairingDeviceId;
  bool _recoveringAuthentication = false;

  ConnectionController(
    this._ref, {
    DeviceDiscoveryRepository? discovery,
    PairingRepository? pairing,
  }) : _discovery = discovery ?? _ref.read(deviceDiscoveryRepositoryProvider),
       _pairing = pairing ?? _ref.read(pairingRepositoryProvider),
       super(const AuthState()) {
    _ref.listen<ConnectionConfig>(settingsProvider, _onConnectionConfigChanged);
    final sessionRepository = _ref.read(remoteSessionRepositoryProvider);
    _sessionFailureSubscription = sessionRepository.authenticationFailures.listen(
      _onAuthenticationFailure,
    );
    _sessionStateSubscription = sessionRepository.states.listen((sessionState) {
      _onSessionState(
        switch (sessionState) {
          RemoteSessionState.ready => WsConnectionState.connected,
          RemoteSessionState.connecting || RemoteSessionState.authenticating =>
            WsConnectionState.connecting,
          _ => WsConnectionState.disconnected,
        },
        sessionState == RemoteSessionState.ready,
      );
    });
    unawaited(_initialize());
  }

  void _setState(AuthState newState) {
    if (!_disposed) state = newState;
  }

  Future<void> _initialize() async {
    await _ref.read(settingsProvider.notifier).loaded;
    if (_disposed) return;
    final hasCachedEndpoint = _ref.read(settingsProvider).baseUrl != null;
    await _refreshDiscovery(updateEndpoint: !hasCachedEndpoint);
    if (_disposed) return;
    _synchronizeConnectionConfig(_ref.read(settingsProvider));
    _discoveryTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => unawaited(_refreshDiscovery()),
    );
  }

  Future<void> _refreshDiscovery({bool updateEndpoint = true}) async {
    if (_disposed) return;
    try {
      final devices = await _discovery.scan(
        timeout: const Duration(seconds: 2),
      );
      if (_disposed) return;
      _setState(state.copyWith(discoveredDevices: devices));
      if (devices.isEmpty) return;
      final config = _ref.read(settingsProvider);
      DiscoveredPc? selected;
      if (config.pcId != null) {
        for (final device in devices) {
          if (device.id == config.pcId) selected = device;
        }
      } else if (devices.length == 1) {
        selected = devices.single;
      }
      if (selected == null) return;
      if (!updateEndpoint) return;
      // Keep a healthy direct-IP session stable. The discovered address is
      // applied after a disconnect or during initial setup, when it is needed
      // to recover from a DHCP change.
      if (_ref.read(remoteSessionRepositoryProvider).isReady &&
          state.isAuthenticated) {
        return;
      }
      await _ref
          .read(settingsProvider.notifier)
          .applyDiscoveredEndpoint(
            pcId: selected.id,
            pcName: selected.name,
            address: selected.host,
            port: selected.port,
          );
    } catch (error) {
      // Keep the cached endpoint, but surface discovery failures for diagnosis.
      if (!_disposed) {
        _setState(state.copyWith(errorMessage: '局域网发现失败：$error'));
      }
    }
  }

  Future<void> refreshDiscovery() => _refreshDiscovery();

  Future<bool> selectDevice(DiscoveredPc device) async {
    if (kDebugMode) {
      debugPrint(
        '[QingLink] selecting PC ${device.host}:${device.port} id=${device.id}',
      );
    }
    await _ref
        .read(settingsProvider.notifier)
        .applyDiscoveredEndpoint(
          pcId: device.id,
          pcName: device.name,
          address: device.host,
          port: device.port,
        );
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
          errorMessage: '配对失败：$error',
        ),
      );
      return false;
    }
  }

  void _onConnectionConfigChanged(
    ConnectionConfig? previous,
    ConnectionConfig next,
  ) {
    if (_disposed || previous == null || _recoveringAuthentication) return;
    if (previous.baseUrl == next.baseUrl &&
        previous.savedToken == next.savedToken) {
      return;
    }

    // Pairing saves the newly issued token while the existing session is
    // authenticating. Keep that socket and let pair() promote it to ready.
    final pairingInProgress =
        state.isAuthenticating &&
        previous.savedToken == null &&
        next.savedToken != null;
    if (!pairingInProgress) {
      _ref.read(remoteSessionRepositoryProvider).disconnect();
      _synchronizeConnectionConfig(next);
    }
  }

  void _synchronizeConnectionConfig(ConnectionConfig config) {
    if (config.baseUrl == null) {
      _setState(const AuthState());
      return;
    }
    if (state.isAuthenticating) return;

    _setState(
      AuthState(
        connection: AppConnectionState.disconnected,
        token: config.savedToken,
      ),
    );
    if (config.savedToken != null) unawaited(connectOnly());
  }

  void _onSessionState(WsConnectionState wsState, bool ready) {
    if (_disposed) return;
    switch (wsState) {
      case WsConnectionState.connecting:
        _setState(state.copyWith(connection: AppConnectionState.connecting));
        break;
      case WsConnectionState.connected:
        _setState(state.copyWith(connection: AppConnectionState.connected));
        break;
      case WsConnectionState.disconnected:
      case WsConnectionState.reconnecting:
        _setState(state.copyWith(connection: AppConnectionState.disconnected));
        break;
    }
    if (ready) {
      _setState(
        restoreAuthenticatedStateAfterReconnect(
          state,
          _ref.read(settingsProvider).savedToken,
        ),
      );
    }
  }

  void _onAuthenticationFailure(RemoteSessionAuthFailure failure) {
    if (_disposed) return;
    unawaited(_handleAuthenticationFailure(failure));
  }

  Future<void> _handleAuthenticationFailure(
    RemoteSessionAuthFailure failure, {
    bool force = false,
  }) async {
    // _connectOnly() owns the invalid-token -> guest reconnect transition.
    // Do not let the broadcast failure listener race that recovery by
    // clearing settings and tearing down the newly opened guest socket.
    if (_recoveringAuthentication && !force) return;
    final invalid =
        failure.code == 'INVALID_TOKEN' ||
        failure.code == 'TOKEN_EXPIRED' ||
        failure.code == 'AUTH_FAILED';
    if (failure.code == 'REPLACED') {
      _setState(
        state.copyWith(
          connection: AppConnectionState.disconnected,
          errorMessage: '此设备已在其他客户端建立连接',
        ),
      );
      return;
    }
    if (!invalid) return;
    await _ref.read(settingsProvider.notifier).clearToken();
    _setState(
      AuthState(
        connection: AppConnectionState.disconnected,
        status: AuthStatus.unauthenticated,
        errorMessage: '连接已过期，请重新配对',
      ),
    );
  }

  Future<bool> connectOnly() async {
    final active = _connecting;
    if (active != null) return active;

    final future = _connectOnly();
    _connecting = future;
    try {
      return await future;
    } finally {
      if (identical(_connecting, future)) _connecting = null;
    }
  }

  Future<bool> waitUntilReady({Duration timeout = const Duration(seconds: 12)}) {
    return _ref.read(remoteSessionRepositoryProvider).waitUntilReady(timeout: timeout);
  }

  Future<bool> _connectOnly({bool allowDiscoveryFallback = true}) async {
    await _ref.read(settingsProvider.notifier).loaded;
    var config = _ref.read(settingsProvider);
    if (config.baseUrl == null) {
      await _refreshDiscovery();
      config = _ref.read(settingsProvider);
    }
    if (config.baseUrl == null) return false;
    final token = config.savedToken;
    // AuthState can briefly retain the previous session while a discovered
    // endpoint is being selected. Only short-circuit when the transport is
    // still actually authenticated and ready.
    if (state.isAuthenticated &&
        state.token == token &&
        _ref.read(remoteSessionRepositoryProvider).isReady) {
      return true;
    }

    _setState(
      AuthState(
        connection: AppConnectionState.connecting,
        status: token == null
            ? AuthStatus.unauthenticated
            : AuthStatus.authenticating,
        token: token,
      ),
    );

    try {
      final session = _ref.read(remoteSessionRepositoryProvider);
      session.configure(config.baseUrl!, token);
      final connected = await session.connect();
      if (!connected && allowDiscoveryFallback) {
        session.disconnect();
        await _refreshDiscovery();
        final discoveredConfig = _ref.read(settingsProvider);
        if (discoveredConfig.baseUrl != null &&
            discoveredConfig.baseUrl != config.baseUrl) {
          return _connectOnly(allowDiscoveryFallback: false);
        }
      }
      if (!connected) return false;

      if (token == null) {
        _setState(
          state.copyWith(
            connection: AppConnectionState.connected,
            status: AuthStatus.unauthenticated,
          ),
        );
        return true;
      }

      _setState(
        state.copyWith(
          connection: AppConnectionState.connected,
          status: AuthStatus.authenticated,
          token: token,
          clearError: true,
        ),
      );
      return true;
    } on RemoteSessionAuthFailure catch (failure) {
      _recoveringAuthentication = true;
      await _handleAuthenticationFailure(failure, force: true);
      // A missing/expired server token is a pairing state, not a dead
      // connection. Re-open the socket without a token so HomeScreen can
      // launch the normal pairing-request flow.
      final canPair =
          failure.code == 'INVALID_TOKEN' ||
          failure.code == 'TOKEN_EXPIRED' ||
          failure.code == 'AUTH_FAILED';
      try {
        if (canPair) {
          try {
            final session = _ref.read(remoteSessionRepositoryProvider);
            session.configure(config.baseUrl!, null);
            final guestConnected = await session.connect();
            if (guestConnected) {
              _setState(
                const AuthState(
                  connection: AppConnectionState.connected,
                  status: AuthStatus.unauthenticated,
                ),
              );
              return true;
            }
          } catch (error, stackTrace) {
            if (kDebugMode) {
              debugPrint('[QingLink] guest reconnect failed: $error\n$stackTrace');
            }
          }
        }
        return false;
      } finally {
        _recoveringAuthentication = false;
      }
    } catch (error) {
      if (kDebugMode) debugPrint('[QingLink] connect failed: $error');
      if (allowDiscoveryFallback) {
        _ref.read(remoteSessionRepositoryProvider).disconnect();
        await _refreshDiscovery();
        final discoveredConfig = _ref.read(settingsProvider);
        if (discoveredConfig.baseUrl != null &&
            discoveredConfig.baseUrl != config.baseUrl) {
          return _connectOnly(allowDiscoveryFallback: false);
        }
      }
      _setState(
        state.copyWith(
          connection: AppConnectionState.disconnected,
          status: token == null
              ? AuthStatus.unauthenticated
              : AuthStatus.authenticating,
          token: token,
          errorMessage: token == null
              ? '无法连接到局域网内的 PC，请确认电脑已启动'
              : 'PC 暂时无法连接，正在等待唤醒后自动重连 ($error)',
        ),
      );
      return false;
    }
  }

  Future<void> beginPairing() async {
    if (_pairingRequestId != null) return;
    await _ref.read(settingsProvider.notifier).loaded;
    var config = _ref.read(settingsProvider);
    if (config.baseUrl == null) {
      await _refreshDiscovery();
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
    _pairingDeviceId = deviceId;
    _pairingRequestId = await _pairing.request(
      baseUrl: baseUrl,
      deviceId: deviceId,
      deviceName: '轻联手机',
    );
    if ((_pairingRequestId?.isEmpty ?? true) &&
        _pairing.lastAutoConfirm != null) {
      final token = _pairing.lastAutoConfirm!['token'] as String?;
      if (token != null && token.isNotEmpty) {
        if (kDebugMode) debugPrint('[QingLink][Pairing] auto-confirmed');
        await _ref.read(settingsProvider.notifier).saveToken(token);
        final pcId = _pairing.lastAutoConfirm!['pc_id'] as String?;
        if (pcId != null && pcId.isNotEmpty) {
          await _ref.read(settingsProvider.notifier).savePcIdentity(pcId);
        }
        final mac = _pairing.lastAutoConfirm!['mac_address'] as String?;
        final wakeNetworks =
            (_pairing.lastAutoConfirm!['wake_networks'] as List<dynamic>? ??
                    const [])
                .whereType<Map>()
                .map((e) => WakeNetwork.fromJson(Map<String, dynamic>.from(e)))
                .toList(growable: false);
        if (mac != null && mac.isNotEmpty) {
          await _ref.read(settingsProvider.notifier).updateMacAddress(mac);
        }
        if (wakeNetworks.isNotEmpty) {
          await _ref
              .read(settingsProvider.notifier)
              .updateWakeNetworks(wakeNetworks);
        }
        _pairingRequestId = null;
        _pairingDeviceId = null;
        _setState(
          state.copyWith(
            connection: AppConnectionState.connected,
            status: AuthStatus.authenticating,
            clearError: true,
          ),
        );
        unawaited(connectOnly());
        return;
      }
    }
    if (kDebugMode) {
      debugPrint('[QingLink][Pairing] request created id=$_pairingRequestId');
    }
    _setState(
      state.copyWith(
        // Pairing requests use HTTP and remain valid even when the optional
        // WS session handshake is unavailable.
        connection: AppConnectionState.connected,
        status: AuthStatus.unauthenticated,
        clearError: true,
      ),
    );
  }

  Future<void> pair(String pin) async {
    final config = _ref.read(settingsProvider);
    final requestId = _pairingRequestId;
    final deviceId = _pairingDeviceId;
    if (config.baseUrl == null || requestId == null || deviceId == null) {
      _setState(
        state.copyWith(
          status: AuthStatus.unauthenticated,
          errorMessage: '请先发起配对请求',
        ),
      );
      return;
    }

    _setState(
      state.copyWith(status: AuthStatus.authenticating, clearError: true),
    );
    try {
      final response = await _pairing.confirm(
        baseUrl: config.baseUrl!,
        requestId: requestId,
        pin: pin,
        deviceId: deviceId,
        deviceName: '轻联手机',
      );
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
      await _ref.read(settingsProvider.notifier).saveToken(token);
      if (pcId != null && pcId.isNotEmpty) {
        await _ref.read(settingsProvider.notifier).savePcIdentity(pcId);
      }
      final pairedPcId = pcId ?? config.pcId;
      if (pairedPcId != null &&
          pairedPcId.isNotEmpty &&
          config.pcIpAddress != null &&
          config.pcIpAddress!.isNotEmpty) {
        await _ref
            .read(settingsProvider.notifier)
            .applyDiscoveredEndpoint(
              pcId: pairedPcId,
              pcName: pcName,
              address: config.pcIpAddress!,
              port: config.pcPort,
            );
      }
      if (mac != null && mac.isNotEmpty) {
        await _ref.read(settingsProvider.notifier).updateMacAddress(mac);
      }
      if (wakeNetworks.isNotEmpty) {
        await _ref
            .read(settingsProvider.notifier)
            .updateWakeNetworks(wakeNetworks);
      }
      final session = _ref.read(remoteSessionRepositoryProvider);
      session.configure(config.baseUrl!, token);
      final connected = await session.connect();
      if (!connected) {
        throw const RemoteSessionAuthFailure('AUTH_FAILED', '配对成功，但无法建立控制连接');
      }
      _setState(
        AuthState(
          connection: AppConnectionState.connected,
          status: AuthStatus.authenticated,
          token: token,
        ),
      );
      _pairingRequestId = null;
    } on RemoteSessionAuthFailure catch (failure) {
      if (kDebugMode) {
        debugPrint('[QingLink][Pairing] auth failure code=${failure.code}');
      }
      _setState(
        state.copyWith(
          status: AuthStatus.unauthenticated,
          errorMessage: failure.code == 'INVALID_PIN'
              ? '配对失败，请检查 PIN 码'
              : failure.message,
        ),
      );
    } on QingLinkPairingException catch (failure) {
      if (kDebugMode) {
        debugPrint('[QingLink][Pairing] request failure code=${failure.code}');
      }
      _setState(
        state.copyWith(
          status: AuthStatus.unauthenticated,
          errorMessage: failure.code == 'INVALID_PIN'
              ? '配对失败，请检查匹配码'
              : failure.message,
        ),
      );
    } catch (_) {
      _setState(
        state.copyWith(
          status: AuthStatus.unauthenticated,
          errorMessage: '配对超时，请重试',
        ),
      );
    }
  }

  Future<void> cancelPairing() async {
    final requestId = _pairingRequestId;
    final baseUrl = _ref.read(settingsProvider).baseUrl;
    _pairingRequestId = null;
    if (requestId != null && baseUrl != null) {
      try {
        await _pairing.cancel(baseUrl: baseUrl, requestId: requestId);
      } catch (error, stackTrace) {
        if (kDebugMode) {
          debugPrint('[QingLink] cancel pairing failed: $error\n$stackTrace');
        }
      }
    }
    if (!state.isAuthenticated) {
      _setState(state.copyWith(status: AuthStatus.unauthenticated));
    }
  }

  Future<void> logout() async {
    try {
      await _ref.read(remoteSessionRepositoryProvider).revokeCurrentToken();
    } catch (error, stackTrace) {
      if (kDebugMode) {
        debugPrint('[QingLink] revoke token failed: $error\n$stackTrace');
      }
    }
    _ref.read(remoteSessionRepositoryProvider).disconnect();
    _setState(const AuthState());
    await _ref.read(settingsProvider.notifier).clearToken();
  }

  @override
  void dispose() {
    _disposed = true;
    _discoveryTimer?.cancel();
    _sessionFailureSubscription?.cancel();
    _sessionStateSubscription?.cancel();
    super.dispose();
  }
}

final connectionProvider = StateNotifierProvider<ConnectionController, AuthState>(
  (ref) => ConnectionController(ref),
);
