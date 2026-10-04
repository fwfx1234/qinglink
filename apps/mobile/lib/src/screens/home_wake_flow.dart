/// 主壳的唤醒流程：Wake-on-LAN 发送、等待电脑上线与唤醒状态回传。
///
/// 主壳只把 [HomeWakeFlow.sendWakeAndWait] 接到首页状态卡的唤醒动作上，
/// 见 `home_screen.dart`；连接与配对编排在 `home_connection.dart`。
library;

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/wake_state.dart';
import '../providers/auth_provider.dart';
import '../providers/settings_provider.dart';
import '../providers/system_controller.dart';
import '../utils/tv_power_follow.dart';
import '../utils/wake_on_lan.dart';

/// 唤醒流程编排器：两轮 Wake-on-LAN + 等待连接，状态变化回传给宿主。
class HomeWakeFlow {
  HomeWakeFlow(
    this._ref, {
    required this.isMounted,
    required this.onWakeStateChanged,
    required this.onConnected,
  });

  final WidgetRef _ref;

  /// 宿主（主壳 `State`）是否仍挂着，用于等待循环中提前退出。
  final bool Function() isMounted;

  /// 唤醒状态变化回传，由宿主 `setState` 落地（宿主内部再做挂载检查）。
  final void Function(WakeState state) onWakeStateChanged;

  /// 电脑上线并连接可用后的收尾动作。
  final Future<void> Function() onConnected;

  bool _wakeFlowInFlight = false;

  /// 首页状态卡的唤醒动作：发两轮唤醒，每轮最多等 2 分钟。
  Future<void> sendWakeAndWait() async {
    if (_wakeFlowInFlight) return;
    _wakeFlowInFlight = true;
    try {
      // 第一轮唤醒
      onWakeStateChanged(WakeState.waking);
      await _doWakeOnLan();

      // 跟随电脑开电视：发唤醒后等 5 秒，电脑已在线说明电视已开，不再切电源。
      unawaited(followTvPowerAfterWake(_ref));

      var success = await _waitAndHandle(const Duration(minutes: 2));
      if (success) return;

      // 第二轮：重试一次
      onWakeStateChanged(WakeState.waking);
      await _doWakeOnLan();

      success = await _waitAndHandle(const Duration(minutes: 2));
      if (!success) onWakeStateChanged(WakeState.failed);
    } catch (error) {
      debugPrint('WoL failed: $error');
      onWakeStateChanged(WakeState.failed);
    } finally {
      _wakeFlowInFlight = false;
    }
  }

  Future<bool> _waitAndHandle(Duration timeout) async {
    final success = await _waitForConnection(timeout);
    if (success) {
      await onConnected();
    }
    return success;
  }

  Future<void> _doWakeOnLan() async {
    final system = _ref.read(systemControllerProvider.notifier);
    var config = _ref.read(settingsProvider);
    var macAddress = config.macAddress;
    if ((macAddress == null || macAddress.isEmpty) &&
        config.wakeNetworks.isNotEmpty) {
      macAddress = config.wakeNetworks.first.macAddress;
      await _ref.read(settingsProvider.notifier).updateMacAddress(macAddress);
      config = _ref.read(settingsProvider);
    }
    if (macAddress == null || macAddress.isEmpty) {
      // Older pairings may not have persisted the MAC/network snapshot. If a
      // session is available, refresh it before giving up on the wake action.
      final fetched = await system.getMacAddress();
      if (fetched != null && fetched.isNotEmpty) {
        await _ref.read(settingsProvider.notifier).updateMacAddress(fetched);
        config = _ref.read(settingsProvider);
        macAddress = config.macAddress;
      }
    }
    if (macAddress == null || macAddress.isEmpty) {
      throw StateError('MAC address is not configured');
    }
    final prefix = 24;
    final targetAddress = getSubnetBroadcast(config.pcIpAddress, prefix);
    await WakeOnLan.wake(
      macAddress: macAddress,
      address: targetAddress,
      networks: config.wakeNetworks.isEmpty ? null : config.wakeNetworks,
    );
  }

  Future<bool> _waitForConnection(Duration timeout) async {
    final stopwatch = Stopwatch()..start();
    while (stopwatch.elapsed < timeout) {
      if (!isMounted()) return false;
      onWakeStateChanged(WakeState.connecting);
      await _ref.read(connectionProvider.notifier).connectOnly();
      if (_ref.read(connectionProvider).isAuthenticated) {
        return true;
      }
      await Future.delayed(const Duration(seconds: 3));
    }
    return false;
  }
}
