/// S13 按键与信号（对应设计稿 `S13-keys-signals-*.png`）。
///
/// 单卡行式列表：按键名 + 协议/信号码 + 测试按钮。长按可编辑或删除。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import 'ir_widgets.dart';
import '../../widgets/app_sheet.dart';

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

  /// 长按按键行：编辑或删除（内置遥控器只能查看）。
  Future<void> _showCommandActions(
    IrRemoteProfile profile,
    IrCommand command,
  ) async {
    if (profile.builtin) {
      showIrMessage(context, '内置遥控器为只读，复制后可编辑');
      return;
    }
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
              command.name,
              style: AppText.cardTitle.copyWith(color: palette.textPrimary),
            ),
            const SizedBox(height: AppSpacing.md),
            AppListRow(
              icon: Icons.edit_outlined,
              title: '编辑按键',
              showChevron: true,
              onTap: () => Navigator.of(sheetContext).pop('edit'),
            ),
            AppListRow(
              icon: Icons.delete_outline_rounded,
              title: '删除按键',
              iconTone: palette.danger,
              showChevron: true,
              onTap: () => Navigator.of(sheetContext).pop('delete'),
            ),
          ],
        ),
      ),
    );
    if (!mounted) return;
    if (action == 'edit') await _editCommand(profile, command: command);
    if (action == 'delete') await _deleteCommand(profile, command);
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
                      '按键与信号',
                      style: AppText.pageTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                  ),
                  TextButton(
                    onPressed: profile.builtin
                        ? () => _copyProfile(profile)
                        : () => _editCommand(profile),
                    child: Text(
                      profile.builtin ? '复制后学习' : '学习',
                      style: AppText.body.copyWith(
                        color: palette.brand,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: AppSpacing.sm),
            Align(
              alignment: Alignment.centerLeft,
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: AppSpacing.md,
                  vertical: 6,
                ),
                decoration: BoxDecoration(
                  color: palette.surface2,
                  border: Border.all(color: palette.line),
                  borderRadius: BorderRadius.circular(AppRadius.pill),
                ),
                child: Text(
                  '${profile.name} · ${profile.commands.length} 键',
                  style: AppText.label.copyWith(color: palette.textMuted),
                ),
              ),
            ),
            const SizedBox(height: AppSpacing.md),
            if (profile.commands.isEmpty)
              EmptyState(
                icon: Icons.keyboard_alt_outlined,
                title: '暂无按键',
                message: '点右上角「学习」录入载波频率与微秒时序。',
                action: SecondaryButton(
                  label: '添加按键',
                  icon: Icons.add_rounded,
                  onPressed: profile.builtin
                      ? null
                      : () => _editCommand(profile),
                ),
              )
            else
              AppCard(
                radius: AppRadius.lg,
                padding: EdgeInsets.zero,
                child: Column(
                  children: [
                    for (var i = 0; i < profile.commands.length; i++) ...[
                      if (i > 0)
                        Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: AppMetrics.cardPadding,
                          ),
                          child: Divider(height: 1, color: palette.line),
                        ),
                      _CommandRow(
                        command: profile.commands[i],
                        onTest: () => _testCommand(profile.commands[i]),
                        onLongPress: () => _showCommandActions(
                          profile,
                          profile.commands[i],
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            if (profile.builtin) ...[
              const SizedBox(height: AppSpacing.md),
              Text(
                '内置遥控器为只读，点右上角「复制后学习」生成可编辑副本。',
                style: AppText.meta.copyWith(color: palette.textSubtle),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// 按键行：名称 + 协议/信号码 + 测试按钮。
class _CommandRow extends StatelessWidget {
  const _CommandRow({
    required this.command,
    required this.onTest,
    required this.onLongPress,
  });

  final IrCommand command;
  final VoidCallback onTest;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final signal = _SignalInfo.of(command);
    return GestureDetector(
      onLongPress: onLongPress,
      child: Container(
        constraints: const BoxConstraints(minHeight: 52),
        padding: const EdgeInsets.symmetric(
          horizontal: AppMetrics.cardPadding,
          vertical: AppSpacing.sm,
        ),
        child: Row(
          children: [
            Expanded(
              child: Text(
                command.name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: AppText.rowTitle.copyWith(color: palette.textPrimary),
              ),
            ),
            const SizedBox(width: AppSpacing.sm),
            Text(
              signal.label,
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
            const SizedBox(width: AppSpacing.md),
            SizedBox(
              height: 28,
              child: OutlinedButton(
                onPressed: onTest,
                style: OutlinedButton.styleFrom(
                  foregroundColor: palette.brand,
                  side: BorderSide(
                    color: palette.brand.withValues(alpha: 0.45),
                  ),
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  minimumSize: const Size(0, 28),
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(AppRadius.sm),
                  ),
                ),
                child: Text(
                  '测试',
                  style: AppText.label.copyWith(
                    color: palette.brand,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 从时序里还原协议名与信号码：标记位长度为 1 个单脉冲，空格长度判断比特。
class _SignalInfo {
  const _SignalInfo(this.label);

  final String label;

  factory _SignalInfo.of(IrCommand command) {
    final pattern = command.patternUs;
    final bytes = pattern.length;
    // NEC 系（含 RCA-38）帧：帧头 2 段 + 每比特 2 段 + 结束 2 段。
    if (bytes > 8 && (bytes - 4).isEven) {
      final bits = (bytes - 4) ~/ 2;
      final unit = pattern[2];
      if (unit > 0 && bits % 8 == 0 && bits <= 32) {
        final threshold = unit * 3;
        var value = BigInt.zero;
        var valid = true;
        for (var i = 0; i < bits; i++) {
          final space = pattern[3 + i * 2];
          if (space <= 0) {
            valid = false;
            break;
          }
          value = (value << 1) | (space > threshold ? BigInt.one : BigInt.zero);
        }
        if (valid) {
          final hex = value.toRadixString(16).toUpperCase().padLeft(
            (bits / 4).ceil(),
            '0',
          );
          final protocol = bits == 24
              ? 'RCA-38'
              : bits == 32
              ? 'NEC'
              : 'RAW';
          return _SignalInfo('$protocol · 0x$hex');
        }
      }
    }
    return _SignalInfo(
      '${(command.carrierHz / 1000).toStringAsFixed(1)} kHz · $bytes 段',
    );
  }
}
