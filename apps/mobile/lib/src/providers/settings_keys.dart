/// 设置项的 `SharedPreferences` 键名。
///
/// 键名是持久化契约：改名等同于丢设置，只允许新增。字符串值必须与历史版本
/// 完全一致。
class SettingsKeys {
  SettingsKeys._();

  static const String pcIpAddress = 'pc_ip_address';
  static const String pcId = 'pc_id';
  static const String pcName = 'pc_name';
  static const String pcPort = 'pc_port';
  static const String endpointMigration = 'endpoint_migration_v1';
  static const String macAddress = 'mac_address';
  static const String wakeNetworks = 'wake_networks';
  static const String isPaired = 'is_paired';
  static const String quickApps = 'quick_apps';
  static const String mouseSensitivity = 'mouse_sensitivity';
  static const String reversePhoneScrollBar = 'reverse_phone_scroll_bar';
  static const String autoUnlockOnWake = 'auto_unlock_on_wake';
  static const String tvFollowPcPowerOn = 'tv_follow_pc_power_on';
  static const String tvFollowPcPowerOff = 'tv_follow_pc_power_off';
  static const String shortcutPresets = 'shortcut_presets';
  static const String activeShortcutPreset = 'active_shortcut_preset';
  static const String lastUsedLauncher = 'last_used_launcher';
  static const String libraryMigrated = 'quick_apps_migrated_v1';
}
