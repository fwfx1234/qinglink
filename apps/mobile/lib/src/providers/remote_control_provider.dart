import 'dart:async';
import 'dart:collection';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../models/display_info.dart';
import '../models/fs_entry.dart';
import '../models/process_info.dart';
import '../models/system_status.dart';
import '../services/mouse_move_frame.dart';
import '../services/remote_session.dart';
import '../services/shortcut_mapper.dart';
import '../services/ws_client.dart';
import '../services/ws_event_adapter.dart';
import '../utils/online_grace_tracker.dart';
import '../utils/wake_on_lan.dart';
import 'settings_provider.dart';

part 'remote_control_system.dart';
part 'remote_control_processes.dart';
part 'remote_control_launcher.dart';
part 'remote_control_files.dart';
part 'remote_control_power.dart';
part 'remote_control_input.dart';
part 'remote_control_windows.dart';

class RemoteCommandException implements Exception {
  const RemoteCommandException(this.code, this.message);

  factory RemoteCommandException.fromResponse(Map<String, dynamic> response) {
    final data = response['data'];
    final error = data is Map<String, dynamic>
        ? data
        : const <String, dynamic>{};
    return RemoteCommandException(
      error['code'] as String? ?? 'REMOTE_COMMAND_FAILED',
      error['message'] as String? ?? 'PC 未能完成该操作',
    );
  }

  final String code;
  final String message;

  @override
  String toString() => '$code: $message';
}

class RemoteControlState {
  final bool isConnected;

  /// PC 是否被认为在线：由心跳活性（宽限期）判定，与 socket 瞬时状态解耦。
  final bool pcOnline;
  final WsConnectionState wsState;
  final SystemStatus? systemStatus;
  final ForegroundResponse? foregroundApp;
  final VolumeStatus? volumeStatus;
  final List<AudioOutputDevice> audioOutputs;
  final bool isLoadingAudioOutputs;
  final String audioOutputsError;
  final List<DisplayInfo> displays;
  final bool isLoadingDisplays;
  final String displayError;
  final UnlockStatus? unlockStatus;
  final LockInputStatus? lockInputStatus;
  final SecureDesktopStatus? secureDesktopStatus;
  final List<ProcessInfo> processes;
  final bool processHasMore;
  final int processPage;
  final String processQuery;
  final String processFilter;
  final List<LauncherItem> appItems;
  final List<LauncherItem> gameItems;
  final List<LauncherItem> launcherItems;
  final List<LauncherItem> recentLauncherItems;
  final bool isLoadingLibrary;
  final bool isLoadingMoreLibrary;
  final bool libraryHasMore;
  final int libraryPage;
  final String libraryQuery;

  /// 当前统一库分类：`steam` / `software` / `other`（`all` 兼容旧调用）。
  final String libraryKind;
  final bool appHasMore;
  final bool gameHasMore;
  final bool isLoadingApps;
  final bool isLoadingGames;
  final bool isLoadingLauncher;
  final bool isLoading;
  final bool isRefreshingApps;
  final String? errorMessage;

  const RemoteControlState({
    this.isConnected = false,
    this.pcOnline = false,
    this.wsState = WsConnectionState.disconnected,
    this.systemStatus,
    this.foregroundApp,
    this.volumeStatus,
    this.audioOutputs = const [],
    this.isLoadingAudioOutputs = false,
    this.audioOutputsError = '',
    this.displays = const [],
    this.isLoadingDisplays = false,
    this.displayError = '',
    this.unlockStatus,
    this.lockInputStatus,
    this.secureDesktopStatus,
    this.processes = const [],
    this.processHasMore = false,
    this.processPage = 0,
    this.processQuery = '',
    this.processFilter = 'user',
    this.appItems = const [],
    this.gameItems = const [],
    this.launcherItems = const [],
    this.recentLauncherItems = const [],
    this.isLoadingLibrary = false,
    this.isLoadingMoreLibrary = false,
    this.libraryHasMore = false,
    this.libraryPage = 0,
    this.libraryQuery = '',
    this.libraryKind = 'all',
    this.appHasMore = false,
    this.gameHasMore = false,
    this.isLoadingApps = false,
    this.isLoadingGames = false,
    this.isLoadingLauncher = false,
    this.isLoading = false,
    this.isRefreshingApps = false,
    this.errorMessage,
  });

