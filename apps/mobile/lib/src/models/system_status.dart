/// 系统状态响应
class SystemStatus {
  final double cpuUsage;
  final int memoryTotal;
  final int memoryUsed;
  final double memoryPercent;
  final int uptimeSeconds;

  const SystemStatus({
    required this.cpuUsage,
    required this.memoryTotal,
    required this.memoryUsed,
    required this.memoryPercent,
    required this.uptimeSeconds,
  });

  factory SystemStatus.fromJson(Map<String, dynamic> json) {
    return SystemStatus(
      cpuUsage: (json['cpu_usage'] as num).toDouble(),
      memoryTotal: json['memory_total'] as int,
      memoryUsed: json['memory_used'] as int,
      memoryPercent: (json['memory_percent'] as num).toDouble(),
      uptimeSeconds: json['uptime_seconds'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
    'cpu_usage': cpuUsage,
    'memory_total': memoryTotal,
    'memory_used': memoryUsed,
    'memory_percent': memoryPercent,
    'uptime_seconds': uptimeSeconds,
  };

  /// 获取格式化的内存使用字符串
  String get memoryUsageText =>
      '${_formatBytes(memoryUsed)} / ${_formatBytes(memoryTotal)}';

  /// 获取格式化的运行时间
  String get uptimeText {
    final hours = uptimeSeconds ~/ 3600;
    final minutes = (uptimeSeconds % 3600) ~/ 60;
    return '${hours}h ${minutes}m';
  }

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
  }
}

/// 主音量状态。
class VolumeStatus {
  final int percent;
  final bool muted;

  const VolumeStatus({required this.percent, required this.muted});

  factory VolumeStatus.fromJson(Map<String, dynamic> json) {
    return VolumeStatus(
      percent: _clampPercent(_readInt(json['percent'])),
      muted: json['muted'] as bool? ?? false,
    );
  }

  Map<String, dynamic> toJson() => {'percent': percent, 'muted': muted};

  VolumeStatus copyWith({int? percent, bool? muted}) {
    return VolumeStatus(
      percent: _clampPercent(percent ?? this.percent),
      muted: muted ?? this.muted,
    );
  }
}

class AudioOutputDevice {
  final String id;
  final String name;
  final bool isDefault;

  const AudioOutputDevice({
    required this.id,
    required this.name,
    required this.isDefault,
  });

  factory AudioOutputDevice.fromJson(Map<String, dynamic> json) {
    return AudioOutputDevice(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '未命名输出设备',
      isDefault: json['is_default'] as bool? ?? false,
    );
  }
}

/// PC 解锁/登录辅助状态。
class UnlockStatus {
  final bool locked;
  final String mode;
  final bool autoLoginEnabled;
  final String message;
  final bool uacDetected;
  final String surface;

  const UnlockStatus({
    required this.locked,
    required this.mode,
    required this.autoLoginEnabled,
    required this.message,
    required this.uacDetected,
    required this.surface,
  });

  factory UnlockStatus.fromJson(Map<String, dynamic> json) {
    return UnlockStatus(
      locked: json['locked'] as bool? ?? false,
      mode: json['mode'] as String? ?? 'manual',
      autoLoginEnabled: json['auto_login_enabled'] as bool? ?? false,
      message: json['message'] as String? ?? '',
      uacDetected: json['uac_detected'] as bool? ?? false,
      surface: json['surface'] as String? ?? 'unknown',
    );
  }

  Map<String, dynamic> toJson() => {
    'locked': locked,
    'mode': mode,
    'auto_login_enabled': autoLoginEnabled,
    'message': message,
    'uac_detected': uacDetected,
    'surface': surface,
  };
}

class LockInputStatus {
  final bool available;
  final bool serviceRunning;
  final String message;

  const LockInputStatus({
    required this.available,
    required this.serviceRunning,
    required this.message,
  });

  factory LockInputStatus.fromJson(Map<String, dynamic> json) {
    return LockInputStatus(
      available: json['available'] as bool? ?? false,
      serviceRunning: json['service_running'] as bool? ?? false,
      message: json['message'] as String? ?? '',
    );
  }
}

class SecureDesktopStatus {
  final String desktop;
  final bool locked;
  final bool inputReady;
  final bool sasSupported;
  final int sessionId;
  final String agentState;
  final bool uacDetected;

  const SecureDesktopStatus({
    required this.desktop,
    required this.locked,
    required this.inputReady,
    required this.sasSupported,
    required this.sessionId,
    required this.agentState,
    required this.uacDetected,
  });

  factory SecureDesktopStatus.fromJson(Map<String, dynamic> json) =>
      SecureDesktopStatus(
        desktop: json['desktop'] as String? ?? 'unknown',
        locked: json['locked'] as bool? ?? false,
        inputReady: json['input_ready'] as bool? ?? false,
        sasSupported: json['sas_supported'] as bool? ?? false,
        sessionId: (json['session_id'] as num?)?.toInt() ?? 0,
        agentState: json['agent_state'] as String? ?? 'unknown',
        uacDetected: json['uac_detected'] as bool? ?? false,
      );
}

int _readInt(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  if (value is String) return int.tryParse(value) ?? 0;
  return 0;
}

int _clampPercent(int value) => value.clamp(0, 100).toInt();
