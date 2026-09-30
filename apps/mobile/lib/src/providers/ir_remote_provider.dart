import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/storage/preferences_repository.dart';
import '../models/ir_remote.dart';
import '../services/ir_transmitter.dart';
import 'dependencies.dart';

/// 生成 RCA-38 时序：`{38.7k,460,msb}<1,-2|1,-4>(8,-8,D:4,F:8,~D:4,~F:8,1,-16)`。
///
/// 雷鸟/TCL 电视红外遥控即使用该协议（设备码 D=15，载波 38.7kHz）。
List<int> _rca38Pattern(int function) {
  const unit = 460;
  final pattern = <int>[unit * 8, unit * 8];
  void addBits(int value, int width, {bool inverted = false}) {
    for (var bit = width - 1; bit >= 0; bit--) {
      final one = ((value >> bit) & 1) == 1;
      final mark = one ^ inverted;
      pattern
        ..add(unit)
        ..add(mark ? unit * 4 : unit * 2);
    }
  }

  addBits(15, 4);
  addBits(function & 0xff, 8);
  addBits(15, 4, inverted: true);
  addBits(function & 0xff, 8, inverted: true);
  pattern
    ..add(unit)
    ..add(unit * 16);
  return pattern;
}

/// 内置的雷鸟/TCL 电视遥控，数据来自 IRDB `codes/TCL/TV/15,-1.csv`。
final _defaultProfile = IrRemoteProfile(
  id: 'tcl_candidate',
  name: '雷鸟/TCL 电视',
  builtin: true,
  commands: [
    for (final item in <(String, String, int)>[
      ('power', '电源切换', 42),
      ('input', '信号源', 163),
      ('menu', '菜单', 8),
      ('back', '返回', 39),
      ('home', 'TV/主页', 58),
      ('up', '上', 89),
      ('down', '下', 88),
      ('left', '左', 86),
      ('right', '右', 87),
      ('ok', '确认', 244),
      ('volume_up', '音量 +', 47),
      ('volume_down', '音量 -', 46),
      ('channel_up', '频道 +', 45),
      ('channel_down', '频道 -', 44),
      ('mute', '静音', 63),
      ('info', '信息', 60),
      ('play', '播放/暂停', 232),
      ('stop', '停止', 228),
      ('seek_back', '后退搜索', 0),
      ('seek_forward', '前进搜索', 216),
      ('media', '媒体', 97),
      ('mts', '音轨', 120),
      ('period', '· 小数点', 147),
      ('pic', '图像', 236),
      ('favorite', '收藏', 5),
      ('clear', '清除', 6),
      ('guide', '节目指南', 26),
      ('num_0', '0', 48),
      ('num_1', '1', 49),
      ('num_2', '2', 50),
      ('num_3', '3', 51),
      ('num_4', '4', 52),
      ('num_5', '5', 53),
      ('num_6', '6', 54),
      ('num_7', '7', 55),
      ('num_8', '8', 56),
      ('num_9', '9', 57),
    ])
      IrCommand(
        id: item.$1,
        name: item.$2,
        carrierHz: 38700,
        patternUs: _rca38Pattern(item.$3),
      ),
  ],
);

class IrRemoteState {
  const IrRemoteState({
    this.profiles = const [],
    this.macros = const [],
    this.capability = const IrCapability(supported: false),
    this.loading = true,
    this.activeProfileId,
    this.runningMacroId,
    this.error,
  });

  final List<IrRemoteProfile> profiles;
  final List<IrMacro> macros;
  final IrCapability capability;
  final bool loading;

  /// 当前选中的遥控器 id，由管理页切换并持久化。
  final String? activeProfileId;
  final String? runningMacroId;
  final String? error;

  IrRemoteProfile? get activeProfile {
    for (final profile in profiles) {
      if (profile.id == activeProfileId) return profile;
    }
    return profiles.isEmpty ? null : profiles.first;
  }