  RemoteControlState copyWith({
    bool? isConnected,
    bool? pcOnline,
    WsConnectionState? wsState,
    SystemStatus? systemStatus,
    ForegroundResponse? foregroundApp,
    VolumeStatus? volumeStatus,
    List<AudioOutputDevice>? audioOutputs,
    bool? isLoadingAudioOutputs,
    String? audioOutputsError,
    List<DisplayInfo>? displays,
    bool? isLoadingDisplays,
    String? displayError,
    UnlockStatus? unlockStatus,
    LockInputStatus? lockInputStatus,
    SecureDesktopStatus? secureDesktopStatus,
    List<ProcessInfo>? processes,
    bool? processHasMore,
    int? processPage,
    String? processQuery,
    String? processFilter,
    List<LauncherItem>? appItems,
    List<LauncherItem>? gameItems,
    List<LauncherItem>? launcherItems,
    List<LauncherItem>? recentLauncherItems,
    bool? isLoadingLibrary,
    bool? isLoadingMoreLibrary,
    bool? libraryHasMore,
    int? libraryPage,
    String? libraryQuery,
    String? libraryKind,
    bool? appHasMore,
    bool? gameHasMore,
    bool? isLoadingApps,
    bool? isLoadingGames,
    bool? isLoadingLauncher,
    bool? isLoading,
    bool? isRefreshingApps,
    String? errorMessage,
    bool clearError = false,
  }) {
    return RemoteControlState(
      isConnected: isConnected ?? this.isConnected,
      pcOnline: pcOnline ?? this.pcOnline,
      wsState: wsState ?? this.wsState,
      systemStatus: systemStatus ?? this.systemStatus,
      foregroundApp: foregroundApp ?? this.foregroundApp,
      volumeStatus: volumeStatus ?? this.volumeStatus,
      audioOutputs: audioOutputs ?? this.audioOutputs,
      isLoadingAudioOutputs:
          isLoadingAudioOutputs ?? this.isLoadingAudioOutputs,
      audioOutputsError: audioOutputsError ?? this.audioOutputsError,
      displays: displays ?? this.displays,
      isLoadingDisplays: isLoadingDisplays ?? this.isLoadingDisplays,
      displayError: displayError ?? this.displayError,
      unlockStatus: unlockStatus ?? this.unlockStatus,
      lockInputStatus: lockInputStatus ?? this.lockInputStatus,
      secureDesktopStatus: secureDesktopStatus ?? this.secureDesktopStatus,
      processes: processes ?? this.processes,
      processHasMore: processHasMore ?? this.processHasMore,
      processPage: processPage ?? this.processPage,
      processQuery: processQuery ?? this.processQuery,
      processFilter: processFilter ?? this.processFilter,
      appItems: appItems ?? this.appItems,
      gameItems: gameItems ?? this.gameItems,
      launcherItems: launcherItems ?? this.launcherItems,
      recentLauncherItems: recentLauncherItems ?? this.recentLauncherItems,
      isLoadingLibrary: isLoadingLibrary ?? this.isLoadingLibrary,
      isLoadingMoreLibrary: isLoadingMoreLibrary ?? this.isLoadingMoreLibrary,
      libraryHasMore: libraryHasMore ?? this.libraryHasMore,
      libraryPage: libraryPage ?? this.libraryPage,
      libraryQuery: libraryQuery ?? this.libraryQuery,
      libraryKind: libraryKind ?? this.libraryKind,
      appHasMore: appHasMore ?? this.appHasMore,
      gameHasMore: gameHasMore ?? this.gameHasMore,
      isLoadingApps: isLoadingApps ?? this.isLoadingApps,
      isLoadingGames: isLoadingGames ?? this.isLoadingGames,
      isLoadingLauncher: isLoadingLauncher ?? this.isLoadingLauncher,
      isLoading: isLoading ?? this.isLoading,
      isRefreshingApps: isRefreshingApps ?? this.isRefreshingApps,
      errorMessage: clearError ? null : (errorMessage ?? this.errorMessage),
    );
  }
}

