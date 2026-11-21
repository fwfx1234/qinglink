/// S14 控制流程（对应设计稿 `S14-flows-*.png`）。
///
/// 卡片列表：流程名 + 运行按钮 + 步骤序列。长按可编辑或删除。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import 'ir_widgets.dart';
import '../../widgets/app_sheet.dart';

class IrMacroScreen extends ConsumerStatefulWidget {
  const IrMacroScreen({super.key, required this.profileId});

  final String profileId;

  @override
  ConsumerState<IrMacroScreen> createState() => _IrMacroScreenState();
}

class _IrMacroScreenState extends ConsumerState<IrMacroScreen> {
  late final String _profileId = widget.profileId;

  IrRemoteProfile? _profile(IrRemoteState state) {
    for (final profile in state.profiles) {
      if (profile.id == _profileId) return profile;
    }
    return state.activeProfile;
  }

  Future<void> _editMacro(IrRemoteProfile profile, {IrMacro? macro}) async {
    if (profile.commands.isEmpty) {
      showIrMessage(context, '请先在「按键与信号」中添加按键');
      return;
    }
    final result = await showDialog<IrMacro>(
      context: context,
      builder: (context) => IrMacroDialog(profile: profile, macro: macro),
    );
    if (result == null || !mounted) return;
    final macros = [...ref.read(irRemoteProvider).macros];
    final index = macros.indexWhere((item) => item.id == result.id);
    if (index >= 0) {
      macros[index] = result;
    } else {
      macros.add(result);
    }
    final error = ref
        .read(irRemoteProvider.notifier)
        .validateProfile(profile, macros: macros);
    if (error != null) {
      showIrMessage(context, error);
      return;
    }
    await ref.read(irRemoteProvider.notifier).saveMacros(macros);
  }

  Future<void> _runMacro(IrMacro macro) async {
    try {
      await ref.read(irRemoteProvider.notifier).runMacro(macro);
    } catch (error) {
      if (mounted) showIrMessage(context, error.toString());
    }
  }

  Future<void> _deleteMacro(IrMacro macro) async {
    await ref
        .read(irRemoteProvider.notifier)
        .saveMacros(
          ref
              .read(irRemoteProvider)
              .macros
              .where((item) => item.id != macro.id)
              .toList(),
        );
  }

  /// 长按流程卡：编辑或删除。
  Future<void> _showMacroActions(
    IrRemoteProfile profile,
    IrMacro macro,
  ) async {
    final palette = context.palette;
    final action = await showAppSheet<String>(
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
              macro.name,
              style: AppText.cardTitle.copyWith(color: palette.textPrimary),
            ),
            const SizedBox(height: AppSpacing.md),
            AppListRow(
              icon: Icons.edit_outlined,
              title: '编辑流程',
              showChevron: true,
              onTap: () => Navigator.of(sheetContext).pop('edit'),
            ),
            AppListRow(
              icon: Icons.delete_outline_rounded,
              title: '删除流程',
              iconTone: palette.danger,
              showChevron: true,
              onTap: () => Navigator.of(sheetContext).pop('delete'),
            ),
          ],
        ),
      ),
    );
    if (!mounted) return;
    if (action == 'edit') await _editMacro(profile, macro: macro);
    if (action == 'delete') await _deleteMacro(macro);
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final state = ref.watch(irRemoteProvider);
    final profile = _profile(state);
    if (profile == null) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    final macros = state.macros
        .where((macro) => macro.profileId == profile.id)
        .toList();
    return Scaffold(
      backgroundColor: palette.background,
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(
            AppMetrics.pagePadding,
            0,
            AppMetrics.pagePadding,
            AppSpacing.xxl,
          ),
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
                      '控制流程',
                      style: AppText.pageTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                  ),
                  SizedBox(
                    height: 36,
                    child: FilledButton.icon(
                      onPressed: () => _editMacro(profile),
                      icon: const Icon(Icons.add_rounded, size: 18),
                      label: const Text('新增'),
                      style: FilledButton.styleFrom(
                        backgroundColor: palette.brand,
                        foregroundColor: palette.onBrand,
                        padding: const EdgeInsets.symmetric(horizontal: 12),
                        textStyle: AppText.label.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(AppRadius.sm),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: AppSpacing.sm),
            Text(
              '一条流程按顺序发送多个按键，步骤间隔按每一步设定的延时执行',
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
            const SizedBox(height: AppSpacing.md),
            if (macros.isEmpty)
              EmptyState(
                icon: Icons.account_tree_outlined,
                title: '暂无控制流程',
                message: '把开机、信号源、音量等按键串成一条流程，一键完成。',
                action: SecondaryButton(
                  label: '新增流程',
                  icon: Icons.add_rounded,
                  onPressed: () => _editMacro(profile),
                ),
              )
            else
              for (final macro in macros)
                Padding(
                  padding: const EdgeInsets.only(bottom: AppSpacing.md),
                  child: _MacroCard(
                    macro: macro,
                    steps: _describe(profile, macro),
                    running: state.runningMacroId == macro.id,
                    onRun: () => _runMacro(macro),
                    onStop: () =>
                        ref.read(irRemoteProvider.notifier).cancelMacro(),
                    onLongPress: () => _showMacroActions(profile, macro),
                  ),
                ),
          ],
        ),
      ),
    );
  }

  /// 步骤序列文案：按键之间插入「等待 Ns」，便于一眼看清节奏。
  String _describe(IrRemoteProfile profile, IrMacro macro) {
    if (macro.steps.isEmpty) return '空流程';
    final parts = <String>[];
    for (final step in macro.steps) {
      final name = profile.commands
          .where((command) => command.id == step.commandId)
          .firstOrNull
          ?.name;
      parts.add(name ?? '失效按键');
      final seconds = step.delayAfterMs / 1000;
      if (seconds >= 0.5) {
        final text = seconds % 1 == 0
            ? seconds.toStringAsFixed(0)
            : seconds.toStringAsFixed(1);
        parts.add('等待 $text s');
      }
    }
    return parts.join(' → ');
  }
}

/// 流程卡：名称 + 运行/停止按钮 + 步骤序列。
class _MacroCard extends StatelessWidget {
  const _MacroCard({
    required this.macro,
    required this.steps,
    required this.running,
    required this.onRun,
    required this.onStop,
    required this.onLongPress,
  });

  final IrMacro macro;
  final String steps;
  final bool running;
  final VoidCallback onRun;
  final VoidCallback onStop;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return GestureDetector(
      onLongPress: onLongPress,
      child: AppCard(
        radius: AppRadius.lg,
        padding: const EdgeInsets.all(AppMetrics.cardPadding),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    macro.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AppText.sectionTitle.copyWith(
                      color: palette.textPrimary,
                    ),
                  ),
                ),
                const SizedBox(width: AppSpacing.sm),
                SizedBox(
                  height: 36,
                  child: FilledButton(
                    onPressed: running ? onStop : onRun,
                    style: FilledButton.styleFrom(
                      backgroundColor: running
                          ? palette.dangerTint
                          : palette.surface2,
                      foregroundColor: running
                          ? palette.danger
                          : palette.textPrimary,
                      padding: const EdgeInsets.symmetric(horizontal: 16),
                      textStyle: AppText.label.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(AppRadius.sm),
                        side: BorderSide(
                          color: running
                              ? palette.danger.withValues(alpha: 0.4)
                              : palette.line,
                        ),
                      ),
                    ),
                    child: Text(running ? '停止' : '运行'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: AppSpacing.sm),
            Text(
              steps,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
          ],
        ),
      ),
    );
  }
}
