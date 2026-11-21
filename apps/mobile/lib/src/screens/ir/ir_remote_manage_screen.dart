/// S11 遥控器管理（对应设计稿 `S11-remote-manage-*.png`）。
///
/// 单卡行式列表：名称 + 按键/流程统计 + 右侧「使用中」标记或复制/删除，
/// 长按任意行把该遥控器设为使用中。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

class IrRemoteManageScreen extends ConsumerWidget {
  const IrRemoteManageScreen({super.key});

  Future<void> _addProfile(BuildContext context, WidgetRef ref) async {
    final controller = TextEditingController(text: '我的遥控器');
    final name = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('新增遥控器'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(labelText: '名称'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text.trim()),
            child: const Text('创建'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (name == null || name.isEmpty || !context.mounted) return;
    final id = 'remote_${DateTime.now().microsecondsSinceEpoch}';
    await ref.read(irRemoteProvider.notifier).saveProfiles([
      ...ref.read(irRemoteProvider).profiles,
      IrRemoteProfile(id: id, name: name, commands: const []),
    ]);
    await ref.read(irRemoteProvider.notifier).selectProfile(id);
  }

  Future<void> _copyProfile(WidgetRef ref, IrRemoteProfile source) async {
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
  }

  Future<void> _deleteProfile(
    BuildContext context,
    WidgetRef ref,
    IrRemoteProfile profile,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('删除“${profile.name}”？'),
        content: const Text('关联的控制流程也会一并删除。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            style: FilledButton.styleFrom(
              backgroundColor: context.palette.danger,
              foregroundColor: context.palette.onBrand,
            ),
            child: const Text('删除'),
          ),
        ],
      ),
    );
    if (confirmed != true || !context.mounted) return;
    final state = ref.read(irRemoteProvider);
    final notifier = ref.read(irRemoteProvider.notifier);
    await notifier.saveProfiles(
      state.profiles.where((item) => item.id != profile.id).toList(),
    );
    await notifier.saveMacros(
      state.macros.where((macro) => macro.profileId != profile.id).toList(),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final state = ref.watch(irRemoteProvider);
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
                      '遥控器管理',
                      style: AppText.pageTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                  ),
                  SizedBox(
                    height: 36,
                    child: FilledButton.icon(
                      onPressed: () => _addProfile(context, ref),
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
              '一个遥控器对应一套红外按键映射，电视快捷面板会使用「使用中」的那套',
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
            const SizedBox(height: AppSpacing.md),
            if (state.profiles.isEmpty)
              EmptyState(
                icon: Icons.settings_remote_rounded,
                title: '还没有遥控器',
                message: '新建一个遥控器后即可录制按键信号。',
                action: SecondaryButton(
                  label: '新增遥控器',
                  icon: Icons.add_rounded,
                  onPressed: () => _addProfile(context, ref),
                ),
              )
            else
              AppCard(
                radius: AppRadius.lg,
                padding: EdgeInsets.zero,
                child: Column(
                  children: [
                    for (var i = 0; i < state.profiles.length; i++) ...[
                      if (i > 0)
                        Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: AppMetrics.cardPadding,
                          ),
                          child: Divider(height: 1, color: palette.line),
                        ),
                      _RemotelRow(
                        profile: state.profiles[i],
                        active:
                            state.profiles[i].id == state.activeProfile?.id,
                        macroCount: state.macros
                            .where(
                              (macro) =>
                                  macro.profileId == state.profiles[i].id,
                            )
                            .length,
                        onActivate: () => ref
                            .read(irRemoteProvider.notifier)
                            .selectProfile(state.profiles[i].id),
                        onCopy: () =>
                            _copyProfile(ref, state.profiles[i]),
                        onDelete: () => _deleteProfile(
                          context,
                          ref,
                          state.profiles[i],
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            const SizedBox(height: AppSpacing.lg),
            Center(
              child: Text(
                '长按遥控器可设为「使用中」',
                style: AppText.meta.copyWith(color: palette.textSubtle),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 遥控器行：名称 + 统计 + 使用中标记 / 复制·删除。
class _RemotelRow extends StatelessWidget {
  const _RemotelRow({
    required this.profile,
    required this.active,
    required this.macroCount,
    required this.onActivate,
    required this.onCopy,
    required this.onDelete,
  });

  final IrRemoteProfile profile;
  final bool active;
  final int macroCount;
  final VoidCallback onActivate;
  final VoidCallback onCopy;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final macroLabel = macroCount == 0 ? '无流程' : '$macroCount 条控制流程';
    return GestureDetector(
      onLongPress: onActivate,
      child: InkWell(
        onTap: onActivate,
        child: Container(
          constraints: const BoxConstraints(minHeight: AppMetrics.listRow + 8),
          padding: const EdgeInsets.symmetric(
            horizontal: AppMetrics.cardPadding,
            vertical: AppSpacing.md,
          ),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      profile.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.sectionTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '${profile.commands.length} 个按键 · $macroLabel',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.meta.copyWith(color: palette.textMuted),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: AppSpacing.sm),
              if (active)
                Container(
                  height: 26,
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: palette.brandTint,
                    borderRadius: BorderRadius.circular(AppRadius.pill),
                  ),
                  child: Text(
                    '使用中',
                    style: AppText.label.copyWith(
                      color: palette.brand,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                )
              else ...[
                IconBtn(
                  icon: Icons.copy_all_outlined,
                  tooltip: '复制',
                  size: 40,
                  iconSize: 18,
                  radius: AppRadius.sm,
                  tonal: true,
                  onPressed: onCopy,
                ),
                if (!profile.builtin) ...[
                  const SizedBox(width: AppSpacing.sm),
                  IconBtn(
                    icon: Icons.delete_outline_rounded,
                    tooltip: '删除',
                    size: 40,
                    iconSize: 18,
                    radius: AppRadius.sm,
                    tint: palette.danger,
                    onPressed: onDelete,
                  ),
                ],
              ],
            ],
          ),
        ),
      ),
    );
  }
}
