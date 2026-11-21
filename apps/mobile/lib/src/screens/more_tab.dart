import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../providers/auth_provider.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import 'connection_diagnose_screen.dart';
import 'control_center_screen.dart';
import 'processes_tab.dart';
import 'settings_tab.dart';
import 'shortcuts_screen.dart';
import 'ir/tv_remote_screen.dart';
import '../app/routes.dart';

class MoreTab extends ConsumerWidget {
  const MoreTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final settings = ref.watch(settingsProvider);
    final connected = ref.watch(connectionProvider.select((s) => s.isAuthenticated));
    return ListView(
      padding: const EdgeInsets.fromLTRB(
        AppMetrics.pagePadding,
        AppSpacing.md,
        AppMetrics.pagePadding,
        AppMetrics.tabBarInset,
      ),
      children: [
        _DeviceSummary(
          name: settings.pcName?.isNotEmpty == true ? settings.pcName! : '客厅电脑',
          connected: connected,
          onTap: () => _push(context, const SettingsTab()),
        ),
        const SizedBox(height: 18),
        Container(
          decoration: BoxDecoration(
            color: palette.surface,
            borderRadius: BorderRadius.circular(AppRadius.lg),
            border: Border.all(color: palette.line),
          ),
          child: Column(
            children: [
              _MenuRow(
                icon: Icons.monitor_heart_outlined,
                tone: AppColors.red,
                title: '进程管理',
                onTap: connected
                    ? () => _push(context, const ProcessesTab())
                    : null,
              ),
              _MenuRow(
                icon: Icons.tune_rounded,
                tone: AppColors.slate,
                title: '控制中心',
                onTap: connected
                    ? () => _push(context, const ControlCenterScreen())
                    : null,
              ),
              _MenuRow(
                icon: Icons.link_rounded,
                tone: AppColors.cyan,
                title: '配对与连接',
                // 已配对时看连接信息即可；未配对才需要走配对流程。
                onTap: connected
                    ? () => _push(context, const SettingsTab())
                    : () => _pair(context, ref),
              ),
              _MenuRow(
                icon: Icons.keyboard_command_key_rounded,
                tone: AppColors.violet,
                title: '快捷键方案',
                onTap: () => _push(context, const ShortcutsScreen()),
              ),
              _MenuRow(
                icon: Icons.tv_rounded,
                tone: AppColors.cyan,
                title: '电视遥控',
                onTap: () => _push(context, const TvRemoteScreen()),
              ),
              _MenuRow(
                icon: Icons.health_and_safety_outlined,
                tone: AppColors.amber,
                title: '连接诊断',
                onTap: () => _push(context, const ConnectionDiagnoseScreen()),
              ),
              // 所有偏好的唯一入口：开关不在这里重复出现，避免两处状态各写一份。
              _MenuRow(
                icon: Icons.settings_outlined,
                tone: AppColors.slate,
                title: '设置',
                onTap: () => _push(context, const SettingsTab()),
                isLast: true,
              ),
            ],
          ),
        ),
      ],
    );
  }

  /// 未配对时走配对流程：先让服务端进入配对态，再进全屏配对页。
  Future<void> _pair(BuildContext context, WidgetRef ref) async {
    try {
      await ref.read(connectionProvider.notifier).beginPairing();
    } catch (error) {
      if (context.mounted) context.showErrorSnackBar('开始配对失败: $error');
      return;
    }
    if (!context.mounted) return;
    await context.push<bool>(AppRoutes.pairing);
  }

  void _push(BuildContext context, Widget page) {
    final route = switch (page.runtimeType) {
      const (ControlCenterScreen) => AppRoutes.controlCenter,
      const (ProcessesTab) => AppRoutes.processes,
      const (SettingsTab) => AppRoutes.settings,
      const (ShortcutsScreen) => AppRoutes.shortcuts,
      const (TvRemoteScreen) => AppRoutes.tvRemote,
      const (ConnectionDiagnoseScreen) => AppRoutes.diagnose,
      _ => AppRoutes.home,
    };
    context.push(route);
  }
}

class _DeviceSummary extends StatelessWidget {
  const _DeviceSummary({
    required this.name,
    required this.connected,
    this.onTap,
  });

  final String name;
  final bool connected;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(14),
      onTap: onTap,
      child: Row(
        children: [
          TileIcon(
            icon: Icons.desktop_windows_outlined,
            color: AppColors.primary,
            size: 44,
            radius: 12,
            iconSize: 21,
            alpha: 0.12,
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  name,
                  style: AppText.cardTitle.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 5),
                Row(
                  children: [
                    StatusDot(
                      color: connected ? AppColors.primary : palette.textSubtle,
                      glow: connected,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      connected ? '已连接' : '已关机',
                      style: AppText.meta.copyWith(color: palette.textMuted),
                    ),
                  ],
                ),
              ],
            ),
          ),
          Icon(
            Icons.chevron_right_rounded,
            size: 18,
            color: palette.textSubtle,
          ),
        ],
      ),
    );
  }
}

class _MenuRow extends StatelessWidget {
  const _MenuRow({
    required this.icon,
    required this.tone,
    required this.title,
    this.onTap,
    this.isLast = false,
  });

  final IconData icon;
  final Color tone;
  final String title;
  final VoidCallback? onTap;
  final bool isLast;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppListRow(
      title: title,
      icon: icon,
      iconTone: tone,
      tile: true,
      minHeight: AppMetrics.listRow,
      showChevron: true,
      enabled: onTap != null,
      onTap: onTap,
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.md,
        vertical: AppSpacing.sm,
      ),
      border: Border(
        bottom: BorderSide(color: isLast ? Colors.transparent : palette.line),
      ),
    );
  }
}
