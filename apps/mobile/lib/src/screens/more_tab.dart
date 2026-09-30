import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../providers/auth_provider.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
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
      padding: const EdgeInsets.fromLTRB(18, 14, 18, 96),
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
                onTap: () => _push(context, const SettingsTab()),
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
                onTap: () => _push(context, const SettingsTab()),
                isLast: true,
              ),
            ],
          ),
        ),
        const SizedBox(height: 18),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 14),
          decoration: BoxDecoration(
            color: palette.surface,
            borderRadius: BorderRadius.circular(AppRadius.lg),
            border: Border.all(color: palette.line),
          ),
          child: Column(
            children: [
              _ToggleRow(
                title: '唤醒后自动解锁',
                value: settings.autoUnlockOnWake,
                onChanged: (value) => ref
                    .read(settingsProvider.notifier)
                    .setAutoUnlockOnWake(value),
              ),
              _ToggleRow(
                title: '跟随电脑开电视',
                subtitle: '发送唤醒时同步按下遥控器电源键',
                value: settings.tvFollowPcPowerOn,
                onChanged: (value) => ref
                    .read(settingsProvider.notifier)
                    .setTvFollowPcPowerOn(value),
              ),
              _ToggleRow(
                title: '跟随电脑关电视',
                subtitle: '关机或睡眠时同步按下遥控器电源键',
                value: settings.tvFollowPcPowerOff,
                onChanged: (value) => ref
                    .read(settingsProvider.notifier)
                    .setTvFollowPcPowerOff(value),
                isLast: true,
              ),
            ],
          ),
        ),
      ],
    );
  }

  void _push(BuildContext context, Widget page) {
    final route = switch (page.runtimeType) {
      const (ProcessesTab) => AppRoutes.processes,
      const (SettingsTab) => AppRoutes.settings,
      const (ShortcutsScreen) => AppRoutes.shortcuts,
      const (TvRemoteScreen) => AppRoutes.tvRemote,
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
      minHeight: 58,
      showChevron: true,
      enabled: onTap != null,
      onTap: onTap,
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      border: Border(
        bottom: BorderSide(color: isLast ? Colors.transparent : palette.line),
      ),
    );
  }
}

class _ToggleRow extends StatelessWidget {
  const _ToggleRow({
    required this.title,
    required this.value,
    required this.onChanged,
    this.subtitle,
    this.isLast = false,
  });

  final String title;
  final String? subtitle;
  final bool value;
  final ValueChanged<bool> onChanged;
  final bool isLast;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppListRow(
      title: title,
      subtitle: subtitle,
      minHeight: 56,
      trailing: AppToggle(value: value, onChanged: onChanged),
      padding: const EdgeInsets.symmetric(vertical: 8),
      border: Border(
        bottom: BorderSide(color: isLast ? Colors.transparent : palette.line),
      ),
    );
  }
}
