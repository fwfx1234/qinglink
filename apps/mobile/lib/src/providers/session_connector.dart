import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../services/remote_session.dart';
import '../services/ws_client.dart';
import 'auth_failure_policy.dart';
import 'auth_state_transitions.dart';
import 'auth_state_writer.dart';
import 'dependencies.dart';
import 'discovery_coordinator.dart';
import 'settings_provider.dart';

/// 远程会话的建立、保持与失效恢复。
///
/// 负责把连接设置同步到会话、把会话状态与认证失败事件映射成 [AuthState]、
/// 断线后的端点回退重连、令牌失效后的游客态重连以及退出登录。配对请求本身
/// 由 `PairingFlow` 负责。
class SessionConnector {
  SessionConnector(this._ref, this._state, this._discovery) {
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
  }

  final Ref _ref;
  final AuthStateWriter _state;
  final DiscoveryCoordinator _discovery;
  Future<bool>? _connecting;
  StreamSubscription? _sessionFailureSubscription;
  StreamSubscription? _sessionStateSubscription;
  bool _recoveringAuthentication = false;

  /// 建立控制连接；重复调用复用进行中的连接。
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

  Future<bool> waitUntilReady({
    Duration timeout = const Duration(seconds: 12),
  }) {
    return _ref
        .read(remoteSessionRepositoryProvider)
        .waitUntilReady(timeout: timeout);
  }

  /// 配对成功后用新令牌建立控制连接。
  ///
  /// 连接失败时抛出 [RemoteSessionAuthFailure]，由配对流程统一映射提示。
  Future<void> connectAfterPairing(String baseUrl, String token) async {
    final session = _ref.read(remoteSessionRepositoryProvider);
    session.configure(baseUrl, token);
    final connected = await session.connect();
    if (!connected) {
      throw const RemoteSessionAuthFailure('AUTH_FAILED', '配对成功，但无法建立控制连接');
    }
    _state.update(
      AuthState(
        connection: AppConnectionState.connected,
        status: AuthStatus.authenticated,
        token: token,
      ),
    );
  }

  /// 按最新设置同步连接目标；没有端点时回到初始状态。
  void synchronizeConfig(ConnectionConfig config) {
    if (config.baseUrl == null) {
      _state.update(const AuthState());
      return;
    }
    if (_state.state.isAuthenticating) return;

    _state.update(
      AuthState(
        connection: AppConnectionState.disconnected,
        token: config.savedToken,
      ),
    );
    if (config.savedToken != null) unawaited(connectOnly());
  }

  /// 撤销当前令牌、断开连接并清空本地令牌。
  Future<void> logout() async {
    try {
      await _ref.read(remoteSessionRepositoryProvider).revokeCurrentToken();
    } catch (error, stackTrace) {
      if (kDebugMode) {
        debugPrint('[QingLink] revoke token failed: $error\n$stackTrace');
      }
    }
    _ref.read(remoteSessionRepositoryProvider).disconnect();
    _state.update(const AuthState());
    await _ref.read(settingsProvider.notifier).clearToken();
  }

  void _onConnectionConfigChanged(
    ConnectionConfig? previous,
    ConnectionConfig next,
  ) {
    if (_state.isDisposed || previous == null || _recoveringAuthentication) {
      return;
    }
    if (previous.baseUrl == next.baseUrl &&
        previous.savedToken == next.savedToken) {
      return;
    }

    // Pairing saves the newly issued token while the existing session is
    // authenticating. Keep that socket and let pair() promote it to ready.
    final pairingInProgress =
        _state.state.isAuthenticating &&
        previous.savedToken == null &&
        next.savedToken != null;
    if (!pairingInProgress) {
      _ref.read(remoteSessionRepositoryProvider).disconnect();
      synchronizeConfig(next);
    }
  }

  void _onSessionState(WsConnectionState wsState, bool ready) {
    if (_state.isDisposed) return;
    switch (wsState) {
      case WsConnectionState.connecting:
        _state.update(
          _state.state.copyWith(connection: AppConnectionState.connecting),
        );
        break;
      case WsConnectionState.connected:
        _state.update(
          _state.state.copyWith(connection: AppConnectionState.connected),
        );
        break;
      case WsConnectionState.disconnected:
      case WsConnectionState.reconnecting:
        _state.update(
          _state.state.copyWith(connection: AppConnectionState.disconnected),
        );
        break;
    }
    if (ready) {
      _state.update(
        restoreAuthenticatedStateAfterReconnect(
          _state.state,
          _ref.read(settingsProvider).savedToken,
        ),
      );
    }
  }

