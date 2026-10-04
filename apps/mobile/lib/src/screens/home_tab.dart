/// 首页分区：状态卡、快捷操作区与常用应用区的装配与状态接线。
///
/// 其余职责分别落在同目录的独立库里：状态模型与推导在 `home_status.dart`，
/// 状态卡组件在 `home_status_card.dart`，快捷操作区在 `home_quick_actions.dart`，
/// 常用应用区在 `home_common_apps.dart`。本文件只保留 `HomeTab` 与它必须持有的
/// 状态（常用列表同步、解锁忙碌标志与首页动作）。
library;

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../models/wake_state.dart';
import '../models/system_status.dart';
import '../providers/auth_provider.dart';
import '../providers/favorites_controller.dart';
import '../providers/ir_remote_provider.dart';
import '../providers/launcher_controller.dart';
import '../providers/feature_facades.dart';
import '../providers/system_controller.dart';
import '../providers/library_controller.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../utils/tv_power_follow.dart';
import '../widgets/active_window.dart';
import '../widgets/app_ui.dart';
import 'home_common_apps.dart';
import 'home_quick_actions.dart';
import 'home_status.dart';
import 'home_status_card.dart';

class HomeTab extends ConsumerStatefulWidget {
  final WakeState wakeState;
  final VoidCallback? onGoToLibrary;
  final VoidCallback? onGoToMore;
  final Future<void> Function()? onWake;

  const HomeTab({
    super.key,
    this.wakeState = WakeState.idle,
    this.onGoToLibrary,
    this.onGoToMore,
    this.onWake,
  });

  @override
  ConsumerState<HomeTab> createState() => _HomeTabState();
}

class _HomeTabState extends ConsumerState<HomeTab> {
  bool _unlockBusy = false;

  @override
  void initState() {
    super.initState();
    // 首页的「常用应用」直接读服务端的常用数据，所以启动时要主动同步一次；
    // 否则冷启动后要等下拉刷新才会出现置顶项。
    WidgetsBinding.instance.addPostFrameCallback((_) {
      unawaited(_syncFavorites());
    });
  }

  Future<void> _syncFavorites() async {
    if (!mounted) return;
    if (!ref.read(connectionProvider.select((s) => s.isAuthenticated))) return;
    await ref.read(favoritesProvider.notifier).refresh();
  }