/// 承载会话与公共 WebSocket 通道，动作分组见各 part 文件。
abstract class _RemoteControlBase extends StateNotifier<RemoteControlState> {
  final Ref _ref;
  bool _disposed = false;
  late final _LifecycleObserver _lifecycleObserver;

  final RemoteSession _session;
  StreamSubscription? _sessionEventSubscription;
  StreamSubscription? _sessionStateSubscription;
  final LinkedHashMap<String, String?> _iconCache = LinkedHashMap();
  final Map<String, int> _launcherLoadGeneration = {'app': 0, 'game': 0};
  Completer<void>? _readyCompleter;
  int _mouseSequence = 0;
  int _pendingMouseDx = 0;
  int _pendingMouseDy = 0;
  Timer? _mouseFlushTimer;

  static const _pcOnlineGrace = Duration(seconds: 15);
  static const _expectOfflineDelay = Duration(seconds: 3);
  late final OnlineGraceTracker _onlineTracker;
  final _offlineEventController = StreamController<void>.broadcast();
  Timer? _expectOfflineTimer;

  _RemoteControlBase(Ref ref)
    : _ref = ref,
      _session = RemoteSession(),
      super(const RemoteControlState()) {
    _sessionEventSubscription = _session.eventStream.listen(_handleWsEvent);
    _sessionStateSubscription = _session.stateStream.listen(
      _handleSessionStateChange,
    );
    _onlineTracker = OnlineGraceTracker(
      grace: _pcOnlineGrace,
      onOffline: () {
        if (!_disposed) _markOfflineGenuine();
      },
    );
    _lifecycleObserver = _LifecycleObserver((state) {
      if (state == AppLifecycleState.paused ||
          state == AppLifecycleState.detached) {
        _session.suspend();
      } else if (state == AppLifecycleState.resumed) {
        if (!_disposed) {
          // 先恢复宽限判定，再重连：连接状态变化会重新驱动在线判定。
          _onlineTracker.resume();
          _session.resume();
        }
      }
    });
    WidgetsBinding.instance.addObserver(_lifecycleObserver);
  }

  // 连接/事件回调与跨分组动作所需契约，由对应动作 mixin 实现。
  Future<void> refreshSystemStatus();
  Future<void> refreshForeground();
  Future<void> refreshProcesses({
    String? search,
    int page = 0,
    bool append = false,
    String? filter,
  });
  Future<UnlockStatus?> getUnlockStatus();
  Future<SecureDesktopStatus?> getSecureDesktopStatus();
  Future<bool> loadLauncherItems(
    String kind, {
    String? query,
    int page = 0,
    bool append = false,
    bool force = false,
  });
  Future<bool> loadUnifiedLibrary({
    String? kind,
    String? query,
    bool force = false,
    int page = 0,
    bool append = false,
  });
  Future<bool> loadRecentUnifiedLibrary();
  void _setState(RemoteControlState newState) {
    if (_disposed) return;
    state = newState;
  }

  // ========== WebSocket ==========

  Future<bool> startWebSocket(String baseUrl, String token) {
    _session.configure(baseUrl, token);
    return _session.connect();
  }

  Future<bool> connectForAuthentication(String baseUrl, String? token) {
    _session.configure(baseUrl, token);
    return _session.connect();
  }

  Future<Map<String, dynamic>> authenticateWithPairing(String pin) {
    return _session.pair(pin);
  }

  Stream<RemoteSessionAuthFailure> get authenticationFailures =>
      _session.failureStream;

  /// 真实离线事件（宽限期到期）流：登出/显式断开不会触发。
  Stream<void> get offlineEvents => _offlineEventController.stream;

  bool get hasTransportConnection => _session.isTransportConnected;

