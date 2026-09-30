import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../services/secure_token_store.dart';
import '../utils/wake_on_lan.dart';
import '../core/storage/preferences_repository.dart';

/// 「我的库」条目：从「全部」添加进常用，或从文件树手动添加。
class QuickApp {
  final String name;
  final String path;
  final String? scannerId;
  final bool isGame;

  /// 原始名称，用于服务端重命名匹配。
  final String? originalName;

  /// 是否置顶到首页（首页「常用应用」只显示置顶项）。
  final bool pinnedToHome;

  /// 启动时附加的命令行参数。
  final List<String> args;

  const QuickApp({
    required this.name,
    required this.path,
    this.scannerId,
    this.isGame = false,
    this.originalName,
    this.pinnedToHome = true,
    this.args = const [],
  });

  /// 稳定标识：优先 scannerId，其次路径。
  String get id => scannerId ?? path;

  QuickApp copyWith({
    String? name,
    String? path,
    String? scannerId,
    bool? isGame,
    String? originalName,
    bool? pinnedToHome,
    List<String>? args,
  }) => QuickApp(
    name: name ?? this.name,
    path: path ?? this.path,
    scannerId: scannerId ?? this.scannerId,
    isGame: isGame ?? this.isGame,
    originalName: originalName ?? this.originalName,
    pinnedToHome: pinnedToHome ?? this.pinnedToHome,
    args: args ?? this.args,
  );

  Map<String, dynamic> toJson() => {
    'name': name,
    'path': path,
    if (scannerId != null) 'scanner_id': scannerId,
    'is_game': isGame,
    if (originalName != null) 'original_name': originalName,
    'pinned_to_home': pinnedToHome,
    if (args.isNotEmpty) 'args': args,
  };

  factory QuickApp.fromJson(Map<String, dynamic> json) {
    return QuickApp(
      name: json['name'] as String,
      path: json['path'] as String,
      scannerId: json['scanner_id'] as String?,
      isGame: json['is_game'] as bool? ?? false,
      originalName: json['original_name'] as String? ?? json['name'] as String?,
      pinnedToHome: json['pinned_to_home'] as bool? ?? true,
      args:
          (json['args'] as List<dynamic>?)
              ?.map((value) => value.toString())
              .toList() ??
          const [],
    );
  }
}

/// 手机端输入面板使用的快捷键方案。
class ShortcutPreset {
  final String id;
  final String name;
  final List<String> shortcuts;
  final bool builtin;

  const ShortcutPreset({
    required this.id,
    required this.name,
    required this.shortcuts,
    this.builtin = false,
  });

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'shortcuts': shortcuts,
    'builtin': builtin,
  };

  factory ShortcutPreset.fromJson(Map<String, dynamic> json) => ShortcutPreset(
    id: json['id'] as String? ?? 'custom',
    name: json['name'] as String? ?? '自定义',
    shortcuts: (json['shortcuts'] as List<dynamic>? ?? const [])
        .map((item) => item.toString())
        .where((item) => item.isNotEmpty)
        .take(12)
        .toList(growable: false),
    builtin: json['builtin'] as bool? ?? false,
  );
}

const defaultShortcutPresets = <ShortcutPreset>[
  ShortcutPreset(
    id: 'common',
    name: '常用',
    shortcuts: ['Tab', 'Esc', '↑', '↓', '←', '→', 'Ctrl + C', 'Ctrl + V'],
    builtin: true,
  ),
  ShortcutPreset(
    id: 'opencode',
    name: 'OpenCode',
    shortcuts: ['Ctrl + K', 'Ctrl + L', 'Ctrl + Enter', 'Esc'],
    builtin: true,
  ),
  ShortcutPreset(
    id: 'codex',
    name: 'Codex',
    shortcuts: ['Ctrl + K', 'Ctrl + Enter', 'Alt + Enter', 'Esc'],
    builtin: true,
  ),
];

/// 连接配置
class ConnectionConfig {
  static const Object _unset = Object();
  static const double defaultMouseSensitivity = 4;
  static const double minMouseSensitivity = 1;
  static const double maxMouseSensitivity = 8;

  /// PC 的 IP 地址
  final String? pcIpAddress;

  /// UDP 发现返回的稳定 PC 标识。
  final String? pcId;

  /// PC 的显示名称。
  final String? pcName;

  /// PC 的端口
  final int pcPort;

