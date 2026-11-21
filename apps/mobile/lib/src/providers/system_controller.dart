import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/system_status.dart';
import '../models/process_info.dart';
import '../models/display_info.dart';
import '../utils/wake_on_lan.dart';
import '../services/ws_event_adapter.dart';
import 'remote_command_bus.dart';

class SystemState {
  const SystemState({
    this.status,
    this.foreground,
    this.unlock,
    this.volume,
    this.keepAwake,
    this.displays = const [],
    this.audioOutputs = const [],
    this.error,
    this.displaysLoading = false,
    this.displayError,
    this.selectedDisplayId,
  });

  final SystemStatus? status;
  final ForegroundResponse? foreground;
  final UnlockStatus? unlock;
  final VolumeStatus? volume;

  /// 服务进程的保持唤醒开关；`null` 表示尚未读取到状态。
  final bool? keepAwake;
  final List<DisplayInfo> displays;
  final List<AudioOutputDevice> audioOutputs;
  final String? error;

  /// 显示器列表正在刷新。
  final bool displaysLoading;

  /// 最近一次读取/切换显示器的失败原因；成功时为 `null`。
  final String? displayError;

  /// 用户在控制中心选中的显示器标识；为空表示跟随主显示器。
  final String? selectedDisplayId;

  /// 当前应展示/操作的显示器：优先用户选中的那台，其次主显示器，最后第一台。
  DisplayInfo? get activeDisplay {
    if (displays.isEmpty) return null;
    final id = selectedDisplayId;
    if (id != null) {
      for (final display in displays) {
        if (display.id == id) return display;
      }
    }
    for (final display in displays) {
      if (display.primary) return display;
    }
    return displays.first;
  }

  SystemState copyWith({
    SystemStatus? status,
    ForegroundResponse? foreground,
    UnlockStatus? unlock,
    VolumeStatus? volume,
    Object? keepAwake = _unset,
    List<DisplayInfo>? displays,
    List<AudioOutputDevice>? audioOutputs,
    Object? error = _unset,
    bool? displaysLoading,
    Object? displayError = _unset,
    Object? selectedDisplayId = _unset,
  }) => SystemState(
    status: status ?? this.status,
    foreground: foreground ?? this.foreground,
    unlock: unlock ?? this.unlock,
    volume: volume ?? this.volume,
    keepAwake: identical(keepAwake, _unset)
        ? this.keepAwake
        : keepAwake as bool?,
    displays: displays ?? this.displays,
    audioOutputs: audioOutputs ?? this.audioOutputs,
    error: identical(error, _unset) ? this.error : error as String?,
    displaysLoading: displaysLoading ?? this.displaysLoading,
    displayError: identical(displayError, _unset)
        ? this.displayError
        : displayError as String?,
    selectedDisplayId: identical(selectedDisplayId, _unset)
        ? this.selectedDisplayId
        : selectedDisplayId as String?,
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

  /// 刷新前台窗口信息，并把本次解析到的结果返回给调用方。
  ///
  /// 失败时返回 `null` 而不是回退到 `state.foreground` 的旧值：界面上继续显示上一次的
  /// 前台进程可以接受（清空会闪），但拿旧值去当输入框基线会把别的窗口的文字回填进来。
  Future<ForegroundResponse?> refreshForeground() async {
    try {
      final response = await _bus.request({'type': 'get_foreground'});
      final value = WsEventAdapter.parseForegroundResponse(response);
      if (value != null) state = state.copyWith(foreground: value);
      return value;
    } catch (error, stackTrace) {
      debugPrint('refreshForeground failed: $error\n$stackTrace');
      return null;
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

  /// 读取显示器列表。
  ///
  /// 失败不再静默：失败原因写进 [SystemState.displayError]，控制中心据此显示错误行和重试按钮。
  Future<void> refreshDisplays() async {
    state = state.copyWith(displaysLoading: true, displayError: null);
    try {
      final response = await _bus.request({'type': 'get_displays'});
      final data = response['data'];
      final raw = data is Map ? data['displays'] : null;
      if (response['type'] != 'displays' || raw is! List) {
        state = state.copyWith(
          displaysLoading: false,
          displayError: '显示器列表格式无法识别',
        );
        return;
      }
      final displays = <DisplayInfo>[];
      for (final item in raw) {
        if (item is! Map) continue;
        try {
          displays.add(DisplayInfo.fromJson(Map<String, dynamic>.from(item)));
        } catch (error) {
          debugPrint('skip unreadable display: $error');
        }
      }
      state = state.copyWith(
        displays: displays,
        displaysLoading: false,
        displayError: displays.isEmpty ? '未检测到已连接的显示器' : null,
      );
    } catch (error, stackTrace) {
      debugPrint('refreshDisplays failed: $error\n$stackTrace');
      state = state.copyWith(displaysLoading: false, displayError: _displayErrorText(error));
    }
  }

  /// 选中要显示/操作哪台显示器（多屏机器）。
  void selectDisplay(String displayId) {
    state = state.copyWith(selectedDisplayId: displayId);
  }

  /// 把异常翻译成给用户看的短语，避免直接把 Dart 异常字符串贴到界面上。
  String _displayErrorText(Object error) {
    final text = error.toString().replaceFirst('Exception: ', '');
    if (text.contains('timeout') || text.contains('Timeout')) {
      return '读取显示器超时，请稍后重试';
    }
    return text.isEmpty ? '读取显示器失败' : text;
  }

  Future<DisplayInfo?> primaryDisplay() async {
    await refreshDisplays();
    for (final display in state.displays) {
      if (display.primary) return display;
    }
    return null;
  }

  /// 读取 PC 焦点输入框里已有的文本，供键盘面板回填。
  ///
  /// 返回空串表示"没有可回填的输入框"（焦点不在编辑框上、密码框、只读、或 UIA 读不到）。
  /// 只看本次请求的结果，请求失败一律当空——绝不能拿上一次的旧文本当基线，那会把别的
  /// 窗口里的内容回填进来。这只影响输入框的初始内容，失败不弹错。
  Future<String> readFocusText() async {
    final foreground = await refreshForeground();
    return foreground?.focusText ?? '';
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

  /// 读取服务进程当前的保持唤醒状态。
  ///
  /// 失败时保留旧值，不把"读取失败"当成"已关闭"，避免界面误发一条关闭命令。
  Future<void> refreshKeepAwake() async {
    try {
      final response = await _bus.request({'type': 'get_keep_awake'});
      final data = response['data'];
      final enabled = data is Map ? data['enabled'] : null;
      if (enabled is bool) state = state.copyWith(keepAwake: enabled);
    } catch (error, stackTrace) {
      debugPrint('refreshKeepAwake failed: $error\n$stackTrace');
    }
  }

  /// 切换保持唤醒；成功后以服务端回传的状态为准更新界面。
  Future<void> setKeepAwake(bool enabled) async {
    final response = await _bus.request({
      'type': 'set_keep_awake',
      'enabled': enabled,
    });
    final data = response['data'];
    final value = data is Map ? data['enabled'] : null;
    state = state.copyWith(keepAwake: value is bool ? value : enabled);
  }
}

final systemControllerProvider =
    StateNotifierProvider<SystemController, SystemState>((ref) {
  return SystemController(ref.watch(remoteCommandBusProvider));
});
