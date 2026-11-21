/// 进程信息
class ProcessInfo {
  final int pid;
  final String name;
  final int memoryBytes;
  final double cpuUsage;
  final String? path;

  /// 服务端返回的本地化/友好名称（可选），优先于内置中文映射。
  final String? displayName;

  /// 分组：app（可见窗口）/ background（后台）/ system（Windows 进程）。
  final String? group;

  const ProcessInfo({
    required this.pid,
    required this.name,
    required this.memoryBytes,
    required this.cpuUsage,
    this.path,
    this.displayName,
    this.group,
  });

  factory ProcessInfo.fromJson(Map<String, dynamic> json) {
    return ProcessInfo(
      pid: json['pid'] as int,
      name: json['name'] as String,
      memoryBytes: json['memory_bytes'] as int,
      cpuUsage: (json['cpu_usage'] as num).toDouble(),
      path: json['path'] as String?,
      displayName:
          json['display_name'] as String? ?? json['description'] as String?,
      group: json['group'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
    'pid': pid,
    'name': name,
    'memory_bytes': memoryBytes,
    'cpu_usage': cpuUsage,
    if (path != null) 'path': path,
    if (displayName != null) 'display_name': displayName,
    if (group != null) 'group': group,
  };

  /// 展示名：服务端本地化名优先，其次原始名称。
  String get displayLabel {
    final local = displayName?.trim();
    if (local != null && local.isNotEmpty) return local;
    return name;
  }

  /// 是否使用了中文/友好名（用于决定是否附带原始进程名）。
  bool get hasFriendlyLabel => displayLabel != name;

  /// 获取格式化的内存使用
  String get memoryText {
    if (memoryBytes < 1024) return '$memoryBytes B';
    if (memoryBytes < 1024 * 1024) {
      return '${(memoryBytes / 1024).toStringAsFixed(1)} KB';
    }
    if (memoryBytes < 1024 * 1024 * 1024) {
      return '${(memoryBytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(memoryBytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
  }
}

/// 前台窗口响应
class ForegroundResponse {
  final int pid;
  final String title;
  final String? path;
  final String processName;
  final int hwnd;

  /// PC 焦点输入框里已有的文本；没有可回填的输入框（含密码框）时为 `null`。
  final String? focusText;

  const ForegroundResponse({
    required this.pid,
    required this.title,
    this.path,
    required this.processName,
    this.hwnd = 0,
    this.focusText,
  });

  factory ForegroundResponse.fromJson(Map<String, dynamic> json) {
    return ForegroundResponse(
      pid: _readInt(json['pid']),
      title: json['title'] as String? ?? '',
      path: json['path'] as String?,
      processName:
          json['process_name'] as String? ??
          _basenameFromPath(json['path'] as String?) ??
          '',
      hwnd: _readInt(json['hwnd']),
      focusText: json['focus_text'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
    'pid': pid,
    'title': title,
    if (path != null) 'path': path,
    'process_name': processName,
    'hwnd': hwnd,
  };
}

/// 可见窗口信息。
class WindowInfo {
  final int hwnd;
  final String title;
  final int pid;
  final String? exePath;
  final bool isVisible;
  final bool isForeground;
  final bool isFullscreen;
  final bool isTopmost;

  const WindowInfo({
    required this.hwnd,
    required this.title,
    required this.pid,
    this.exePath,
    this.isVisible = true,
    this.isForeground = false,
    this.isFullscreen = false,
    this.isTopmost = false,
  });

  factory WindowInfo.fromJson(Map<String, dynamic> json) {
    return WindowInfo(
      hwnd: _readInt(json['hwnd'] ?? json['id']),
      title: json['title'] as String? ?? '',
      pid: _readInt(json['pid']),
      exePath: json['exe_path'] as String?,
      isVisible: json['is_visible'] as bool? ?? true,
      isForeground: json['is_foreground'] as bool? ?? false,
      isFullscreen: json['is_fullscreen'] as bool? ?? false,
      isTopmost: json['is_topmost'] as bool? ?? false,
    );
  }

  String get processName => _basenameFromPath(exePath) ?? '';
}

int _readInt(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  if (value is String) return int.tryParse(value) ?? 0;
  return 0;
}

String? _basenameFromPath(String? path) {
  if (path == null || path.isEmpty) return null;
  final parts = path.split(RegExp(r'[/\\]'));
  return parts.isNotEmpty ? parts.last : null;
}