  void _handleWsEvent(Map<String, dynamic> event) {
    if (_disposed) return;
    final type = event['type'] as String;

    switch (type) {
      case 'foreground_changed':
        final fg = WsEventAdapter.parseForegroundChanged(event);
        if (fg != null) {
          _setState(state.copyWith(foregroundApp: fg));
        }
        break;
      case 'process_started':
      case 'process_ended':
        refreshProcesses(
          search: state.processQuery.isEmpty ? null : state.processQuery,
        );
        break;
      case 'launcher_catalog_ready':
        final kind = event['kind'] as String?;
        if (kind != null && (kind == 'app' || kind == 'game')) {
          _setState(state.copyWith(isRefreshingApps: false));
          _iconCache.clear();
          loadLauncherItems(kind, force: true);
          // 扫描完成后刷新当前分类，保证新增/删除条目立即反映。
          final libraryKind = state.libraryKind;
          if (libraryKind == 'steam' ||
              libraryKind == 'software' ||
              libraryKind == 'other') {
            loadUnifiedLibrary(kind: libraryKind, force: true);
          }
          loadRecentUnifiedLibrary();
        }
        break;
      case 'system_suspend':
        _setState(
          state.copyWith(
            isConnected: false,
            foregroundApp: null,
            errorMessage: 'PC已进入休眠',
          ),
        );
        break;
      case 'system_resume':
        _setState(
          state.copyWith(
            isConnected: false,
            foregroundApp: null,
            clearError: true,
          ),
        );
        _ensureConnected();
        break;
      case 'system_shutdown':
        _setState(
          state.copyWith(
            isConnected: false,
            foregroundApp: null,
            errorMessage: 'PC正在关机',
          ),
        );
        break;
      case 'system_sleep':
        _setState(
          state.copyWith(
            isConnected: false,
            foregroundApp: null,
            errorMessage: 'PC已进入睡眠',
          ),
        );
        break;
    }
  }

  void _handleSessionStateChange(RemoteSessionState sessionState) {
    if (_disposed) return;

    // 在线状态由心跳活性宽限期驱动，与 socket 瞬时状态解耦：
    // ready → 在线；后台挂起 → 暂停判定；显式断开 → 立即离线；
    // 其余断线/重连 → 进入宽限期，宽限内恢复则保持在线。
    if (sessionState == RemoteSessionState.ready) {
      _onlineTracker.markOnline();
    } else if (sessionState == RemoteSessionState.suspended) {
      _onlineTracker.pause();
    } else if (sessionState == RemoteSessionState.closing) {
      _onlineTracker.onDisconnected(explicit: true);
    } else {
      _onlineTracker.onDisconnected(explicit: false);
    }

    final wsState = switch (sessionState) {
      RemoteSessionState.ready => WsConnectionState.connected,
      RemoteSessionState.connecting =>
        _session.isReconnectScheduled
            ? WsConnectionState.reconnecting
            : WsConnectionState.connecting,
      RemoteSessionState.authenticating => WsConnectionState.connecting,
      RemoteSessionState.disconnected ||
      RemoteSessionState.suspended ||
      RemoteSessionState.closing => WsConnectionState.disconnected,
    };
    final isConnected = sessionState == RemoteSessionState.ready;
    _setState(
      state.copyWith(
        wsState: wsState,
        isConnected: isConnected,
        pcOnline: _onlineTracker.isOnline,
        clearError: isConnected,
        errorMessage: wsState == WsConnectionState.reconnecting
            ? '连接已断开，正在重连'
            : state.errorMessage,
      ),
    );

    if (isConnected) {
      _session.setFastReconnect(enabled: false);
      _session.setHealthInterval(const Duration(seconds: 10));
      _expectOfflineTimer?.cancel();
      _expectOfflineTimer = null;
      _completeReadyWait();
      refreshSystemStatus();
      refreshForeground();
      getUnlockStatus();
      getSecureDesktopStatus();
      loadLauncherItems('app');
      loadLauncherItems('game');
    }
  }

  /// 睡眠/关机/重启等状态操作成功后，3s 内快速确认离线并进入快速感知模式。
  void expectPcOfflineSoon() {
    if (_disposed) return;
    _expectOfflineTimer?.cancel();
    _mouseFlushTimer?.cancel();
    _expectOfflineTimer = Timer(_expectOfflineDelay, () {
      _expectOfflineTimer = null;
      _markOfflineGenuine(force: true);
    });
  }

