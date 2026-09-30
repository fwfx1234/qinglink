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

  Future<void> setDisplayMode(String displayId, DisplayModeInfo mode) async {
    await _bus.request({
      'type': 'set_display_mode',
      'display_id': displayId,
      'mode': mode.toJson(),
    });
    await refreshDisplays();
  }

  Future<String?> getMacAddress() async {
    final response = await _bus.request({'type': 'get_mac_address'});
    return response['mac_address'] as String?;
  }

  Future<List<WakeNetwork>> getWakeNetworks() async {
    final response = await _bus.request({'type': 'get_wake_networks'});
    final values = response['wake_networks'];
    if (values is! List) return const [];
    return values
        .whereType<Map>()
        .map((item) => WakeNetwork.fromJson(Map<String, dynamic>.from(item)))
        .toList(growable: false);
  }

  Future<LockInputStatus?> refreshLockInput() async {
    final response = await _bus.request({'type': 'get_lock_input_status'});
    final value = response['data'];
    if (value is! Map) return null;
    return LockInputStatus.fromJson(Map<String, dynamic>.from(value));
  }

  Future<void> sendSecureAttention() =>
      _bus.request({'type': 'secure_attention'}).then((_) {});

  Future<void> sendSecureAscii(String text) =>
      _bus.request({'type': 'secure_ascii', 'text': text}).then((_) {});
}

final systemControllerProvider =
    StateNotifierProvider<SystemController, SystemState>((ref) {
  return SystemController(ref.watch(remoteCommandBusProvider));
});