  /// PC 的 MAC 地址（用于 Wake-on-LAN）
  final String? macAddress;
  final List<WakeNetwork> wakeNetworks;

  /// 保存的 Token
  final String? savedToken;

  /// 是否已配对
  final bool isPaired;

  /// 常用软件列表
  final List<QuickApp> quickApps;

  /// 输入面板快捷键方案。
  final List<ShortcutPreset> shortcutPresets;

  /// 当前启用的快捷键方案标识。
  final String activeShortcutPresetId;

  /// 上次启动的启动器条目标识。
  final String? lastUsedLauncherId;

  /// 鼠标触控板移动倍率
  final double mouseSensitivity;

  /// 手机滚动条是否使用 macOS 自然滚动方向
  final bool reversePhoneScrollBar;

  /// 唤醒后是否自动尝试确认解锁
  final bool autoUnlockOnWake;

  /// 发送唤醒时是否同步打开电视（当前遥控器的红外电源键）。
  final bool tvFollowPcPowerOn;

  /// 电脑关机/睡眠时是否同步关闭电视（当前遥控器的红外电源键）。
  final bool tvFollowPcPowerOff;

  /// 是否已在安全存储中配置 Windows 登录密码。
  final bool lockPasswordConfigured;

  const ConnectionConfig({
    this.pcIpAddress,
    this.pcId,
    this.pcName,
    this.pcPort = 3721,
    this.macAddress,
    this.wakeNetworks = const [],
    this.savedToken,
    this.isPaired = false,
    this.quickApps = const [],
    this.shortcutPresets = defaultShortcutPresets,
    this.activeShortcutPresetId = 'common',
    this.lastUsedLauncherId,
    this.mouseSensitivity = defaultMouseSensitivity,
    this.reversePhoneScrollBar = false,
    this.autoUnlockOnWake = false,
    this.tvFollowPcPowerOn = false,
    this.tvFollowPcPowerOff = false,
    this.lockPasswordConfigured = false,
  });

  static double normalizeMouseSensitivity(double? value) {
    return (value ?? defaultMouseSensitivity)
        .clamp(minMouseSensitivity, maxMouseSensitivity)
        .toDouble();
  }

  ConnectionConfig copyWith({
    String? pcIpAddress,
    String? pcId,
    String? pcName,
    int? pcPort,
    String? macAddress,
    List<WakeNetwork>? wakeNetworks,
    Object? savedToken = _unset,
    bool? isPaired,
    List<QuickApp>? quickApps,
    List<ShortcutPreset>? shortcutPresets,
    String? activeShortcutPresetId,
    Object? lastUsedLauncherId = _unset,
    double? mouseSensitivity,
    bool? reversePhoneScrollBar,
    bool? autoUnlockOnWake,
    bool? tvFollowPcPowerOn,
    bool? tvFollowPcPowerOff,
    bool? lockPasswordConfigured,
  }) {
    return ConnectionConfig(
      pcIpAddress: pcIpAddress ?? this.pcIpAddress,
      pcId: pcId ?? this.pcId,
      pcName: pcName ?? this.pcName,
      pcPort: pcPort ?? this.pcPort,
      macAddress: macAddress ?? this.macAddress,
      wakeNetworks: wakeNetworks ?? this.wakeNetworks,
      savedToken: identical(savedToken, _unset)
          ? this.savedToken
          : savedToken as String?,
      isPaired: isPaired ?? this.isPaired,
      quickApps: quickApps ?? this.quickApps,
      shortcutPresets: shortcutPresets ?? this.shortcutPresets,
      activeShortcutPresetId:
          activeShortcutPresetId ?? this.activeShortcutPresetId,
      lastUsedLauncherId: identical(lastUsedLauncherId, _unset)
          ? this.lastUsedLauncherId
          : lastUsedLauncherId as String?,
      mouseSensitivity: mouseSensitivity ?? this.mouseSensitivity,
      reversePhoneScrollBar:
          reversePhoneScrollBar ?? this.reversePhoneScrollBar,
      autoUnlockOnWake: autoUnlockOnWake ?? this.autoUnlockOnWake,
      tvFollowPcPowerOn: tvFollowPcPowerOn ?? this.tvFollowPcPowerOn,
      tvFollowPcPowerOff: tvFollowPcPowerOff ?? this.tvFollowPcPowerOff,
      lockPasswordConfigured:
          lockPasswordConfigured ?? this.lockPasswordConfigured,
    );
  }