  @override
  Widget build(BuildContext context) {
    // 配对成功后才连得上服务端，此时把常用列表补拉一次。
    ref.listen(connectionProvider.select((s) => s.isAuthenticated), (_, next) {
      if (next) unawaited(_syncFavorites());
    });
    final authenticated = ref.watch(
      connectionProvider.select((s) => s.isAuthenticated),
    );
    final online = ref.watch(connectionReadyProvider) && authenticated;
    final system = ref.watch(systemControllerProvider);
    final settings = ref.watch(settingsProvider);
    final uac =
        online &&
        system.unlock?.uacDetected == true;
    final locked = online && system.unlock?.locked == true;
    final ready = online && !locked;
    final irState = ref.watch(irRemoteProvider);
    final irReady =
        irState.capability.supported && irState.activeProfile != null;
    final canWake = settings.macAddress?.isNotEmpty ?? false;

    final status = resolveHomeStatus(
      wakeState: widget.wakeState,
      online: online,
      locked: locked,
      uac: uac,
      canWake: canWake,
    );
    final pinnedApps = _sortedPinnedApps(
      ref.watch(favoritesProvider).pinned,
      settings.lastUsedLauncherId,
    );

    return RefreshIndicator(
      color: AppColors.primary,
      onRefresh: () => _refreshAll(online),
      child: Stack(
        children: [
          ListView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const EdgeInsets.fromLTRB(18, 14, 18, 96),
            children: [
              if (!ready || uac) ...[
                HomeStatusCard(
                  status: status,
                  onPrimary: _statusPrimaryAction(status, canWake),
                ),
                const SizedBox(height: 18),
              ],
              const SectionHeading(title: '快捷操作', bottom: 2),
              HomeQuickActionsSection(
                online: online,
                ready: ready,
                irReady: irReady,
                onUnlock: () => _unlock(context, ref),
                onTvPower: () => _tvPower(context, ref),
              ),
              const SizedBox(height: 14),
              const ActiveWindowSection(sectionTitle: '当前窗口'),
              const SizedBox(height: 22),
              HomeCommonAppsSection(
                apps: pinnedApps,
                lastUsedId: settings.lastUsedLauncherId,
                online: online,
                onOpenLibrary: widget.onGoToLibrary,
                onLaunch: (app) => _launchQuickApp(context, ref, app),
              ),
            ],
          ),
        ],
      ),
    );
  }

  /// 下拉刷新：电脑不可用时保持无副作用，状态卡仍是显式唤醒/连接入口。
  Future<void> _refreshAll(bool online) async {
    // Keep pull-to-refresh harmless while the PC is unavailable. The
    // status card remains the explicit wake/connect entry point.
    if (!online) return;
    await Future.wait([
      ref.read(systemControllerProvider.notifier).refreshStatus(),
      ref.read(systemControllerProvider.notifier).refreshForeground(),
      ref.read(favoritesProvider.notifier).refresh(),
      ref.read(libraryControllerProvider.notifier).refresh(),
    ]);
  }

  /// 置顶应用排序：上次使用过的排最前，其余按名称升序。
  List<LauncherItem> _sortedPinnedApps(
    List<LauncherItem> pinned,
    String? lastUsedId,
  ) {
    return pinned.toList()
      ..sort((a, b) {
        if (a.id == lastUsedId) return -1;
        if (b.id == lastUsedId) return 1;
        return a.name.toLowerCase().compareTo(b.name.toLowerCase());
      });
  }

  /// 状态卡主按钮的动作：唤醒、解锁，或没有动作时不可点击。
  VoidCallback? _statusPrimaryAction(HomeStatusView status, bool canWake) {
    if (status.action == HomeStatusAction.wake) {
      return canWake
          ? (widget.onWake ?? widget.onGoToMore)
          : widget.onGoToMore;
    }
    if (status.action == HomeStatusAction.unlock) {
      return () => _unlock(context, ref);
    }
    return null;
  }

  Future<void> _launchQuickApp(
    BuildContext context,
    WidgetRef ref,
    LauncherItem app,
  ) async {
    try {
      await ref.read(launcherControllerProvider).launch(app.id);
      await ref.read(settingsProvider.notifier).setLastUsedLauncher(app.id);
      if (context.mounted) context.showSuccessSnackBar('正在启动 ${app.name}');
    } catch (error) {
      if (context.mounted) context.showErrorSnackBar('启动失败: $error');
    }
  }

  /// Sends the configured unlock action immediately. Lock state is refreshed
  /// only after the action completes so a stale status never suppresses an
  /// explicit user request.
  Future<void> _unlock(BuildContext context, WidgetRef ref) async {
    if (_unlockBusy) return;
    setState(() => _unlockBusy = true);
    try {
      final password = await ref
          .read(settingsProvider.notifier)
          .readLockPassword();
      final result = await ref
          .read(systemControllerProvider.notifier)
          .unlockWithMode(lockPassword: password);
      if (context.mounted) _reportUnlockResult(context, result);
    } catch (error) {
      if (context.mounted) context.showErrorSnackBar('解锁失败: $error');
    } finally {
      if (mounted) setState(() => _unlockBusy = false);
    }
  }

  void _reportUnlockResult(BuildContext context, UnlockStatus? status) {
    if (status?.locked != false) {
      context.showWarningSnackBar('仍未解锁，可用「控制」里的键盘直接输入密码');
    }
  }

  /// 首页「电视电源」：用当前遥控器的电源键发送红外信号（电视只有电源切换）。
  Future<void> _tvPower(BuildContext context, WidgetRef ref) async {
    final command = findPowerCommand(ref.read(irRemoteProvider).activeProfile);
    if (command == null) {
      context.showWarningSnackBar('当前遥控器没有电源键');
      return;
    }
    try {
      await ref.read(irRemoteProvider.notifier).transmit(command);
      if (context.mounted) context.showSuccessSnackBar('已发送电视电源');
    } catch (error) {
      if (context.mounted) context.showErrorSnackBar('电视电源发送失败: $error');
    }
  }
}