  IrRemoteState copyWith({
    List<IrRemoteProfile>? profiles,
    List<IrMacro>? macros,
    IrCapability? capability,
    bool? loading,
    Object? activeProfileId = _unset,
    Object? runningMacroId = _unset,
    Object? error = _unset,
  }) => IrRemoteState(
    profiles: profiles ?? this.profiles,
    macros: macros ?? this.macros,
    capability: capability ?? this.capability,
    loading: loading ?? this.loading,
    activeProfileId: identical(activeProfileId, _unset)
        ? this.activeProfileId
        : activeProfileId as String?,
    runningMacroId: identical(runningMacroId, _unset)
        ? this.runningMacroId
        : runningMacroId as String?,
    error: identical(error, _unset) ? this.error : error as String?,
  );

  static const _unset = Object();
}

class IrRemoteNotifier extends StateNotifier<IrRemoteState> {
  IrRemoteNotifier({IrTransmitter? transmitter, PreferencesRepository? preferences})
    : _transmitter = transmitter ?? const IrTransmitter(),
      _preferences = preferences ?? SharedPreferencesRepository(),
      super(const IrRemoteState()) {
    unawaited(_load());
  }

  static const _profilesKey = 'ir_remote_profiles_v1';
  static const _macrosKey = 'ir_macros_v1';
  static const _activeProfileKey = 'ir_active_profile_v1';
  final IrTransmitter _transmitter;
  final Completer<void> _loadCompleter = Completer<void>();
  final PreferencesRepository _preferences;
  bool _disposed = false;
  int _macroGeneration = 0;

  /// 首次加载（内置遥控器 + 持久化配置 + 手机红外能力）完成的信号。
  Future<void> get loaded => _loadCompleter.future;

  Future<void> _load() async {
    try {
      final profiles = _decodeProfiles(await _preferences.readString(_profilesKey));
      final macros = _decodeMacros(await _preferences.readString(_macrosKey));
      IrCapability capability;
      try {
        capability = await _transmitter.capabilities();
      } catch (error) {
        capability = const IrCapability(supported: false);
      }
      final allProfiles = [_defaultProfile, ...profiles];
      final savedActive = await _preferences.readString(_activeProfileKey);
      final activeId = allProfiles.any((profile) => profile.id == savedActive)
          ? savedActive
          : allProfiles.first.id;
      if (!_disposed) {
        state = state.copyWith(
          profiles: allProfiles,
          macros: macros,
          capability: capability,
          activeProfileId: activeId,
          loading: false,
        );
      }
    } finally {
      if (!_loadCompleter.isCompleted) _loadCompleter.complete();
    }
  }

  List<IrRemoteProfile> _decodeProfiles(String? raw) {
    if (raw == null || raw.isEmpty) return const [];
    try {
      return (jsonDecode(raw) as List<dynamic>)
          .whereType<Map>()
          .map(
            (item) => IrRemoteProfile.fromJson(Map<String, dynamic>.from(item)),
          )
          .where((profile) => profile.id.isNotEmpty && !profile.builtin)
          .toList(growable: false);
    } catch (_) {
      return const [];
    }
  }

  List<IrMacro> _decodeMacros(String? raw) {
    if (raw == null || raw.isEmpty) return const [];
    try {
      return (jsonDecode(raw) as List<dynamic>)
          .whereType<Map>()
          .map((item) => IrMacro.fromJson(Map<String, dynamic>.from(item)))
          .where((macro) => macro.id.isNotEmpty)
          .toList(growable: false);
    } catch (_) {
      return const [];
    }
  }

  Future<void> _persist() async {
    await _preferences.writeString(
      _profilesKey,
      encodeIrProfiles(state.profiles.where((p) => !p.builtin).toList()),
    );
    await _preferences.writeString(_macrosKey, encodeIrMacros(state.macros));
  }

  Future<void> saveProfiles(List<IrRemoteProfile> profiles) async {
    final activeId = profiles.any((p) => p.id == state.activeProfileId)
        ? state.activeProfileId
        : (profiles.isEmpty ? null : profiles.first.id);
    state = state.copyWith(profiles: profiles, activeProfileId: activeId);
    await _persist();
  }

  Future<void> saveMacros(List<IrMacro> macros) async {
    state = state.copyWith(macros: macros);
    await _persist();
  }

  Future<void> selectProfile(String id) async {
    if (!state.profiles.any((profile) => profile.id == id)) return;
    state = state.copyWith(activeProfileId: id);
    await _preferences.writeString(_activeProfileKey, id);
  }