  void _onAuthenticationFailure(RemoteSessionAuthFailure failure) {
    if (_state.isDisposed) return;
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
    final hint = AuthFailurePolicy.messageFor(failure.code);
    if (hint != null) {
      _state.update(
        _state.state.copyWith(
          connection: AppConnectionState.disconnected,
          errorMessage: hint,
        ),
      );
      return;
    }
    if (!AuthFailurePolicy.isInvalidTokenCode(failure.code)) return;
    await _ref.read(settingsProvider.notifier).clearToken();
    _state.update(
      const AuthState(
        connection: AppConnectionState.disconnected,
        status: AuthStatus.unauthenticated,
        errorMessage: '连接已过期，请重新配对',
      ),
    );
  }

  Future<bool> _connectOnly({bool allowDiscoveryFallback = true}) async {
    await _ref.read(settingsProvider.notifier).loaded;
    var config = _ref.read(settingsProvider);
    if (config.baseUrl == null) {
      await _discovery.refresh();
      config = _ref.read(settingsProvider);
    }
    final baseUrl = config.baseUrl;
    if (baseUrl == null) return false;
    final token = config.savedToken;
    // AuthState can briefly retain the previous session while a discovered
    // endpoint is being selected. Only short-circuit when the transport is
    // still actually authenticated and ready.
    if (_state.state.isAuthenticated &&
        _state.state.token == token &&
        _ref.read(remoteSessionRepositoryProvider).isReady) {
      return true;
    }

    _state.update(
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
      session.configure(baseUrl, token);
      final connected = await session.connect();
      if (!connected) {
        if (allowDiscoveryFallback) {
          final retried = await _retryWithDiscoveredEndpoint(config);
          if (retried != null) return retried;
        }
        return false;
      }
      return _markSessionConnected(token);
    } on RemoteSessionAuthFailure catch (failure) {
      return _recoverFromAuthenticationFailure(failure, config);
    } catch (error) {
      if (kDebugMode) debugPrint('[QingLink] connect failed: $error');
      if (allowDiscoveryFallback) {
        final retried = await _retryWithDiscoveredEndpoint(config);
        if (retried != null) return retried;
      }
      _state.update(
        _state.state.copyWith(
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

  /// 重新扫描局域网；端点变化时用新端点重连一次。
  ///
  /// 返回 `null` 表示端点没有变化，调用方继续按原有失败路径处理。
  Future<bool?> _retryWithDiscoveredEndpoint(ConnectionConfig previous) async {
    _ref.read(remoteSessionRepositoryProvider).disconnect();
    await _discovery.refresh();
    final discoveredConfig = _ref.read(settingsProvider);
    if (discoveredConfig.baseUrl != null &&
        discoveredConfig.baseUrl != previous.baseUrl) {
      return _connectOnly(allowDiscoveryFallback: false);
    }
    return null;
  }

  /// 传输层连上后写入认证状态。
  bool _markSessionConnected(String? token) {
    if (token == null) {
      _state.update(
        _state.state.copyWith(
          connection: AppConnectionState.connected,
          status: AuthStatus.unauthenticated,
        ),
      );
      return true;
    }

    _state.update(
      _state.state.copyWith(
        connection: AppConnectionState.connected,
        status: AuthStatus.authenticated,
        token: token,
        clearError: true,
      ),
    );
    return true;
  }

  Future<bool> _recoverFromAuthenticationFailure(
    RemoteSessionAuthFailure failure,
    ConnectionConfig config,
  ) async {
    _recoveringAuthentication = true;
    await _handleAuthenticationFailure(failure, force: true);
    // A missing/expired server token is a pairing state, not a dead
    // connection. Re-open the socket without a token so HomeScreen can
    // launch the normal pairing-request flow.
    try {
      if (!AuthFailurePolicy.isInvalidTokenCode(failure.code)) return false;
      return await _connectAsGuest(config.baseUrl!);
    } finally {
      _recoveringAuthentication = false;
    }
  }

  /// 以游客（无令牌）身份重开连接，供配对流程使用。
  Future<bool> _connectAsGuest(String baseUrl) async {
    try {
      final session = _ref.read(remoteSessionRepositoryProvider);
      session.configure(baseUrl, null);
      final guestConnected = await session.connect();
      if (!guestConnected) return false;
      _state.update(
        const AuthState(
          connection: AppConnectionState.connected,
          status: AuthStatus.unauthenticated,
        ),
      );
      return true;
    } catch (error, stackTrace) {
      if (kDebugMode) {
        debugPrint('[QingLink] guest reconnect failed: $error\n$stackTrace');
      }
      return false;
    }
  }

  /// 取消会话事件订阅。
  void dispose() {
    _sessionFailureSubscription?.cancel();
    _sessionStateSubscription?.cancel();
  }
}
