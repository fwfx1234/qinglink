import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/wake_state.dart';
import '../models/system_status.dart';
import '../providers/auth_provider.dart';
import '../providers/ir_remote_provider.dart';
import '../providers/remote_control_provider.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../utils/remote_actions.dart';
import '../utils/snackbar_extension.dart';
import '../utils/tv_power_follow.dart';
import '../widgets/active_window.dart';
import '../widgets/app_icon.dart';
import '../widgets/app_ui.dart';
import 'ir/tv_quick_panel.dart';
import 'mouse_keyboard_screen.dart';
import 'processes_tab.dart';

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
  Widget build(BuildContext context) {
    final authenticated = ref.watch(
      authProvider.select((s) => s.isAuthenticated),
    );
    final remote = ref.watch(remoteControlProvider);
    final settings = ref.watch(settingsProvider);
    final online = authenticated && remote.isConnected;
    final uac =
        online &&
        (remote.unlockStatus?.uacDetected == true ||
            remote.secureDesktopStatus?.uacDetected == true);
    final locked = online && remote.unlockStatus?.locked == true;
    final ready = online && !locked;
    final irState = ref.watch(irRemoteProvider);
    final irReady =
        irState.capability.supported && irState.activeProfile != null;
    final canWake = settings.macAddress?.isNotEmpty ?? false;

    final status = _resolveStatus(
      wakeState: widget.wakeState,
      online: online,
      locked: locked,
      uac: uac,
      canWake: canWake,
    );
    final pinnedApps =
        settings.quickApps.where((app) => app.pinnedToHome).toList()
          ..sort((a, b) {
            if (a.id == settings.lastUsedLauncherId) return -1;
            if (b.id == settings.lastUsedLauncherId) return 1;
            return a.name.toLowerCase().compareTo(b.name.toLowerCase());
          });

    return RefreshIndicator(
      color: AppColors.primary,
      onRefresh: () async {
        // Keep pull-to-refresh harmless while the PC is unavailable. The
        // status card remains the explicit wake/connect entry point.
        if (!online) return;
        await Future.wait([
          ref.read(remoteControlProvider.notifier).refreshSystemStatus(),
          ref.read(remoteControlProvider.notifier).refreshForeground(),
          ref
              .read(remoteControlProvider.notifier)
              .loadUnifiedLibrary(force: true),
        ]);
      },
      child: Stack(
        children: [
          ListView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const EdgeInsets.fromLTRB(18, 14, 18, 96),
            children: [
              if (!ready || uac) ...[
                _StatusCard(
                  status: status,
                  onPrimary: status.action == _StatusAction.wake
                      ? (canWake
                            ? (widget.onWake ?? widget.onGoToMore)
                            : widget.onGoToMore)
                      : status.action == _StatusAction.unlock
                      ? () => _unlock(context, ref)
                      : null,
                ),
                const SizedBox(height: 18),
              ],
              const SectionHeading(title: '快捷操作', bottom: 2),
              _QuickGrid(
                crossAxisCount: irReady ? 4 : 3,
                items: [
                  _QuickTile(
                    icon: Icons.lock_open_rounded,
                    label: '解锁',
                    tone: AppColors.amber,
                    // Keep the entry visible while preventing a remote
                    // command from being sent before the PC session exists.
                    enabled: online,
                    onTap: () => _unlock(context, ref),
                  ),
                  _QuickTile(
                    icon: Icons.dynamic_feed_rounded,
                    label: '进程',
                    tone: AppColors.violet,
                    enabled: ready,
                    onTap: () => Navigator.of(context).push(
                      MaterialPageRoute(builder: (_) => const ProcessesTab()),
                    ),
                  ),
                  _QuickTile(
                    icon: Icons.bedtime_rounded,
                    label: '休眠',
                    tone: AppColors.slate,
                    enabled: ready,
                    onTap: () => confirmPowerAction(context, ref, 'sleep'),
                  ),
                  _QuickTile(
                    icon: Icons.power_settings_new_rounded,
                    label: '关机',
                    tone: AppColors.red,
                    enabled: ready,
                    onTap: () => confirmPowerAction(context, ref, 'shutdown'),
                  ),
                  _QuickTile(
                    icon: Icons.volume_up_rounded,
                    label: '音量',
                    tone: AppColors.amber,
                    enabled: ready,
                    onTap: () => showVolumeSheet(context, ref),
                  ),
                  if (irReady) ...[
                    _QuickTile(
                      icon: Icons.power_settings_new_rounded,
                      label: '电源',
                      tone: AppColors.slate,
                      enabled: true,
                      onTap: () => _tvPower(context, ref),
                    ),
                    _QuickTile(
                      icon: Icons.settings_remote_rounded,
                      label: 'TV',
                      tone: AppColors.cyan,
                      enabled: true,
                      onTap: () => showTvQuickPanel(context),
                    ),
                  ],
                  _QuickTile(
                    icon: Icons.mouse_rounded,
                    label: '控制',
                    tone: AppColors.cyan,
                    enabled: ready,
                    onTap: () => Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (_) => const MouseKeyboardScreen(),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 14),
              const ActiveWindowSection(sectionTitle: '当前窗口'),
              const SizedBox(height: 22),
              _CommonAppsSection(
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

  Future<void> _launchQuickApp(
    BuildContext context,
    WidgetRef ref,
    QuickApp app,
  ) async {
    try {
      final remote = ref.read(remoteControlProvider.notifier);
      final scannerId = app.scannerId;
      if (scannerId != null && scannerId.isNotEmpty) {
        await remote.launchScannerApp(scannerId);
      } else {
        await remote.launchApp(app.path);
      }
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
    final remote = ref.read(remoteControlProvider.notifier);
    try {
      final password = await ref
          .read(settingsProvider.notifier)
          .readLockPassword();
      final result = await remote.unlockWithMode(lockPassword: password);
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

enum _StatusAction { wake, unlock, none }

class _StatusView {
  const _StatusView({
    required this.label,
    required this.dot,
    required this.title,
    required this.action,
    required this.primaryLabel,
    required this.primaryIcon,
    this.progressLabel,
    this.busy = false,
  });

  final String label;
  final Color dot;
  final String title;
  final _StatusAction action;
  final String primaryLabel;
  final IconData primaryIcon;
  final String? progressLabel;
  final bool busy;
}

_StatusView _resolveStatus({
  required WakeState wakeState,
  required bool online,
  required bool locked,
  required bool uac,
  required bool canWake,
}) {
  if (wakeState == WakeState.waking) {
    return const _StatusView(
      label: '正在唤醒',
      dot: AppColors.amber,
      title: '正在唤醒客厅电脑',
      action: _StatusAction.none,
      primaryLabel: '正在唤醒…',
      primaryIcon: Icons.rotate_right_rounded,
      progressLabel: '正在发送唤醒信号…',
      busy: true,
    );
  }
  if (wakeState == WakeState.checking || wakeState == WakeState.connecting) {
    return const _StatusView(
      label: '连接中',
      dot: AppColors.amber,
      title: '正在建立连接',
      action: _StatusAction.none,
      primaryLabel: '连接中…',
      primaryIcon: Icons.rotate_right_rounded,
      progressLabel: '正在等待电脑响应…',
      busy: true,
    );
  }
  if (wakeState == WakeState.failed) {
    return const _StatusView(
      label: '连接失败',
      dot: AppColors.red,
      title: '没有连上电脑',
      action: _StatusAction.wake,
      primaryLabel: '重新连接',
      primaryIcon: Icons.refresh_rounded,
    );
  }
  if (uac) {
    return const _StatusView(
      label: 'UAC 确认中',
      dot: AppColors.amber,
      title: '当前账户已登录，正在等待 UAC 确认',
      action: _StatusAction.none,
      primaryLabel: 'UAC 确认中',
      primaryIcon: Icons.verified_user_rounded,
    );
  }
  if (locked) {
    return const _StatusView(
      label: '需要解锁',
      dot: AppColors.amber,
      title: '解锁后即可使用',
      action: _StatusAction.unlock,
      primaryLabel: '解锁 Windows',
      primaryIcon: Icons.lock_open_rounded,
    );
  }
  if (!canWake) {
    return const _StatusView(
      label: '未连接',
      dot: AppColors.primary,
      title: '连接电脑，开始使用',
      action: _StatusAction.wake,
      primaryLabel: '连接电脑',
      primaryIcon: Icons.link_rounded,
    );
  }
  return const _StatusView(
    label: '已关机',
    dot: AppColors.primary,
    title: '唤醒电脑，开始游戏',
    action: _StatusAction.wake,
    primaryLabel: '唤醒电脑',
    primaryIcon: Icons.power_settings_new_rounded,
  );
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({required this.status, this.onPrimary});

  final _StatusView status;
  final VoidCallback? onPrimary;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(AppRadius.lg),
        boxShadow: [
          BoxShadow(
            color: palette.shadow,
            blurRadius: 24,
            offset: const Offset(0, 12),
            spreadRadius: -14,
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(AppRadius.lg),
        child: Stack(
          children: [
            Positioned(
              right: -46,
              top: -34,
              child: Container(
                width: 156,
                height: 156,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  border: Border.all(
                    color: AppColors.primary.withValues(alpha: 0.13),
                  ),
                ),
              ),
            ),
            Positioned(
              right: 34,
              bottom: -64,
              child: Container(
                width: 132,
                height: 132,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: AppColors.primary.withValues(alpha: 0.05),
                ),
              ),
            ),
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [
                    palette.statusGradientStart,
                    palette.statusGradientEnd,
                  ],
                ),
                border: Border.all(color: palette.statusBorder),
                borderRadius: BorderRadius.circular(AppRadius.lg),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      StatusDot(color: status.dot, glow: true),
                      const SizedBox(width: 7),
                      Text(
                        status.label,
                        style: AppText.label.copyWith(
                          color: status.dot,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  Text(
                    status.title,
                    style: AppText.cardTitle.copyWith(
                      fontSize: 15,
                      color: palette.statusTitle,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  if (status.busy) ...[
                    const SizedBox(height: 12),
                    const AppProgressBar(),
                    const SizedBox(height: 8),
                    Text(
                      status.progressLabel ?? '',
                      style: AppText.meta.copyWith(
                        color: AppColors.progressText,
                      ),
                    ),
                  ],
                  const SizedBox(height: 12),
                  PrimaryButton(
                    label: status.primaryLabel,
                    icon: status.primaryIcon,
                    height: 42,
                    expand: true,
                    busy: status.busy,
                    onPressed: status.busy ? null : onPrimary,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _QuickTile {
  const _QuickTile({
    required this.icon,
    required this.label,
    required this.tone,
    required this.enabled,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final Color tone;
  final bool enabled;
  final VoidCallback onTap;
}

class _QuickGrid extends StatelessWidget {
  const _QuickGrid({required this.items, this.crossAxisCount = 3});
  final List<_QuickTile> items;
  final int crossAxisCount;

  @override
  Widget build(BuildContext context) {
    return GridView.builder(
      shrinkWrap: true,
      padding: EdgeInsets.zero,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: items.length,
      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: crossAxisCount,
        mainAxisSpacing: 8,
        crossAxisSpacing: 8,
        mainAxisExtent: 48,
      ),
      itemBuilder: (context, index) => _QuickTileView(item: items[index]),
    );
  }
}

class _QuickTileView extends StatelessWidget {
  const _QuickTileView({required this.item});
  final _QuickTile item;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Opacity(
      opacity: item.enabled ? 1 : 0.4,
      child: PressableScale(
        enabled: item.enabled,
        child: AppCard(
          radius: AppRadius.md,
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 8),
          onTap: item.enabled ? item.onTap : null,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              TileIcon(
                icon: item.icon,
                color: item.tone,
                size: 26,
                radius: 8,
                iconSize: 15,
                alpha: 0.16,
              ),
              const SizedBox(width: 7),
              Flexible(
                child: Text(
                  item.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.label.copyWith(
                    fontWeight: FontWeight.w700,
                    color: palette.textPrimary,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CommonAppsSection extends StatelessWidget {
  const _CommonAppsSection({
    required this.apps,
    required this.lastUsedId,
    required this.online,
    required this.onLaunch,
    this.onOpenLibrary,
  });

  final List<QuickApp> apps;
  final String? lastUsedId;
  final bool online;
  final ValueChanged<QuickApp> onLaunch;
  final VoidCallback? onOpenLibrary;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 5),
          child: Row(
            children: [
              Text(
                '常用应用',
                style: AppText.sectionTitle.copyWith(
                  color: palette.textPrimary,
                ),
              ),
              const Spacer(),
              if (apps.isNotEmpty)
                GestureDetector(
                  onTap: onOpenLibrary,
                  child: Text(
                    '管理',
                    style: AppText.meta.copyWith(color: AppColors.primary),
                  ),
                ),
            ],
          ),
        ),
        if (apps.isEmpty)
          _EmptyApps(onTap: onOpenLibrary)
        else
          GridView.count(
            crossAxisCount: 2,
            padding: EdgeInsets.zero,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            mainAxisSpacing: 8,
            crossAxisSpacing: 8,
            childAspectRatio: 2.7,
            children: apps
                .take(4)
                .map(
                  (item) => _PinnedApp(
                    item: item,
                    lastUsed: lastUsedId == item.id,
                    enabled: online,
                    onTap: () => onLaunch(item),
                  ),
                )
                .toList(),
          ),
      ],
    );
  }
}

class _EmptyApps extends StatelessWidget {
  const _EmptyApps({this.onTap});
  final VoidCallback? onTap;
  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      constraints: const BoxConstraints(minHeight: 56),
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: BoxDecoration(
        border: Border.all(color: palette.line),
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Row(
        children: [
          Text(
            '还没有置顶应用',
            style: AppText.label.copyWith(color: palette.textSubtle),
          ),
          const Spacer(),
          TextButton(
            onPressed: onTap,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  '去库里添加',
                  style: AppText.body.copyWith(color: AppColors.primary),
                ),
                const Icon(
                  Icons.north_east_rounded,
                  size: 14,
                  color: AppColors.primary,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _PinnedApp extends StatelessWidget {
  const _PinnedApp({
    required this.item,
    required this.lastUsed,
    required this.enabled,
    required this.onTap,
  });
  final QuickApp item;
  final bool lastUsed;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final meta = lastUsed ? '上次使用' : '程序';
    return Opacity(
      opacity: enabled ? 1 : 0.5,
      child: PressableScale(
        enabled: enabled,
        child: AppCard(
          minHeight: 58,
          padding: const EdgeInsets.all(9),
          radius: AppRadius.lg,
          onTap: enabled ? onTap : null,
          child: Row(
            children: [
              AppIcon(id: item.scannerId ?? item.path, size: 34, radius: 9),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      item.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.label.copyWith(
                        fontWeight: FontWeight.w700,
                        color: palette.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      meta,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.tiny.copyWith(color: palette.textSubtle),
                    ),
                  ],
                ),
              ),
              const Icon(
                Icons.play_arrow_rounded,
                size: 14,
                color: AppColors.primary,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
