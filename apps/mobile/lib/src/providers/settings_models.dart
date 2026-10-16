import '../utils/wake_on_lan.dart';

/// 「我的库」条目的**旧本地格式**。
///
/// 服务端已经是常用列表的唯一所有者；这个模型只用于把旧版本存在这台手机上的
/// 列表搬过去（见 `favorites_controller.dart` 的迁移逻辑），迁移完成后本地副本
/// 会被清空。
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

/// 首页快捷操作区中显示的快捷键标签，按用户加入顺序保存。
List<String> normalizeHomeShortcuts(Iterable<String> shortcuts) {
  final seen = <String>{};
  return shortcuts
      .map((item) => item.trim())
      .where((item) => item.isNotEmpty && seen.add(item))
      .take(12)
      .toList(growable: false);
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

/// 解析已解码的快捷键方案 JSON 值。
///
/// 入参不是列表、或所有条目都没有快捷键时回落到 [defaultShortcutPresets]，
/// 保证输入面板始终有可用方案。
List<ShortcutPreset> parseShortcutPresets(Object? raw) {
  if (raw is! List) return defaultShortcutPresets;
  final parsed = raw
      .whereType<Map>()
      .map((item) => ShortcutPreset.fromJson(Map<String, dynamic>.from(item)))
      .where((item) => item.shortcuts.isNotEmpty)
      .toList(growable: false);
  return parsed.isEmpty ? defaultShortcutPresets : parsed;
}

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

  /// 常用软件列表（**旧本地副本**，只作为一次迁移的输入）。
  final List<QuickApp> quickApps;

  /// 输入面板快捷键方案。
  final List<ShortcutPreset> shortcutPresets;

  /// 当前启用的快捷键方案标识。
  final String activeShortcutPresetId;

  /// 首页快捷操作区中的自定义组合键。
  final List<String> homeShortcuts;

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

  /// 本地「常用」是否已迁移到服务端。
  ///
  /// 旧版本把常用应用存在这台手机上；迁移后服务端是唯一所有者，本地副本只
  /// 作为一次性的迁移输入保留。
  final bool libraryMigrated;

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
    this.homeShortcuts = const [],
    this.lastUsedLauncherId,
    this.mouseSensitivity = defaultMouseSensitivity,
    this.reversePhoneScrollBar = false,
    this.autoUnlockOnWake = false,
    this.tvFollowPcPowerOn = false,
    this.tvFollowPcPowerOff = false,
    this.lockPasswordConfigured = false,
    this.libraryMigrated = false,
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
    List<String>? homeShortcuts,
    Object? lastUsedLauncherId = _unset,
    double? mouseSensitivity,
    bool? reversePhoneScrollBar,
    bool? autoUnlockOnWake,
    bool? tvFollowPcPowerOn,
    bool? tvFollowPcPowerOff,
    bool? lockPasswordConfigured,
    bool? libraryMigrated,
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
      homeShortcuts: homeShortcuts ?? this.homeShortcuts,
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
      libraryMigrated: libraryMigrated ?? this.libraryMigrated,
    );
  }

  /// 获取完整的 Base URL
  String? get baseUrl {
    if (pcIpAddress == null) return null;
    return 'http://$pcIpAddress:$pcPort';
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
    'homeShortcuts': homeShortcuts,
    'lastUsedLauncherId': lastUsedLauncherId,
    'mouseSensitivity': mouseSensitivity,
    'reversePhoneScrollBar': reversePhoneScrollBar,
    'autoUnlockOnWake': autoUnlockOnWake,
    'tvFollowPcPowerOn': tvFollowPcPowerOn,
    'tvFollowPcPowerOff': tvFollowPcPowerOff,
    'lockPasswordConfigured': lockPasswordConfigured,
    'libraryMigrated': libraryMigrated,
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
      shortcutPresets: parseShortcutPresets(json['shortcutPresets']),
      activeShortcutPresetId:
          json['activeShortcutPresetId'] as String? ?? 'common',
      homeShortcuts: normalizeHomeShortcuts(
        (json['homeShortcuts'] as List<dynamic>? ?? const []).map(
          (item) => item.toString(),
        ),
      ),
      lastUsedLauncherId: json['lastUsedLauncherId'] as String?,
      mouseSensitivity: normalizeMouseSensitivity(
        (json['mouseSensitivity'] as num?)?.toDouble(),
      ),
      reversePhoneScrollBar: json['reversePhoneScrollBar'] as bool? ?? false,
      autoUnlockOnWake: json['autoUnlockOnWake'] as bool? ?? false,
      tvFollowPcPowerOn: json['tvFollowPcPowerOn'] as bool? ?? false,
      tvFollowPcPowerOff: json['tvFollowPcPowerOff'] as bool? ?? false,
      lockPasswordConfigured: json['lockPasswordConfigured'] as bool? ?? false,
      libraryMigrated: json['libraryMigrated'] as bool? ?? false,
    );
  }
}
