import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/system_status.dart';
import '../models/process_info.dart';
import '../models/display_info.dart';
import '../utils/wake_on_lan.dart';
import '../services/ws_event_adapter.dart';
import 'remote_command_bus.dart';

class SystemState {
  const SystemState({this.status, this.foreground, this.unlock, this.volume, this.displays = const [], this.audioOutputs = const [], this.error});

  final SystemStatus? status;
  final ForegroundResponse? foreground;
  final UnlockStatus? unlock;
  final VolumeStatus? volume;
  final List<DisplayInfo> displays;
  final List<AudioOutputDevice> audioOutputs;
  final String? error;

  SystemState copyWith({
    SystemStatus? status,
    ForegroundResponse? foreground,
    UnlockStatus? unlock,
    VolumeStatus? volume,
    List<DisplayInfo>? displays,
    List<AudioOutputDevice>? audioOutputs,
    Object? error = _unset,
  }) => SystemState(
    status: status ?? this.status,
    foreground: foreground ?? this.foreground,
    unlock: unlock ?? this.unlock,
    volume: volume ?? this.volume,
    displays: displays ?? this.displays,
    audioOutputs: audioOutputs ?? this.audioOutputs,
    error: identical(error, _unset) ? this.error : error as String?,
  );

  static const _unset = Object();
}

class SystemController extends StateNotifier<SystemState> {
  SystemController(this._bus) : super(const SystemState());

  final RemoteCommandBus _bus;

  Future<void> refreshStatus() async {
    try {
      final response = await _bus.request({'type': 'get_system_status'});
      final value = WsEventAdapter.parseSystemStatus(response);
      if (value != null) state = state.copyWith(status: value, error: null);
    } catch (error, stackTrace) {
      debugPrint('refreshStatus failed: $error\n$stackTrace');
      state = state.copyWith(error: error.toString());
    }
  }

  Future<void> refreshForeground() async {
    try {
      final response = await _bus.request({'type': 'get_foreground'});
      final value = WsEventAdapter.parseForegroundResponse(response);
      if (value != null) state = state.copyWith(foreground: value);
    } catch (error, stackTrace) {
      debugPrint('refreshForeground failed: $error\n$stackTrace');
    }
  }

  Future<UnlockStatus?> refreshUnlock() async {
    try {
      final response = await _bus.request({'type': 'get_unlock_status'});
      final value = WsEventAdapter.parseUnlockStatus(response);
      if (value != null) state = state.copyWith(unlock: value);
      return value;
    } catch (error, stackTrace) {
      debugPrint('refreshUnlock failed: $error\n$stackTrace');
      return null;
    }
  }

  Future<UnlockStatus?> unlockWithMode({String? lockPassword}) async {
    final payload = <String, dynamic>{'type': 'auto_unlock'};
    if (lockPassword != null && lockPassword.trim().isNotEmpty) {
      payload['pin'] = lockPassword.trim();
    }
    await _bus.request(payload);
    return refreshUnlock();
  }

  Future<void> refreshDisplays() async {
    try {
      final response = await _bus.request({'type': 'get_displays'});
      final data = response['data'];
      if (response['type'] == 'displays' && data is Map<String, dynamic> && data['displays'] is List) {
        state = state.copyWith(displays: (data['displays'] as List).map((e) => DisplayInfo.fromJson(e as Map<String, dynamic>)).toList(growable: false));
      }
    } catch (error, stackTrace) {
      debugPrint('refreshDisplays failed: $error\n$stackTrace');
    }
  }

  Future<DisplayInfo?> primaryDisplay() async {
    await refreshDisplays();
    for (final display in state.displays) {
      if (display.primary) return display;
    }
    return null;
  }

  Future<void> refreshVolume() async {
    try {
      final response = await _bus.request({'type': 'get_volume'});
      final value = WsEventAdapter.parseVolumeStatus(response);
      if (value != null) state = state.copyWith(volume: value);
    } catch (error, stackTrace) {
      debugPrint('refreshVolume failed: $error\n$stackTrace');
    }
  }

