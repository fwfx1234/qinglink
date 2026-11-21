/// S10 电视遥控（对应设计稿 `S10-tv-remote-*.png`）。
///
/// 结构：顶栏（返回 + 标题 + 三个入口）→ 遥控器选择 → 遥控面板
/// （方向盘 / 音量·电源·静音·频道 / 返回·首页·菜单）。
/// 配置类入口统一放在子页面，本页只负责发送按键。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/routes.dart';
import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import 'ir_widgets.dart';
import '../../widgets/app_sheet.dart';

class TvRemoteScreen extends ConsumerWidget {
  const TvRemoteScreen({super.key});

  void _openCommand(BuildContext context, String profileId) => context.push(
    '${AppRoutes.irCommand}?profile=${Uri.encodeComponent(profileId)}',
  );

  void _openMacro(BuildContext context, String profileId) => context.push(
    '${AppRoutes.irMacro}?profile=${Uri.encodeComponent(profileId)}',
  );

  void _openManage(BuildContext context) => context.push(AppRoutes.irManage);

  Future<void> _send(
    BuildContext context,
    WidgetRef ref,
    IrCommand command,
  ) async {
    try {
      await ref.read(irRemoteProvider.notifier).transmit(command);
      if (context.mounted) showIrMessage(context, '已发送“${command.name}”');
    } catch (error) {
      if (context.mounted) showIrMessage(context, error.toString());
    }
  }

  /// 遥控器切换弹层：列出全部方案。
  Future<void> _pickProfile(BuildContext context, WidgetRef ref) async {
    final palette = context.palette;
    final state = ref.read(irRemoteProvider);
    final selected = await showAppSheet<String>(
      context,
        builder: (sheetContext) => Container(
        padding: EdgeInsets.fromLTRB(
          AppSpacing.xl,
          AppSpacing.md,
          AppSpacing.xl,
          MediaQuery.of(sheetContext).viewPadding.bottom + AppSpacing.lg,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Center(
              child: Container(
                width: 36,
                height: 4,
                decoration: BoxDecoration(
                  color: palette.lineStrong,
                  borderRadius: BorderRadius.circular(AppRadius.pill),
                ),
              ),
            ),
            const SizedBox(height: AppSpacing.lg),
            Text(
              '选择遥控器',
              style: AppText.cardTitle.copyWith(color: palette.textPrimary),
            ),
            const SizedBox(height: AppSpacing.md),
            for (final profile in state.profiles)
              AppListRow(
                icon: Icons.settings_remote_rounded,
                iconTone: profile.id == state.activeProfile?.id
                    ? palette.brand
                    : palette.textMuted,
                title: profile.name,
                subtitle: '${profile.commands.length} 个按键'
                    '${profile.builtin ? ' · 内置' : ''}',
                trailing: profile.id == state.activeProfile?.id
                    ? Icon(
                        Icons.check_circle_rounded,
                        size: 18,
                        color: palette.brand,
                      )
                    : null,
                onTap: () => Navigator.of(sheetContext).pop(profile.id),
              ),
          ],
        ),
      ),
    );
    if (selected != null) {
      await ref.read(irRemoteProvider.notifier).selectProfile(selected);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final state = ref.watch(irRemoteProvider);
    final profile = state.activeProfile;
    if (state.loading || profile == null) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    return Scaffold(
      backgroundColor: palette.background,
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(
            AppMetrics.pagePadding,
            0,
            AppMetrics.pagePadding,
            AppSpacing.lg,
          ),
          child: Column(
            children: [
              SizedBox(
                height: AppMetrics.navHeight,
                child: Row(
                  children: [
                    IconBtn(
                      icon: Icons.chevron_left_rounded,
                      tooltip: '返回',
                      tonal: true,
                      iconSize: 22,
                      onPressed: () => Navigator.of(context).maybePop(),
                    ),
                    const SizedBox(width: AppSpacing.md),
                    Expanded(
                      child: Text(
                        '电视遥控',
                        style: AppText.pageTitle.copyWith(
                          color: palette.textPrimary,
                        ),
                      ),
                    ),
                    IconBtn(
                      icon: Icons.keyboard_alt_outlined,
                      tooltip: '按键与信号',
                      tonal: true,
                      onPressed: () => _openCommand(context, profile.id),
                    ),
                    const SizedBox(width: AppSpacing.sm),
                    IconBtn(
                      icon: Icons.account_tree_outlined,
                      tooltip: '控制流程',
                      tonal: true,
                      onPressed: () => _openMacro(context, profile.id),
                    ),
                    const SizedBox(width: AppSpacing.sm),
                    IconBtn(
                      icon: Icons.settings_remote_rounded,
                      tooltip: '遥控器管理',
                      tonal: true,
                      onPressed: () => _openManage(context),
                    ),
                  ],
                ),
              ),
              if (!state.capability.supported) ...[
                const SizedBox(height: AppSpacing.xs),
                Row(
                  children: [
                    Icon(
                      Icons.info_outline_rounded,
                      size: 14,
                      color: palette.warn,
                    ),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        '未检测到手机红外发射器，按键只会走电脑端转发',
                        style: AppText.tiny.copyWith(color: palette.warn),
                      ),
                    ),
                  ],
                ),
              ],
              const SizedBox(height: AppSpacing.sm),
              _ProfilePill(
                name: profile.name,
                onTap: () => _pickProfile(context, ref),
              ),
              const SizedBox(height: AppSpacing.lg),
              Expanded(
                child: TvRemotePad(
                  profile: profile,
                  onCommand: (command) => _send(context, ref, command),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 遥控器选择胶囊：名称 + 下拉箭头（设计稿位于页面上部居中）。
class _ProfilePill extends StatelessWidget {
  const _ProfilePill({required this.name, required this.onTap});

  final String name;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Center(
      child: Material(
        color: palette.surface2,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.pill),
          side: BorderSide(color: palette.line),
        ),
        child: InkWell(
          borderRadius: BorderRadius.circular(AppRadius.pill),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.md,
              vertical: AppSpacing.sm,
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  name,
                  style: AppText.label.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(width: 4),
                Icon(
                  Icons.keyboard_arrow_down_rounded,
                  size: 16,
                  color: palette.textMuted,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