  String? validateCommand(IrCommand command) {
    if (command.name.trim().isEmpty) return '请输入按键名称';
    if (command.carrierHz <= 0 || command.carrierHz > 500000) return '载波频率无效';
    if (state.capability.supported &&
        !state.capability.supportsCarrier(command.carrierHz)) {
      return '手机不支持该载波频率';
    }
    if (command.patternUs.isEmpty) return '脉冲时序不能为空';
    if (command.patternUs.length.isOdd) return '脉冲数组必须包含偶数个数值';
    if (command.patternUs.any((value) => value <= 0)) return '脉冲时长必须为正整数';
    if (command.patternUs.length > 4096 ||
        command.patternUs.fold<int>(0, (sum, value) => sum + value) > 2000000) {
      return '脉冲时序超过长度或总时长限制';
    }
    if (command.repeatCount < 1 || command.repeatCount > 5) {
      return '重复次数必须为 1 到 5';
    }
    return null;
  }

  /// Validate a profile and all of its macro references locally.
  String? validateProfile(
    IrRemoteProfile profile, {
    List<IrMacro> macros = const [],
  }) {
    String? localError;
    if (profile.name.trim().isEmpty) localError = '请输入遥控器名称';
    for (final command in profile.commands) {
      localError ??= validateCommand(command);
    }
    if (profile.commands.length > 4096) {
      localError ??= '按键数量超过限制';
    }
    final commandIds = profile.commands.map((command) => command.id).toSet();
    for (final macro in macros.where(
      (macro) => macro.profileId == profile.id,
    )) {
      if (macro.steps.length > 32) {
        localError ??= '宏步骤不能超过 32 步';
      }
      for (var index = 0; index < macro.steps.length; index++) {
        final step = macro.steps[index];
        if (!commandIds.contains(step.commandId)) {
          localError ??= '流程“${macro.name}”第 ${index + 1} 步引用了失效按键';
        } else if (step.delayAfterMs < 0 || step.delayAfterMs > 5000) {
          localError ??= '流程“${macro.name}”第 ${index + 1} 步延时无效';
        }
      }
    }
    return localError;
  }

  Future<void> transmit(IrCommand command) async {
    final error = validateCommand(command);
    if (error != null) throw ArgumentError(error);
    if (!state.capability.supported) {
      throw const IrTransmitterException('UNSUPPORTED', '当前手机没有红外发射器');
    }
    await _transmitter.transmit(
      carrierHz: command.carrierHz,
      patternUs: command.patternUs,
      repeatCount: command.repeatCount,
    );
  }

  Future<void> runMacro(IrMacro macro) async {
    if (state.runningMacroId != null) throw StateError('已有红外流程正在执行');
    final profile = state.profiles
        .where((item) => item.id == macro.profileId)
        .firstOrNull;
    if (profile == null) throw ArgumentError('遥控器不存在');
    final commands = {
      for (final command in profile.commands) command.id: command,
    };
    for (final step in macro.steps) {
      if (!commands.containsKey(step.commandId)) {
        throw ArgumentError('流程包含失效按键');
      }
      if (step.delayAfterMs < 0 || step.delayAfterMs > 5000) {
        throw ArgumentError('流程延时无效');
      }
    }
    final generation = ++_macroGeneration;
    state = state.copyWith(runningMacroId: macro.id, error: null);
    try {
      for (final step in macro.steps) {
        if (generation != _macroGeneration) return;
        await transmit(commands[step.commandId]!);
        if (step.delayAfterMs > 0) {
          await Future<void>.delayed(Duration(milliseconds: step.delayAfterMs));
        }
      }
    } catch (error) {
      state = state.copyWith(error: error.toString());
      rethrow;
    } finally {
      if (!_disposed) state = state.copyWith(runningMacroId: null);
    }
  }

  void cancelMacro() {
    ++_macroGeneration;
    if (!_disposed) state = state.copyWith(runningMacroId: null);
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}

final irRemoteProvider = StateNotifierProvider<IrRemoteNotifier, IrRemoteState>(
  (ref) => IrRemoteNotifier(
    preferences: ref.read(preferencesRepositoryProvider),
  ),
);
