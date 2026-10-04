import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../core/storage/preferences_repository.dart';
import '../services/secure_token_store.dart';
import '../utils/wake_on_lan.dart';
import 'settings_keys.dart';
import 'settings_models.dart';
import 'settings_persistence.dart';

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
  }) : _preferencesRepository = preferences,
       super(initialState) {
    if (loadPersisted) {
      _loadSettings();
    } else {
      _loadCompleter.complete();
    }
  }

  /// 设置加载完成的 Future
  Future<void> get loaded => _loadCompleter.future;

  /// 缓存的 SharedPreferences 实例（插件内部已是单例，这里避免每次重复 await）。
  Future<SharedPreferences> _getPrefs() => settingsPreferences();

  Future<void> _writeString(String key, String value) =>
      writeSettingsString(_preferencesRepository, key, value);

  /// Exposes the cached preferences for stable device identity and secure
  /// storage adapters owned by the application layer.
  Future<SharedPreferences> get preferences => _getPrefs();

  /// 安全更新状态，避免 notifier dispose 后触发断言
  void _setState(ConnectionConfig newState) {
    if (_disposed) return;
    state = newState;
  }

  Future<void> _loadSettings() async {
    final prefs = await _getPrefs();
    await applyEndpointMigration(prefs);
    final savedToken = await _tokenStore.read();
    final lockPassword = await _lockPasswordStore.read();
    _setState(
      readConnectionConfig(
        prefs,
        savedToken: savedToken,
        lockPasswordConfigured: lockPassword?.isNotEmpty == true,
      ),
    );
    if (!_loadCompleter.isCompleted) {
      _loadCompleter.complete();
    }
  }

  Future<void> saveShortcutPresets(List<ShortcutPreset> presets) async {
    final normalized = presets
        .where((preset) => preset.shortcuts.isNotEmpty)
        .toList(growable: false);
    await _writeString(
      SettingsKeys.shortcutPresets,
      jsonEncode(normalized.map((preset) => preset.toJson()).toList()),
    );
    _setState(state.copyWith(shortcutPresets: normalized));
  }

  Future<void> setActiveShortcutPreset(String id) async {
    if (!state.shortcutPresets.any((preset) => preset.id == id)) return;
    await _writeString(SettingsKeys.activeShortcutPreset, id);
    _setState(state.copyWith(activeShortcutPresetId: id));
  }

  Future<void> setLastUsedLauncher(String id) async {
    await _writeString(SettingsKeys.lastUsedLauncher, id);
    _setState(state.copyWith(lastUsedLauncherId: id));
  }

  Future<void> _persistLibrary(List<QuickApp> apps) async {
    await _writeString(
      SettingsKeys.quickApps,
      jsonEncode(apps.map((a) => a.toJson()).toList()),
    );
    _setState(state.copyWith(quickApps: apps));
  }

  /// 清空本地「常用」副本。
  ///
  /// 服务端已经是唯一所有者，本地副本只在下述迁移里被读过一次；迁移完成后
  /// 必须删掉，否则两端会出现两份互相矛盾的列表。
  Future<void> clearLocalLibrary() async {
    if (state.quickApps.isEmpty) return;
    await _persistLibrary(const []);
  }

  /// 记录本地「常用」已迁移到服务端，迁移只会执行一次。
  Future<void> markLibraryMigrated() async {
    final prefs = await _getPrefs();
    await prefs.setBool(SettingsKeys.libraryMigrated, true);
    _setState(state.copyWith(libraryMigrated: true));
  }

  Future<void> saveConnectionSettings({
    required String ipAddress,
    int port = 3721,
    String? macAddress,
  }) async {
    final prefs = await _getPrefs();
    await prefs.setString(SettingsKeys.pcIpAddress, ipAddress);
    await prefs.setInt(SettingsKeys.pcPort, port);
    if (macAddress != null) {
      await prefs.setString(SettingsKeys.macAddress, macAddress);
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
    await prefs.setBool(SettingsKeys.isPaired, true);
    _setState(state.copyWith(savedToken: token, isPaired: true));
  }

  Future<void> savePcIdentity(String pcId) async {
    final prefs = await _getPrefs();
    await prefs.setString(SettingsKeys.pcId, pcId);
    _setState(state.copyWith(pcId: pcId));
  }

  Future<void> applyDiscoveredEndpoint({
    required String pcId,
    String? pcName,
    required String address,
    required int port,
  }) async {
    final prefs = await _getPrefs();
    await prefs.setString(SettingsKeys.pcId, pcId);
    if (pcName != null && pcName.isNotEmpty) {
      await prefs.setString(SettingsKeys.pcName, pcName);
    }
    await prefs.setString(SettingsKeys.pcIpAddress, address);
    await prefs.setInt(SettingsKeys.pcPort, port);
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
    await prefs.setBool(SettingsKeys.isPaired, false);
    _setState(state.copyWith(savedToken: null, isPaired: false));
  }

  Future<void> updateMacAddress(String macAddress) async {
    final prefs = await _getPrefs();
    await prefs.setString(SettingsKeys.macAddress, macAddress);
    _setState(state.copyWith(macAddress: macAddress));
  }

  Future<void> updateWakeNetworks(List<WakeNetwork> networks) async {
    final prefs = await _getPrefs();
    await prefs.setString(
      SettingsKeys.wakeNetworks,
      jsonEncode(networks.map((e) => e.toJson()).toList()),
    );
    _setState(
      state.copyWith(wakeNetworks: List<WakeNetwork>.unmodifiable(networks)),
    );
  }

  Future<void> saveMouseSensitivity(double value) async {
    final sensitivity = ConnectionConfig.normalizeMouseSensitivity(value);
    final prefs = await _getPrefs();
    await prefs.setDouble(SettingsKeys.mouseSensitivity, sensitivity);
    _setState(state.copyWith(mouseSensitivity: sensitivity));
  }

  Future<void> setReversePhoneScrollBar(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(SettingsKeys.reversePhoneScrollBar, enabled);
    _setState(state.copyWith(reversePhoneScrollBar: enabled));
  }

  Future<void> setAutoUnlockOnWake(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(SettingsKeys.autoUnlockOnWake, enabled);
    _setState(state.copyWith(autoUnlockOnWake: enabled));
  }

  Future<void> setTvFollowPcPowerOn(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(SettingsKeys.tvFollowPcPowerOn, enabled);
    _setState(state.copyWith(tvFollowPcPowerOn: enabled));
  }

  Future<void> setTvFollowPcPowerOff(bool enabled) async {
    final prefs = await _getPrefs();
    await prefs.setBool(SettingsKeys.tvFollowPcPowerOff, enabled);
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
