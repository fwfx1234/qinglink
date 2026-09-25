import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import 'ir_widgets.dart';

/// 按键与信号配置页：编辑当前遥控器的载波/时序数据并测试发送。
class IrCommandScreen extends ConsumerStatefulWidget {
  const IrCommandScreen({super.key, required this.profileId});

  final String profileId;

  @override
  ConsumerState<IrCommandScreen> createState() => _IrCommandScreenState();
}

class _IrCommandScreenState extends ConsumerState<IrCommandScreen> {
  late String _profileId = widget.profileId;

  IrRemoteProfile? _profile(IrRemoteState state) {
    for (final profile in state.profiles) {
      if (profile.id == _profileId) return profile;
    }
    return state.activeProfile;
  }

  Future<void> _testCommand(IrCommand command) async {
    try {
      await ref.read(irRemoteProvider.notifier).transmit(command);
      if (mounted) showIrMessage(context, '已发送“${command.name}”');
    } catch (error) {
      if (mounted) showIrMessage(context, error.toString());
    }
  }

  Future<void> _editCommand(IrRemoteProfile profile, {IrCommand? command}) async {
    final result = await showDialog<IrCommand>(
      context: context,
      builder: (context) => IrCommandDialog(command: command),
    );
    if (result == null || !mounted) return;
    final notifier = ref.read(irRemoteProvider.notifier);
    final error = notifier.validateCommand(result);
    if (error != null) {
      showIrMessage(context, error);
      return;
    }
    final commands = [...profile.commands];
    final index = commands.indexWhere((item) => item.id == result.id);
    if (index >= 0) {
      commands[index] = result;
    } else {
      commands.add(result);
    }
    final candidate = profile.copyWith(commands: commands);
    final remoteError = notifier.validateProfile(
      candidate,
      macros: ref.read(irRemoteProvider).macros,
    );
    if (remoteError != null) {
      showIrMessage(context, remoteError);
      return;
    }
    await _saveProfile(candidate);
  }

  Future<void> _deleteCommand(IrRemoteProfile profile, IrCommand command) async {
    final referenced = ref
        .read(irRemoteProvider)
        .macros
        .any(
          (macro) =>
              macro.profileId == profile.id &&
              macro.steps.any((step) => step.commandId == command.id),
        );
    if (referenced) {
      showIrMessage(context, '该按键仍被控制流程引用，请先编辑流程');
      return;
    }
    await _saveProfile(
      profile.copyWith(
        commands: profile.commands
            .where((value) => value.id != command.id)
            .toList(),
      ),
    );
  }

  Future<void> _saveProfile(IrRemoteProfile profile) async {
    final profiles = ref
        .read(irRemoteProvider)
        .profiles
        .map((item) => item.id == profile.id ? profile : item)
        .toList();
    await ref.read(irRemoteProvider.notifier).saveProfiles(profiles);
  }

  Future<void> _copyProfile(IrRemoteProfile source) async {
    final id = 'remote_${DateTime.now().microsecondsSinceEpoch}';
    final copy = source.copyWith(
      id: id,
      name: '${source.name} 副本',
      builtin: false,
    );
    await ref.read(irRemoteProvider.notifier).saveProfiles([
      ...ref.read(irRemoteProvider).profiles,
      copy,
    ]);
    await ref.read(irRemoteProvider.notifier).selectProfile(id);
    if (mounted) setState(() => _profileId = id);
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final state = ref.watch(irRemoteProvider);
    final profile = _profile(state);
    if (profile == null) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(18, 20, 18, 28),
          children: [
            PageHeading(
              eyebrow: profile.name,
              title: '按键与信号',
              trailing: IconBtn(
                icon: Icons.close_rounded,
                tooltip: '返回',
                onPressed: () => Navigator.of(context).maybePop(),
              ),
            ),
            if (profile.builtin)
              Padding(
                padding: const EdgeInsets.only(bottom: 14),
                child: AppCard(
                  radius: AppRadius.lg,
                  padding: const EdgeInsets.all(13),
                  color: AppColors.cyan.withValues(alpha: 0.07),
                  borderColor: AppColors.cyan.withValues(alpha: 0.3),
                  boxShadow: const [],
                  child: Row(
                    children: [
                      const Icon(
                        Icons.lock_outline_rounded,
                        size: 18,
                        color: AppColors.cyan,
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Text(
                          '内置遥控器为只读，复制后可自由编辑。',
                          style: AppText.meta.copyWith(
                            color: palette.textMuted,
                          ),
                        ),
                      ),
                      TextButton(
                        onPressed: () => _copyProfile(profile),
                        child: const Text('复制'),
                      ),
                    ],
                  ),
                ),
              ),
            SectionHeading(
              title: '按键列表',
              trailing: Text(
                '${profile.commands.length} 个',
                style: AppText.meta.copyWith(color: palette.textSubtle),
              ),
            ),
            if (profile.commands.isEmpty)
              const EmptyState(
                icon: Icons.keyboard_alt_outlined,
                title: '暂无按键',
                message: '请录入载波频率和微秒时序。',
              )
            else
              ...profile.commands.map(
                (command) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: AppCard(
                    radius: AppRadius.lg,
                    padding: EdgeInsets.zero,
                    child: AppListRow(
                      title: command.name,
                      subtitle:
                          '${command.carrierHz} Hz · ${command.patternUs.length} 个脉冲 · 重复 ${command.repeatCount} 次',
                      icon: Icons.radio_button_checked_rounded,
                      iconTone: AppColors.cyan,
                      tile: true,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 6,
                      ),
                      trailing: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconBtn(
                            icon: Icons.send_rounded,
                            tooltip: '测试发送',
                            size: 36,
                            iconSize: 17,
                            onPressed: () => _testCommand(command),
                          ),
                          if (!profile.builtin)
                            IconBtn(
                              icon: Icons.edit_outlined,
                              tooltip: '编辑',
                              size: 36,
                              iconSize: 17,
                              onPressed: () =>
                                  _editCommand(profile, command: command),
                            ),
                          if (!profile.builtin)
                            IconBtn(
                              icon: Icons.delete_outline,
                              tooltip: '删除',
                              size: 36,
                              iconSize: 17,
                              color: AppColors.red,
                              onPressed: () =>
                                  _deleteCommand(profile, command),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            const SizedBox(height: 6),
            SecondaryButton(
              label: '添加按键',
              icon: Icons.add_rounded,
              expand: true,
              onPressed: profile.builtin
                  ? null
                  : () => _editCommand(profile),
            ),
          ],
        ),
      ),
    );
  }
}