  /// 获取完整的 Base URL
  String? get baseUrl {
    if (pcIpAddress == null) return null;
    return 'http://$pcIpAddress:$pcPort';
  }

  /// 是否已在「我的库」（常用）中。
  bool isInLibrary(String id) =>
      quickApps.any((a) => a.id == id || a.scannerId == id);

  /// 是否已置顶到首页。
  bool isPinnedToHome(String id) =>
      quickApps.any((a) => a.id == id && a.pinnedToHome);

  QuickApp? libraryApp(String id) {
    for (final app in quickApps) {
      if (app.id == id || app.scannerId == id) return app;
    }
    return null;
  }

  Map<String, dynamic> toJson() => {
    'pcIpAddress': pcIpAddress,
    'pcId': pcId,
    'pcName': pcName,
    'pcPort': pcPort,
    'macAddress': macAddress,
    'wakeNetworks': wakeNetworks.map((e) => e.toJson()).toList(),
    'isPaired': isPaired,
    'quickApps': quickApps.map((a) => a.toJson()).toList(),
    'shortcutPresets': shortcutPresets
        .map((preset) => preset.toJson())
        .toList(),
    'activeShortcutPresetId': activeShortcutPresetId,
    'lastUsedLauncherId': lastUsedLauncherId,
    'mouseSensitivity': mouseSensitivity,
    'reversePhoneScrollBar': reversePhoneScrollBar,
    'autoUnlockOnWake': autoUnlockOnWake,
    'tvFollowPcPowerOn': tvFollowPcPowerOn,
    'tvFollowPcPowerOff': tvFollowPcPowerOff,
    'lockPasswordConfigured': lockPasswordConfigured,
  };

  factory ConnectionConfig.fromJson(Map<String, dynamic> json) {
    return ConnectionConfig(
      pcIpAddress: json['pcIpAddress'] as String?,
      pcId: json['pcId'] as String?,
      pcName: json['pcName'] as String?,
      pcPort: json['pcPort'] as int? ?? 3721,
      macAddress: json['macAddress'] as String?,
      wakeNetworks: (json['wakeNetworks'] as List<dynamic>? ?? const [])
          .whereType<Map>()
          .map((e) => WakeNetwork.fromJson(Map<String, dynamic>.from(e)))
          .toList(growable: false),
      savedToken: null,
      isPaired: json['isPaired'] as bool? ?? false,
      quickApps:
          (json['quickApps'] as List<dynamic>?)
              ?.map((e) => QuickApp.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
      shortcutPresets: _parseShortcutPresets(json['shortcutPresets']),
      activeShortcutPresetId:
          json['activeShortcutPresetId'] as String? ?? 'common',
      lastUsedLauncherId: json['lastUsedLauncherId'] as String?,
      mouseSensitivity: normalizeMouseSensitivity(
        (json['mouseSensitivity'] as num?)?.toDouble(),
      ),
      reversePhoneScrollBar: json['reversePhoneScrollBar'] as bool? ?? false,
      autoUnlockOnWake: json['autoUnlockOnWake'] as bool? ?? false,
      tvFollowPcPowerOn: json['tvFollowPcPowerOn'] as bool? ?? false,
      tvFollowPcPowerOff: json['tvFollowPcPowerOff'] as bool? ?? false,
      lockPasswordConfigured: json['lockPasswordConfigured'] as bool? ?? false,
    );
  }

  static List<ShortcutPreset> _parseShortcutPresets(Object? raw) {
    if (raw is! List) return defaultShortcutPresets;
    final parsed = raw
        .whereType<Map>()
        .map((item) => ShortcutPreset.fromJson(Map<String, dynamic>.from(item)))
        .where((item) => item.shortcuts.isNotEmpty)
        .toList(growable: false);
    return parsed.isEmpty ? defaultShortcutPresets : parsed;
  }
}

List<WakeNetwork> _parseWakeNetworks(String? raw) {
  if (raw == null || raw.isEmpty) return const [];
  try {
    final decoded = jsonDecode(raw);
    if (decoded is! List) return const [];
    return decoded
        .whereType<Map>()
        .map((item) => WakeNetwork.fromJson(Map<String, dynamic>.from(item)))
        .toList(growable: false);
  } catch (_) {
    return const [];
  }
}

/// 设置管理 Provider
class SettingsNotifier extends StateNotifier<ConnectionConfig> {
  bool _disposed = false;
  final SecureTokenStore _tokenStore;
  final LockPasswordStore _lockPasswordStore;
  final PreferencesRepository? _preferencesRepository;
  final Completer<void> _loadCompleter = Completer<void>();

