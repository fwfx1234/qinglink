import '../models/process_info.dart';
import '../models/system_status.dart';

class WsEventAdapter {
  static ForegroundResponse? parseForegroundChanged(Map<String, dynamic> data) {
    final eventData = data['data'];
    if (eventData == null) return null;
    final map = eventData as Map<String, dynamic>;
    return ForegroundResponse(
      pid: _readInt(map['pid']),
      title: map['title'] as String? ?? '',
      path: map['path'] as String?,
      processName:
          map['process_name'] as String? ??
          _extractProcessName(map['path'] as String?) ??
          '',
      hwnd: _readInt(map['hwnd']),
    );
  }

  static SystemStatus? parseSystemStatus(Map<String, dynamic> data) {
    final nested = data['data'];
    if (nested is Map<String, dynamic>) {
      return SystemStatus.fromJson(nested);
    }
    return null;
  }

  static ForegroundResponse? parseForegroundResponse(
    Map<String, dynamic> data,
  ) {
    final nested = data['data'];
    if (nested is Map<String, dynamic>) {
      return ForegroundResponse.fromJson(nested);
    }
    return null;
  }

  static VolumeStatus? parseVolumeStatus(Map<String, dynamic> data) {
    final nested = data['data'];
    if (nested is Map<String, dynamic>) {
      return VolumeStatus.fromJson(nested);
    }
    return null;
  }

  static List<AudioOutputDevice>? parseAudioOutputs(Map<String, dynamic> data) {
    final nested = data['data'];
    if (nested is! Map<String, dynamic> || nested['devices'] is! List) {
      return null;
    }
    return (nested['devices'] as List)
        .whereType<Map<String, dynamic>>()
        .map(AudioOutputDevice.fromJson)
        .toList();
  }

  static UnlockStatus? parseUnlockStatus(Map<String, dynamic> data) {
    final nested = data['data'];
    if (nested is Map<String, dynamic>) {
      return UnlockStatus.fromJson(nested);
    }
    return null;
  }

  static SecureDesktopStatus? parseSecureDesktopStatus(Map<String, dynamic> data) {
    final nested = data['data'];
    if (nested is Map<String, dynamic>) return SecureDesktopStatus.fromJson(nested);
    return null;
  }

  static String? _extractProcessName(String? path) {
    if (path == null || path.isEmpty) return null;
    final parts = path.split(RegExp(r'[/\\]'));
    return parts.isNotEmpty ? parts.last : null;
  }

  static int _readInt(Object? value) {
    if (value is int) return value;
    if (value is num) return value.toInt();
    if (value is String) return int.tryParse(value) ?? 0;
    return 0;
  }
}
