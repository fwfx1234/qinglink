class DisplayModeInfo {
  final int width;
  final int height;
  final int refreshMillihz;

  const DisplayModeInfo({
    required this.width,
    required this.height,
    required this.refreshMillihz,
  });

  factory DisplayModeInfo.fromJson(Map<String, dynamic> json) =>
      DisplayModeInfo(
        width: (json['width'] as num).toInt(),
        height: (json['height'] as num).toInt(),
        refreshMillihz: (json['refresh_millihz'] as num?)?.toInt() ?? 0,
      );

  String get resolution => '${width}x$height';
  String get refreshText {
    if (refreshMillihz == 0) return '';
    final precision = refreshMillihz % 1000 == 0 ? 0 : 2;
    return '${(refreshMillihz / 1000).toStringAsFixed(precision)} Hz';
  }

  String get label => refreshText.isEmpty
      ? resolution
      : '$resolution · $refreshText';

  /// 两条模式是否完全相同（宽、高、刷新率逐项相等）。
  bool equals(DisplayModeInfo other) =>
      width == other.width &&
      height == other.height &&
      refreshMillihz == other.refreshMillihz;

  Map<String, dynamic> toJson() => {
    'width': width,
    'height': height,
    'refresh_millihz': refreshMillihz,
  };
}

class DisplayInfo {
  final String id;
  final String name;
  final bool primary;
  final DisplayModeInfo currentMode;
  final List<DisplayModeInfo> modes;

  const DisplayInfo({
    required this.id,
    required this.name,
    required this.primary,
    required this.currentMode,
    required this.modes,
  });

  factory DisplayInfo.fromJson(Map<String, dynamic> json) {
    // 旧版服务端可能不返回 modes（当时只给三种裁剪过的候选），字段缺失一律按空处理，
    // 让界面显示"没有可选的分辨率"而不是整页解析失败。
    final rawModes = json['modes'];
    final modes = <DisplayModeInfo>[];
    if (rawModes is List) {
      for (final item in rawModes) {
        if (item is Map) {
          modes.add(DisplayModeInfo.fromJson(Map<String, dynamic>.from(item)));
        }
      }
    }
    final rawCurrent = json['current_mode'];
    final currentMode = rawCurrent is Map
        ? DisplayModeInfo.fromJson(Map<String, dynamic>.from(rawCurrent))
        : (modes.isNotEmpty
              ? modes.first
              : const DisplayModeInfo(width: 0, height: 0, refreshMillihz: 0));
    return DisplayInfo(
      id: json['id'] as String,
      name: json['name'] as String? ?? '显示器',
      primary: json['primary'] as bool? ?? false,
      currentMode: currentMode,
      modes: modes,
    );
  }

  /// 该模式是否就是当前生效的模式。
  bool isCurrent(DisplayModeInfo mode) => currentMode.equals(mode);
}