  SettingsNotifier({
    ConnectionConfig initialState = const ConnectionConfig(),
    bool loadPersisted = true,
    this._tokenStore = const PlatformSecureTokenStore(),
    this._lockPasswordStore = const PlatformLockPasswordStore(),
    PreferencesRepository? preferences,
  }) : _preferencesRepository = preferences, super(initialState) {
    if (loadPersisted) {
      _loadSettings();
    } else {
      _loadCompleter.complete();
    }
  }

  /// 设置加载完成的 Future
  Future<void> get loaded => _loadCompleter.future;

  /// 缓存的 SharedPreferences 实例（插件内部已是单例，这里避免每次重复 await）。
  Future<SharedPreferences> _getPrefs() async => SharedPreferences.getInstance();

  Future<void> _writeString(String key, String value) async {
    final repository = _preferencesRepository;
    if (repository != null) {
      await repository.writeString(key, value);
      return;
    }
    await (await _getPrefs()).setString(key, value);
  }

  /// Exposes the cached preferences for stable device identity and secure
  /// storage adapters owned by the application layer.
  Future<SharedPreferences> get preferences => _getPrefs();

  /// 安全更新状态，避免 notifier dispose 后触发断言
  void _setState(ConnectionConfig newState) {
    if (_disposed) return;
    state = newState;
  }

  static const String _keyPcIpAddress = 'pc_ip_address';
  static const String _keyPcId = 'pc_id';
  static const String _keyPcName = 'pc_name';
  static const String _keyPcPort = 'pc_port';
  static const String _keyEndpointMigration = 'endpoint_migration_v1';
  static const String _keyMacAddress = 'mac_address';
  static const String _keyWakeNetworks = 'wake_networks';
  static const String _keyIsPaired = 'is_paired';
  static const String _keyQuickApps = 'quick_apps';
  static const String _keyMouseSensitivity = 'mouse_sensitivity';
  static const String _keyReversePhoneScrollBar = 'reverse_phone_scroll_bar';
  static const String _keyAutoUnlockOnWake = 'auto_unlock_on_wake';
  static const String _keyTvFollowPcPowerOn = 'tv_follow_pc_power_on';
  static const String _keyTvFollowPcPowerOff = 'tv_follow_pc_power_off';
  static const String _keyShortcutPresets = 'shortcut_presets';
  static const String _keyActiveShortcutPreset = 'active_shortcut_preset';
  static const String _keyLastUsedLauncher = 'last_used_launcher';

  Future<void> _loadSettings() async {
    final prefs = await _getPrefs();
    if (!(prefs.getBool(_keyEndpointMigration) ?? false)) {
      // IP/port values from the old manual-configuration flow must not be
      // treated as authoritative. A fresh UDP discovery will repopulate the
      // endpoint, while the stable PC id, token, and MAC remain intact.
      await prefs.remove(_keyPcIpAddress);
      await prefs.remove(_keyPcPort);
      await prefs.setBool(_keyEndpointMigration, true);
    }
    final savedToken = await _tokenStore.read();
    final lockPassword = await _lockPasswordStore.read();
    _setState(
      ConnectionConfig(
        pcIpAddress: prefs.getString(_keyPcIpAddress),
        pcId: prefs.getString(_keyPcId),
        pcName: prefs.getString(_keyPcName),
        pcPort: prefs.getInt(_keyPcPort) ?? 3721,
        macAddress: prefs.getString(_keyMacAddress),
        wakeNetworks: _parseWakeNetworks(prefs.getString(_keyWakeNetworks)),
        savedToken: savedToken,
        isPaired: savedToken != null && savedToken.isNotEmpty,
        quickApps: _parseQuickApps(prefs.getString(_keyQuickApps)),
        shortcutPresets: _parseShortcutPresets(
          prefs.getString(_keyShortcutPresets),
        ),
        activeShortcutPresetId:
            prefs.getString(_keyActiveShortcutPreset) ?? 'common',
        lastUsedLauncherId: prefs.getString(_keyLastUsedLauncher),
        mouseSensitivity: ConnectionConfig.normalizeMouseSensitivity(
          prefs.getDouble(_keyMouseSensitivity),
        ),
        reversePhoneScrollBar:
            prefs.getBool(_keyReversePhoneScrollBar) ?? false,
        autoUnlockOnWake: prefs.getBool(_keyAutoUnlockOnWake) ?? false,
        tvFollowPcPowerOn: prefs.getBool(_keyTvFollowPcPowerOn) ?? false,
        tvFollowPcPowerOff: prefs.getBool(_keyTvFollowPcPowerOff) ?? false,
        lockPasswordConfigured: lockPassword?.isNotEmpty == true,
      ),
    );
    if (!_loadCompleter.isCompleted) {
      _loadCompleter.complete();
    }
  }

