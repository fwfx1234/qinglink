import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/ir_remote.dart';
import '../providers/auth_provider.dart';
import '../providers/ir_remote_provider.dart';
import '../providers/remote_control_provider.dart';
import '../providers/settings_provider.dart';

/// 电视跟随电脑的动作类型。
enum TvFollowAction { powerOn, powerOff }

/// 从遥控器中解析按键：优先按 id，找不到时按名称关键字兜底。
IrCommand? findIrCommand(
  IrRemoteProfile? profile,
  String id, {
  List<String> nameKeywords = const [],
}) {
  if (profile == null) return null;
  for (final command in profile.commands) {
    if (command.id == id) return command;
  }
  for (final command in profile.commands) {
    if (nameKeywords.any(command.name.contains)) return command;
  }
  return null;
}

/// 电视电源键。多数电视使用同一个电源切换键，开/关共用。
IrCommand? findPowerCommand(IrRemoteProfile? profile) =>
    findIrCommand(profile, 'power', nameKeywords: const ['电源', '开关']);

/// 跟随电脑控制电视电源。
///
/// 未开启对应开关、手机无红外、或遥控器没有电源键时静默跳过，不影响
/// 电脑本身的动作。
Future<void> followPcWithTvPower(WidgetRef ref, TvFollowAction action) async {
  final settings = ref.read(settingsProvider);
  final enabled = switch (action) {
    TvFollowAction.powerOn => settings.tvFollowPcPowerOn,
    TvFollowAction.powerOff => settings.tvFollowPcPowerOff,
  };
  if (!enabled) return;

  final notifier = ref.read(irRemoteProvider.notifier);
  await notifier.loaded;
  if (!ref.read(irRemoteProvider).capability.supported) return;
  final command = findPowerCommand(ref.read(irRemoteProvider).activeProfile);
  if (command == null) return;
  try {
    await notifier.transmit(command);
  } catch (error) {
    debugPrint('跟随电视电源失败: $error');
  }
}

/// 电脑是否已可达（已连接、心跳在线或已认证）。
bool isPcReachable(WidgetRef ref) {
  final remote = ref.read(remoteControlProvider);
  return remote.isConnected ||
      remote.pcOnline ||
      ref.read(authProvider).isAuthenticated;
}

/// 发送唤醒后等待 [delay]，只有电脑仍未在线时才切换电视电源，避免电脑/
/// 电视本就开着时被误关。
Future<void> followTvPowerAfterWake(
  WidgetRef ref, {
  Duration delay = const Duration(seconds: 5),
}) async {
  await Future<void>.delayed(delay);
  if (isPcReachable(ref)) return;
  await followPcWithTvPower(ref, TvFollowAction.powerOn);
}
