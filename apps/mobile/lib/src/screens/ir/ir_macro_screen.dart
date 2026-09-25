import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import 'ir_widgets.dart';

/// 控制流程页：把多个按键串成可一键执行的宏。
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
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(18, 20, 18, 28),
          children: [
            PageHeading(
              eyebrow: profile.name,
              title: '控制流程',
              trailing: IconBtn(
                icon: Icons.close_rounded,
                tooltip: '返回',
                onPressed: () => Navigator.of(context).maybePop(),
              ),
            ),
            if (macros.isEmpty)
              const EmptyState(
                icon: Icons.playlist_play_rounded,
                title: '暂无控制流程',
                message: '将多个按键串成流程，一键完成开机等操作。',
              )
            else
              ...macros.map(
                (macro) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: AppCard(
                    radius: AppRadius.lg,
                    padding: EdgeInsets.zero,
                    child: AppListRow(
                      title: macro.name,
                      subtitle: _describe(profile, macro),
                      icon: Icons.playlist_play_rounded,
                      iconTone: AppColors.violet,
                      tile: true,
                      minHeight: 64,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 6,
                      ),
                      trailing: state.runningMacroId == macro.id
                          ? IconBtn(
                              icon: Icons.stop_rounded,
                              tooltip: '停止',
                              size: 36,
                              iconSize: 18,
                              color: AppColors.red,
                              onPressed: () =>
                                  ref.read(irRemoteProvider.notifier).cancelMacro(),
                            )
                          : Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                IconBtn(
                                  icon: Icons.play_arrow_rounded,
                                  tooltip: '执行',
                                  size: 36,
                                  iconSize: 18,
                                  color: AppColors.primary,
                                  onPressed: () => _runMacro(macro),
                                ),
                                IconBtn(
                                  icon: Icons.edit_outlined,
                                  tooltip: '编辑',
                                  size: 36,
                                  iconSize: 17,
                                  onPressed: () =>
                                      _editMacro(profile, macro: macro),
                                ),
                                IconBtn(
                                  icon: Icons.delete_outline,
                                  tooltip: '删除',
                                  size: 36,
                                  iconSize: 17,
                                  color: AppColors.red,
                                  onPressed: () => ref
                                      .read(irRemoteProvider.notifier)
                                      .saveMacros(
                                        ref
                                            .read(irRemoteProvider)
                                            .macros
                                            .where((item) => item.id != macro.id)
                                            .toList(),
                                      ),
                                ),
                              ],
                            ),
                    ),
                  ),
                ),
              ),
            const SizedBox(height: 6),
            SecondaryButton(
              label: '新增流程',
              icon: Icons.add_rounded,
              expand: true,
              onPressed: () => _editMacro(profile),
            ),
            const SizedBox(height: 8),
            Text(
              '流程按顺序发送按键，可设置每步之间的延时。',
              style: AppText.meta.copyWith(color: palette.textSubtle),
            ),
          ],
        ),
      ),
    );
  }

  String _describe(IrRemoteProfile profile, IrMacro macro) {
    if (macro.steps.isEmpty) return '空流程';
    final names = macro.steps
        .map(
          (step) =>
              profile.commands
                  .where((command) => command.id == step.commandId)
                  .firstOrNull
                  ?.name ??
              '失效按键',
        )
        .join(' → ');
    return '${macro.steps.length} 步 · $names';
  }
}
