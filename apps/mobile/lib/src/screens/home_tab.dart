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
import '../providers/input_controller.dart';
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
    await Future.wait([
      ref.read(favoritesProvider.notifier).refresh(),
      ref.read(libraryControllerProvider.notifier).refreshRecent(),
    ]);
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
    final uac = online && system.unlock?.uacDetected == true;
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
      deviceName: settings.pcName,
      subtitle: _statusSubtitle(
        online: online,
        locked: locked,
        uptime: system.status?.uptimeSeconds ?? 0,
        uptimeText: system.status?.uptimeText,
      ),
    );
    final pinnedApps = _sortedPinnedApps(
      ref.watch(favoritesProvider).pinned,
      settings.lastUsedLauncherId,
    );
    final recentApps = ref.watch(libraryControllerProvider).recent;
    final homeApps = _homeApps(
      recentApps: recentApps,
      pinnedApps: pinnedApps,
      fallbackLastUsedId: settings.lastUsedLauncherId,
    );

    return RefreshIndicator(
      color: AppColors.primary,
      onRefresh: () => _refreshAll(online),
      child: Column(
        children: [
          Expanded(
            child: ListView(
              physics: const AlwaysScrollableScrollPhysics(),
              padding: const EdgeInsets.fromLTRB(
                AppMetrics.pagePadding,
                AppSpacing.md,
                AppMetrics.pagePadding,
                AppSpacing.lg,
              ),
              children: [
                HomeStatusCard(
                  status: status,
                  onPrimary: _statusPrimaryAction(status, canWake),
                ),
                const SizedBox(height: AppSpacing.xl),
                const SectionHeading(title: '快捷操作'),
                HomeQuickActionsSection(
                  online: online,
                  ready: ready,
                  irReady: irReady,
                  shortcuts: settings.homeShortcuts,
                  onUnlock: () => _unlock(context, ref),
                  onTvPower: () => _tvPower(context, ref),
                  onShortcut: (label) => _sendShortcut(context, ref, label),
                ),
                const SizedBox(height: AppSpacing.md),
                HomeCommonAppsSection(
                  apps: homeApps,
                  online: online,
                  onOpenLibrary: widget.onGoToLibrary,
                  onLaunch: (app) => _launchQuickApp(context, ref, app),
                ),
              ],
            ),
          ),
          // 窗口控制是高频操作，常驻在 TabBar 上方的拇指区（设计稿 Dock 条）。
          Padding(
            padding: const EdgeInsets.fromLTRB(
              AppMetrics.pagePadding,
              AppSpacing.sm,
              AppMetrics.pagePadding,
              AppSpacing.sm,
            ),
            child: const WindowDockSection(),
          ),
        ],
      ),
    );
  }

  /// 状态卡副标题：在线时给出系统、锁屏与运行时长，离线时不额外解释。
  String? _statusSubtitle({
    required bool online,
    required bool locked,
    required int uptime,
    String? uptimeText,
  }) {
    if (!online) return null;
    final parts = <String>['Windows', locked ? '已锁定' : '已解锁'];
    // 卡片右侧有「在线」药丸，副标题宽度有限：运行时长压成 3h31m 这种
    // 紧凑写法（去掉小时与分钟之间的空格），三段落才不会被省略号截断。
    if (uptime > 0 && uptimeText != null) {
      parts.add('运行 ${uptimeText.replaceAll(' ', '')}');
    }
    return parts.join(' · ');
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
      ref.read(libraryControllerProvider.notifier).refreshRecent(),
    ]);
  }

  /// 置顶应用排序：上次使用过的排最前，其余按名称升序。
  List<LauncherItem> _sortedPinnedApps(
    List<LauncherItem> pinned,
    String? lastUsedId,
  ) {
    return pinned.toList()..sort((a, b) {
      if (a.id == lastUsedId) return -1;
      if (b.id == lastUsedId) return 1;
      return a.name.toLowerCase().compareTo(b.name.toLowerCase());
    });
  }

  /// 首页固定把最近启动项放在第一张，其余位置再补置顶项并去重。
  List<LauncherItem> _homeApps({
    required List<LauncherItem> recentApps,
    required List<LauncherItem> pinnedApps,
    required String? fallbackLastUsedId,
  }) {
    final recent = recentApps.isNotEmpty
        ? recentApps.first
        : pinnedApps.where((item) => item.id == fallbackLastUsedId).firstOrNull;
    final result = <LauncherItem>[];
    if (recent != null) result.add(recent);
    for (final item in pinnedApps) {
      if (result.length >= 12) break;
      if (result.every((existing) => existing.id != item.id)) result.add(item);
    }
    return result;
  }

  /// 状态卡主按钮的动作：唤醒、解锁，或没有动作时不可点击。
  VoidCallback? _statusPrimaryAction(HomeStatusView status, bool canWake) {
    if (status.action == HomeStatusAction.wake) {
      return canWake ? (widget.onWake ?? widget.onGoToMore) : widget.onGoToMore;
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
      await ref.read(libraryControllerProvider.notifier).refreshRecent();
      if (context.mounted) context.showSuccessSnackBar('正在启动 ${app.name}');
    } catch (error) {
      if (context.mounted) context.showErrorSnackBar('启动失败: $error');
    }
  }

  void _sendShortcut(BuildContext context, WidgetRef ref, String label) {
    unawaited(
      ref.read(inputControllerProvider).sendShortcut(label).catchError((error) {
        if (context.mounted) context.showErrorSnackBar('快捷键发送失败: $error');
      }),
    );
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
