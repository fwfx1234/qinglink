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
  String get refreshText => refreshMillihz == 0
      ? ''
      : '${(refreshMillihz / 1000).toStringAsFixed(0)} Hz';
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

  factory DisplayInfo.fromJson(Map<String, dynamic> json) => DisplayInfo(
    id: json['id'] as String,
    name: json['name'] as String? ?? '显示器',
    primary: json['primary'] as bool? ?? false,
    currentMode: DisplayModeInfo.fromJson(
      json['current_mode'] as Map<String, dynamic>,
    ),
    modes: (json['modes'] as List? ?? const [])
        .map((mode) => DisplayModeInfo.fromJson(mode as Map<String, dynamic>))
        .toList(),
  );
}
