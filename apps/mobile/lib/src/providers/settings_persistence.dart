/// 设置项的本地持久化访问层。
///
/// 这一层只负责「键名 → `SharedPreferences` / [PreferencesRepository] → 强类型值」
/// 的读写与旧数据解析，不持有任何状态；状态的更新由 `SettingsNotifier` 完成。
/// 键名与解析规则都是持久化契约，改动会导致用户设置丢失。
library;

import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import '../core/storage/preferences_repository.dart';
import '../utils/wake_on_lan.dart';
import 'settings_keys.dart';
import 'settings_models.dart';

/// 缓存的 SharedPreferences 实例（插件内部已是单例，这里避免每次重复 await）。
Future<SharedPreferences> settingsPreferences() =>
    SharedPreferences.getInstance();

/// 写入单个字符串设置项。
///
/// 注入 [repository] 时优先走仓储（应用层可控的持久化边界），否则退回插件 API。
Future<void> writeSettingsString(
  PreferencesRepository? repository,
  String key,
  String value,
) async {
  if (repository != null) {
    await repository.writeString(key, value);
    return;
  }
  await (await settingsPreferences()).setString(key, value);
}

/// 一次性端点迁移：丢弃旧手工配置流程写入的 IP / 端口。
///
/// 标记位写回后不再重复执行；稳定 PC id、token 与 MAC 都保留。
Future<void> applyEndpointMigration(SharedPreferences prefs) async {
  if (prefs.getBool(SettingsKeys.endpointMigration) ?? false) {
    return;
  }
  // IP/port values from the old manual-configuration flow must not be
  // treated as authoritative. A fresh UDP discovery will repopulate the
  // endpoint, while the stable PC id, token, and MAC remain intact.
  await prefs.remove(SettingsKeys.pcIpAddress);
  await prefs.remove(SettingsKeys.pcPort);
  await prefs.setBool(SettingsKeys.endpointMigration, true);
}

/// 从已加载的 [prefs] 还原 [ConnectionConfig]。
///
/// [savedToken] 来自安全存储，[lockPasswordConfigured] 表示安全存储中已配置
/// Windows 登录密码；两者都不进 `SharedPreferences`。
ConnectionConfig readConnectionConfig(
  SharedPreferences prefs, {
  required String? savedToken,
  required bool lockPasswordConfigured,
}) {
  return ConnectionConfig(
    pcIpAddress: prefs.getString(SettingsKeys.pcIpAddress),
    pcId: prefs.getString(SettingsKeys.pcId),
    pcName: prefs.getString(SettingsKeys.pcName),
    pcPort: prefs.getInt(SettingsKeys.pcPort) ?? 3721,
    macAddress: prefs.getString(SettingsKeys.macAddress),
    wakeNetworks: parseStoredWakeNetworks(
      prefs.getString(SettingsKeys.wakeNetworks),
    ),
    savedToken: savedToken,
    isPaired: savedToken != null && savedToken.isNotEmpty,
    quickApps: parseStoredQuickApps(prefs.getString(SettingsKeys.quickApps)),
    shortcutPresets: parseStoredShortcutPresets(
      prefs.getString(SettingsKeys.shortcutPresets),
    ),
    activeShortcutPresetId:
        prefs.getString(SettingsKeys.activeShortcutPreset) ?? 'common',
    lastUsedLauncherId: prefs.getString(SettingsKeys.lastUsedLauncher),
    mouseSensitivity: ConnectionConfig.normalizeMouseSensitivity(
      prefs.getDouble(SettingsKeys.mouseSensitivity),
    ),
    reversePhoneScrollBar:
        prefs.getBool(SettingsKeys.reversePhoneScrollBar) ?? false,
    autoUnlockOnWake: prefs.getBool(SettingsKeys.autoUnlockOnWake) ?? false,
    tvFollowPcPowerOn: prefs.getBool(SettingsKeys.tvFollowPcPowerOn) ?? false,
    tvFollowPcPowerOff: prefs.getBool(SettingsKeys.tvFollowPcPowerOff) ?? false,
    lockPasswordConfigured: lockPasswordConfigured,
    libraryMigrated: prefs.getBool(SettingsKeys.libraryMigrated) ?? false,
  );
}

/// 解析本地存储的「我的库」旧副本；缺失或损坏时返回空列表。
List<QuickApp> parseStoredQuickApps(String? jsonStr) {
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

/// 解析本地存储的快捷键方案；缺失或损坏时回落到内置方案。
List<ShortcutPreset> parseStoredShortcutPresets(String? jsonStr) {
  if (jsonStr == null || jsonStr.isEmpty) return defaultShortcutPresets;
  try {
    return parseShortcutPresets(jsonDecode(jsonStr));
  } catch (_) {
    return defaultShortcutPresets;
  }
}

/// 解析本地存储的 Wake-on-LAN 网络配置；缺失或损坏时返回空列表。
List<WakeNetwork> parseStoredWakeNetworks(String? raw) {
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