  Future<void> refreshAudioOutputs() async {
    try {
      final response = await _bus.request({'type': 'get_audio_outputs'});
      final value = WsEventAdapter.parseAudioOutputs(response);
      if (value != null) state = state.copyWith(audioOutputs: value);
    } catch (error, stackTrace) {
      debugPrint('refreshAudioOutputs failed: $error\n$stackTrace');
    }
  }

  Future<void> setVolume(int percent) async {
    final value = percent.clamp(0, 100).toInt();
    await _bus.request({'type': 'set_volume', 'percent': value});
    state = state.copyWith(volume: (state.volume ?? const VolumeStatus(percent: 0, muted: false)).copyWith(percent: value));
  }

  Future<void> setMuted(bool muted) async {
    await _bus.request({'type': 'set_muted', 'muted': muted});
    state = state.copyWith(volume: (state.volume ?? const VolumeStatus(percent: 0, muted: false)).copyWith(muted: muted));
  }

  Future<void> setAudioOutput(String deviceId) async {
    await _bus.request({'type': 'set_audio_output', 'device_id': deviceId});
    await refreshAudioOutputs();
  }

  /// 切分辨率/刷新率。服务端要的是扁平三字段，不是嵌套的 `mode` 对象。
  Future<void> setDisplayMode(String displayId, DisplayModeInfo mode) async {
    await _bus.request({
      'type': 'set_display_mode',
      'display_id': displayId,
      'width': mode.width,
      'height': mode.height,
      'refresh_millihz': mode.refreshMillihz,
    });
    await refreshDisplays();
  }

  /// 读取 PC 主网卡 MAC。
  ///
  /// 服务端把它放在信封的 `data.mac_address` 里；取不到（`MAC_ERROR`）或传输失败时
  /// 返回 `null`——调用方在连接成功后做的是"补齐本地 MAC"，不应因此抛异常。
  Future<String?> getMacAddress() async {
    try {
      final response = await _bus.request({'type': 'get_mac_address'});
      final data = response['data'];
      final mac = data is Map ? data['mac_address'] : null;
      return mac is String && mac.isNotEmpty ? mac : null;
    } catch (error) {
      debugPrint('getMacAddress failed: $error');
      return null;
    }
  }

  /// 读取支持网络唤醒的网卡列表（信封路径同上是 `data.networks`）。
  Future<List<WakeNetwork>> getWakeNetworks() async {
    try {
      final response = await _bus.request({'type': 'get_wake_networks'});
      final data = response['data'];
      final values = data is Map ? data['networks'] : null;
      if (values is! List) return const [];
      return values
          .whereType<Map>()
          .map((item) => WakeNetwork.fromJson(Map<String, dynamic>.from(item)))
          .toList(growable: false);
    } catch (error) {
      debugPrint('getWakeNetworks failed: $error');
      return const [];
    }
  }

  /// 读取锁屏输入通道状态；服务端拒绝或连接异常时返回 `null`。
  ///
  /// 调用点（控制页的首帧回调）不兜异常，失败逃逸会变成未捕获异步异常。
  Future<LockInputStatus?> refreshLockInput() async {
    try {
      final response = await _bus.request({'type': 'get_lock_input_status'});
      final value = response['data'];
      if (value is! Map) return null;
      return LockInputStatus.fromJson(Map<String, dynamic>.from(value));
    } catch (error, stackTrace) {
      debugPrint('refreshLockInput failed: $error\n$stackTrace');
      return null;
    }
  }

  Future<void> sendSecureAttention() =>
      _bus.request({'type': 'secure_attention'}).then((_) {});

  Future<void> sendSecureAscii(String text) =>
      _bus.request({'type': 'send_secure_ascii', 'text': text}).then((_) {});
}

final systemControllerProvider =
    StateNotifierProvider<SystemController, SystemState>((ref) {
  return SystemController(ref.watch(remoteCommandBusProvider));
});