  List<QuickApp> _parseQuickApps(String? jsonStr) {
    if (jsonStr == null || jsonStr.isEmpty) return [];
    try {
      final list = jsonDecode(jsonStr) as List<dynamic>;
      return list
          .map((e) => QuickApp.fromJson(e as Map<String, dynamic>))
          .toList();
    } catch (_) {
      return [];
    }
  }

  List<ShortcutPreset> _parseShortcutPresets(String? jsonStr) {
    if (jsonStr == null || jsonStr.isEmpty) return defaultShortcutPresets;
    try {
      return ConnectionConfig._parseShortcutPresets(jsonDecode(jsonStr));
    } catch (_) {
      return defaultShortcutPresets;
    }
  }

  Future<void> saveShortcutPresets(List<ShortcutPreset> presets) async {
    final normalized = presets
        .where((preset) => preset.shortcuts.isNotEmpty)
        .toList(growable: false);
    await _writeString(
      _keyShortcutPresets,
      jsonEncode(normalized.map((preset) => preset.toJson()).toList()),
    );
    _setState(state.copyWith(shortcutPresets: normalized));
  }

  Future<void> setActiveShortcutPreset(String id) async {
    if (!state.shortcutPresets.any((preset) => preset.id == id)) return;
    await _writeString(_keyActiveShortcutPreset, id);
    _setState(state.copyWith(activeShortcutPresetId: id));
  }

  Future<void> setLastUsedLauncher(String id) async {
    await _writeString(_keyLastUsedLauncher, id);
    _setState(state.copyWith(lastUsedLauncherId: id));
  }

  Future<void> _persistLibrary(List<QuickApp> apps) async {
    await _writeString(
      _keyQuickApps,
      jsonEncode(apps.map((a) => a.toJson()).toList()),
    );
    _setState(state.copyWith(quickApps: apps));
  }

  /// 加入「我的库」（常用）。已存在则忽略。
  Future<void> addToLibrary(QuickApp app) async {
    if (state.isInLibrary(app.id)) return;
    await _persistLibrary([...state.quickApps, app]);
  }

  /// 从「我的库」移除。
  Future<void> removeFromLibrary(String id) async {
    final updated = state.quickApps
        .where((a) => a.id != id && a.scannerId != id)
        .toList();
    if (updated.length != state.quickApps.length) {
      await _persistLibrary(updated);
    }
  }

  /// 置顶/取消置顶到首页。
  Future<void> setPinnedToHome(String id, bool pinned) async {
    final updated = state.quickApps
        .map(
          (a) => (a.id == id || a.scannerId == id)
              ? a.copyWith(pinnedToHome: pinned)
              : a,
        )
        .toList();
    await _persistLibrary(updated);
  }

  /// 重命名库中条目（仅本地显示名）。
  Future<void> renameLibraryApp(String id, String name) async {
    final updated = state.quickApps
        .map(
          (a) => (a.id == id || a.scannerId == id) ? a.copyWith(name: name) : a,
        )
        .toList();
    await _persistLibrary(updated);
  }

  /// 更新库中条目的启动参数（仅本地副本，PC 端另行更新）。
  Future<void> setLibraryArgs(String id, List<String> args) async {
    final updated = state.quickApps
        .map(
          (a) => (a.id == id || a.scannerId == id) ? a.copyWith(args: args) : a,
        )
        .toList();
    await _persistLibrary(updated);
  }