  /// 真实离线（宽限期到期或状态操作确认后）：切快速重连/心跳并通知上层。
  void _markOfflineGenuine({bool force = false}) {
    if (_disposed) return;
    if (force) {
      // 状态操作（睡眠/关机/重启）确认：即使 socket 尚未断开也直接判离线。
      _onlineTracker.onDisconnected(explicit: true);
    } else if (_session.isReady) {
      return;
    }
    _expectOfflineTimer?.cancel();
    _expectOfflineTimer = null;
    _session.setFastReconnect(enabled: true);
    _session.setHealthInterval(const Duration(seconds: 3));
    _setState(state.copyWith(pcOnline: false));
    if (!_offlineEventController.isClosed) {
      _offlineEventController.add(null);
    }
  }

  void stopWebSocket() {
    _session.disconnect();
    _setState(
      state.copyWith(
        isConnected: false,
        wsState: WsConnectionState.disconnected,
        pcOnline: false,
      ),
    );
  }

  Future<void> revokeCurrentToken() async {
    await _session.revokeToken();
  }

  void _ensureConnected() {
    final config = _ref.read(settingsProvider);
    if (config.baseUrl != null) {
      _session.configure(config.baseUrl!, config.savedToken);
      unawaited(_session.connect());
    }
  }

  void reconnectNow() {
    _session.reconnectNow();
  }

  Future<bool> waitForWebSocket({
    Duration timeout = const Duration(seconds: 12),
  }) async {
    if (_disposed) return false;
    if (_session.isReady) return true;

    _ensureConnected();
    if (_session.isReady) return true;

    // 由状态流通知就绪，避免 200ms 轮询。
    final completer = _readyCompleter ??= Completer<void>();
    try {
      await completer.future.timeout(timeout);
    } on TimeoutException {
      // 超时后返回当前连接状态，由调用方决定是否重试。
    }
    return _session.isReady;
  }

  void _completeReadyWait() {
    final completer = _readyCompleter;
    if (completer == null) return;
    _readyCompleter = null;
    if (!completer.isCompleted) completer.complete();
  }

  Future<Map<String, dynamic>> _wsRequest(
    Map<String, dynamic> payload, {
    Duration? timeout,
  }) async {
    if (!_session.isReady) {
      final connected = await waitForWebSocket();
      if (!connected) {
        throw StateError('无法连接到 PC，请检查网络和连接设置后重试');
      }
    }
    final response = await _session.request(payload, timeout: timeout);
    if (response['type'] == 'error') {
      throw RemoteCommandException.fromResponse(response);
    }
    return response;
  }

  Future<void> _wsSend(Map<String, dynamic> payload) async {
    if (!_session.isReady) {
      final connected = await waitForWebSocket();
      if (!connected) {
        throw StateError('无法连接到 PC，请检查网络和连接设置后重试');
      }
    }
    _session.send(payload);
  }

  Future<Map<String, dynamic>> _wsCommand(
    Map<String, dynamic> payload, {
    Duration? timeout,
  }) async {
    return _wsRequest(payload, timeout: timeout);
  }

  @override
  void dispose() {
    _disposed = true;
    _sessionEventSubscription?.cancel();
    _sessionStateSubscription?.cancel();
    stopWebSocket();
    _session.dispose();
    _expectOfflineTimer?.cancel();
    _expectOfflineTimer = null;
    _onlineTracker.dispose();
    _offlineEventController.close();
    WidgetsBinding.instance.removeObserver(_lifecycleObserver);
    super.dispose();
  }
}

/// 组合连接通道（基类）与动作分组（mixin）的具体 Notifier。
class RemoteControlNotifier extends _RemoteControlBase
    with
        _SystemStatusActions,
        _ProcessActions,
        _LauncherActions,
        _FileSystemActions,
        _PowerActions,
        _InputActions,
        _WindowActions {
  RemoteControlNotifier(super.ref);
}

class _LifecycleObserver extends WidgetsBindingObserver {
  final void Function(AppLifecycleState) onStateChanged;

  _LifecycleObserver(this.onStateChanged);

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    onStateChanged(state);
  }
}

final remoteControlProvider =
    StateNotifierProvider<RemoteControlNotifier, RemoteControlState>((ref) {
      return RemoteControlNotifier(ref);
    });
