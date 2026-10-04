/// 主壳的连接编排：配对弹窗触发、在线/离线提示、回前台重连，以及连接成功后的
/// MAC / 唤醒网络补齐与数据刷新。
///
/// 主壳只保留 Tab 装配与生命周期接线，见 `home_screen.dart`；唤醒流程在
/// `home_wake_flow.dart`，提示投递在 `home_notifications.dart`。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../models/wake_state.dart';
import '../providers/auth_provider.dart';
import '../providers/library_controller.dart';
import '../providers/settings_provider.dart';
import '../providers/system_controller.dart';
import '../theme/app_theme.dart';
import '../utils/presence_notifier.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/pairing_dialog.dart';
import 'home_notifications.dart';

/// 主壳连接编排器：持有配对冷却、在线提示簿记与连接后的刷新动作。
///
/// 状态回传给宿主 `State` 的方式：
/// - [isMounted] 判断宿主是否仍挂着（替代 `State.mounted`）；
/// - [isForegrounded] 读取宿主的前后台状态（[onPcOnlineChanged] 判提示用）；
/// - [contextOf] 取宿主 `BuildContext`，未挂载时为 null；
/// - [onWakeStateChanged] 由宿主 `setState` 落地新的 [WakeState]。
class HomeConnectionCoordinator {
  HomeConnectionCoordinator(
    this._ref, {
    required this.isMounted,
    required this.isForegrounded,
    required this.contextOf,
    required this.onWakeStateChanged,
    required this.notifications,
  });

  final WidgetRef _ref;

  /// 宿主（主壳 `State`）是否仍挂着。
  final bool Function() isMounted;

  /// 宿主是否处于前台。
  final bool Function() isForegrounded;

  /// 取宿主 `BuildContext`，宿主未挂载时返回 null。
  final BuildContext? Function() contextOf;

  /// 唤醒状态变化回传，由宿主 `setState` 落地。
  final void Function(WakeState state) onWakeStateChanged;

  /// 提示投递器，用于「PC已上线」等连接提示。
  final HomeNotificationPresenter notifications;

  bool _pairingDialogShown = false;
  DateTime? _lastPairingTime;
  bool _lastPcOnline = false;
  final PresenceNotificationTracker _presenceTracker =
      PresenceNotificationTracker();

  /// 连接成功且未认证时弹出配对对话框（带 3 秒冷却，避免重复触发）。
  void maybeShowPairingDialog(AuthState next) {
    if (_pairingDialogShown || !isMounted()) return;
    if (!next.isConnected || next.status != AuthStatus.unauthenticated) return;
    final now = DateTime.now();
    if (_lastPairingTime != null &&
        now.difference(_lastPairingTime!) < const Duration(seconds: 3)) {
      return;
    }
    _pairingDialogShown = true;
    _lastPairingTime = now;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final dialogContext = contextOf();
      if (dialogContext != null) _showPairingDialog(dialogContext);
    });
  }

  Future<void> _showPairingDialog(BuildContext context) async {
    try {
      await _ref.read(connectionProvider.notifier).beginPairing();
    } catch (error) {
      _pairingDialogShown = false;
      if (context.mounted) {
        final message =
            _ref.read(connectionProvider).errorMessage ?? error.toString();
        context.showErrorSnackBar(message);
      }
      return;
    }
    if (!context.mounted) return;
    final result = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (_) => const PairingDialog(),
    );
    _pairingDialogShown = false;
    if (result == true && context.mounted) {
      await handleAuthenticatedConnection();
    }
  }

  /// 心跳活性在线状态恢复 → 仅当本轮离线已提示过才补一条上线提示。
  void onPcOnlineChanged(bool pcOnline) {
    if (!isMounted()) return;
    final previous = _lastPcOnline;
    _lastPcOnline = pcOnline;
    if (previous == pcOnline) return;
    if (pcOnline) {
      if (_presenceTracker.onOnline(foregrounded: isForegrounded())) {
        notifications.showNotification('PC已上线', AppColors.success);
      }
    }
  }

  /// 清除未结算的离线/上线回合：后台期间的连接变化不再补提示。
  void resetPresenceEpisode() {
    _presenceTracker.resetEpisode();
  }

  void fallbackNormalConnect() {
    final config = _ref.read(settingsProvider);
    if (config.baseUrl != null) {
      _ref.read(connectionProvider.notifier).connectOnly().then((_) {
        if (isMounted() && _ref.read(connectionProvider).isAuthenticated) {
          handleAuthenticatedConnection();
        }
      });
    }
  }

  Future<void> handleAuthenticatedConnection() async {
    await _ref.read(connectionProvider.notifier).waitUntilReady();
    var unlockStatus = await _ref.read(systemControllerProvider.notifier).refreshUnlock();

    // A saved PIN selects credentialed unlock; without one the server runs the
    // confirmation flow. Keep the client side single-shot so a transitional
    // login surface cannot receive duplicates.
    final autoUnlockEnabled = _ref.read(settingsProvider).autoUnlockOnWake;
    if (autoUnlockEnabled && unlockStatus?.locked == true) {
      try {
        final password = await _ref
            .read(settingsProvider.notifier)
            .readLockPassword();
        unlockStatus = await _ref
            .read(systemControllerProvider.notifier)
            .unlockWithMode(lockPassword: password) ?? unlockStatus;
      } catch (error) {
        debugPrint('Automatic unlock failed: $error');
        unlockStatus = await _ref.read(systemControllerProvider.notifier).refreshUnlock() ?? unlockStatus;
      }
    }

    if (!isMounted()) return;
    // A failed status query must not be treated as an unlocked PC. Keep the
    // conservative locked state so the user can retry unlock instead of
    // receiving a false "ready" indication.
    final locked = unlockStatus?.locked ?? true;
    onWakeStateChanged(locked ? WakeState.locked : WakeState.connected);

    _fetchMacAddress();
    _refreshAllData();
  }

  Future<void> _fetchMacAddress() async {
    final config = _ref.read(settingsProvider);
    if (config.macAddress != null && config.macAddress!.isNotEmpty) return;

    final mac = await _ref.read(systemControllerProvider.notifier).getMacAddress();
    if (mac != null && mac.isNotEmpty && isMounted()) {
      await _ref.read(settingsProvider.notifier).updateMacAddress(mac);
    }
  }

  Future<void> fetchWakeNetworks() async {
    final networks = await _ref.read(systemControllerProvider.notifier).getWakeNetworks();
    if (networks.isNotEmpty && isMounted()) {
      await _ref.read(settingsProvider.notifier).updateWakeNetworks(networks);
      final first = networks.first;
      if ((_ref.read(settingsProvider).macAddress?.isNotEmpty ?? false) ==
          false) {
        await _ref
            .read(settingsProvider.notifier)
            .updateMacAddress(first.macAddress);
      }
    } else {
      await _fetchMacAddress();
    }
  }

  Future<void> _refreshAllData() async {
    if (!isMounted()) return;
    await Future.wait([
      _ref.read(systemControllerProvider.notifier).refreshStatus(),
      _ref.read(systemControllerProvider.notifier).refreshForeground(),
      _ref.read(libraryControllerProvider.notifier).refresh(),
      _ref.read(libraryControllerProvider.notifier).refreshRecent(),
    ]);
  }
}