  Future<void> saveConnectionSettings({
    required String ipAddress,
    int port = 3721,
    String? macAddress,
  }) async {
    final prefs = await _getPrefs();
    await prefs.setString(_keyPcIpAddress, ipAddress);
    await prefs.setInt(_keyPcPort, port);
    if (macAddress != null) {
      await prefs.setString(_keyMacAddress, macAddress);
    }
    // 保留已有字段（savedToken, isPaired, quickApps）
    _setState(
      state.copyWith(
        pcIpAddress: ipAddress,
        pcPort: port,
        macAddress: macAddress ?? state.macAddress,
      ),
    );
  }

  Future<void> saveToken(String token) async {
    final prefs = await _getPrefs();
    await _tokenStore.write(token);
    await prefs.setBool(_keyIsPaired, true);
    _setState(state.copyWith(savedToken: token, isPaired: true));
  }

  Future<void> savePcIdentity(String pcId) async {
    final prefs = await _getPrefs();
    await prefs.setString(_keyPcId, pcId);
    _setState(state.copyWith(pcId: pcId));
  }

  Future<void> applyDiscoveredEndpoint({
    required String pcId,
    String? pcName,
    required String address,
    required int port,
  }) async {
    final prefs = await _getPrefs();
    await prefs.setString(_keyPcId, pcId);
    if (pcName != null && pcName.isNotEmpty) {
      await prefs.setString(_keyPcName, pcName);
    }
    await prefs.setString(_keyPcIpAddress, address);
    await prefs.setInt(_keyPcPort, port);
    _setState(
      state.copyWith(
        pcId: pcId,
        pcName: pcName,
        pcIpAddress: address,
        pcPort: port,
      ),
    );
  }

  Future<void> clearToken() async {
    final prefs = await _getPrefs();
    await _tokenStore.clear();
    await prefs.setBool(_keyIsPaired, false);
    _setState(state.copyWith(savedToken: null, isPaired: false));
  }

  Future<void> updateMacAddress(String macAddress) async {
    final prefs = await _getPrefs();
    await prefs.setString(_keyMacAddress, macAddress);
    _setState(state.copyWith(macAddress: macAddress));
  }

  Future<void> updateWakeNetworks(List<WakeNetwork> networks) async {
    final prefs = await _getPrefs();
    await prefs.setString(
      _keyWakeNetworks,
      jsonEncode(networks.map((e) => e.toJson()).toList()),
    );
    _setState(
      state.copyWith(wakeNetworks: List<WakeNetwork>.unmodifiable(networks)),
    );
  }

  Future<void> saveMouseSensitivity(double value) async {
    final sensitivity = ConnectionConfig.normalizeMouseSensitivity(value);
    final prefs = await _getPrefs();
    await prefs.setDouble(_keyMouseSensitivity, sensitivity);
    _setState(state.copyWith(mouseSensitivity: sensitivity));
  }

  Future<void> setReversePhoneScrollBar(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(_keyReversePhoneScrollBar, enabled);
    _setState(state.copyWith(reversePhoneScrollBar: enabled));
  }

  Future<void> setAutoUnlockOnWake(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(_keyAutoUnlockOnWake, enabled);
    _setState(state.copyWith(autoUnlockOnWake: enabled));
  }

  Future<void> setTvFollowPcPowerOn(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(_keyTvFollowPcPowerOn, enabled);
    _setState(state.copyWith(tvFollowPcPowerOn: enabled));
  }

  Future<void> setTvFollowPcPowerOff(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(_keyTvFollowPcPowerOff, enabled);
    _setState(state.copyWith(tvFollowPcPowerOff: enabled));
  }

  Future<String?> readLockPassword() => _lockPasswordStore.read();

  Future<void> saveLockPassword(String password) async {
    final normalized = password.trim();
    if (!RegExp(r'^\d{4,32}$').hasMatch(normalized)) {
      throw ArgumentError('Windows 登录 PIN 必须是 4-32 位数字');
    }
    await _lockPasswordStore.write(normalized);
    _setState(state.copyWith(lockPasswordConfigured: true));
  }

  Future<void> clearLockPassword() async {
    await _lockPasswordStore.clear();
    _setState(state.copyWith(lockPasswordConfigured: false));
  }

  Future<void> clearAll() async {
    final prefs = await _getPrefs();
    await _tokenStore.clear();
    await _lockPasswordStore.clear();
    await prefs.clear();
    _setState(const ConnectionConfig());
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}

/// 设置 Provider
final settingsProvider =
    StateNotifierProvider<SettingsNotifier, ConnectionConfig>((ref) {
      return SettingsNotifier();
    });
