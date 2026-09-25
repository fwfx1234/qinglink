import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

/// 遥控器管理页：切换、新增、复制和删除遥控器配置。
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
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(18, 20, 18, 28),
          children: [
            PageHeading(
              title: '遥控器管理',
              trailing: IconBtn(
                icon: Icons.close_rounded,
                tooltip: '返回',
                onPressed: () => Navigator.of(context).maybePop(),
              ),
            ),
            ...state.profiles.map((profile) {
              final active = profile.id == state.activeProfile?.id;
              final macroCount = state.macros
                  .where((macro) => macro.profileId == profile.id)
                  .length;
              return Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: AppCard(
                  radius: AppRadius.lg,
                  padding: EdgeInsets.zero,
                  color: active
                      ? AppColors.primary.withValues(alpha: 0.06)
                      : palette.surface,
                  borderColor: active ? AppColors.primaryDark : palette.line,
                  child: AppListRow(
                    title: profile.name,
                    subtitle:
                        '${profile.commands.length} 个按键 · $macroCount 个流程${profile.builtin ? ' · 内置' : ''}',
                    icon: Icons.tv_rounded,
                    iconTone: active ? AppColors.primary : AppColors.cyan,
                    tile: true,
                    minHeight: 64,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 6,
                    ),
                    onTap: () =>
                        ref.read(irRemoteProvider.notifier).selectProfile(
                          profile.id,
                        ),
                    trailing: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        if (active) ...[
                          const StatusTag('使用中'),
                          const SizedBox(width: 4),
                        ],
                        IconBtn(
                          icon: Icons.copy_rounded,
                          tooltip: '复制',
                          size: 36,
                          iconSize: 17,
                          onPressed: () => _copyProfile(ref, profile),
                        ),
                        if (!profile.builtin)
                          IconBtn(
                            icon: Icons.delete_outline,
                            tooltip: '删除',
                            size: 36,
                            iconSize: 17,
                            color: AppColors.red,
                            onPressed: () =>
                                _deleteProfile(context, ref, profile),
                          ),
                      ],
                    ),
                  ),
                ),
              );
            }),
            const SizedBox(height: 6),
            SecondaryButton(
              label: '新增遥控器',
              icon: Icons.add_rounded,
              expand: true,
              onPressed: () => _addProfile(context, ref),
            ),
          ],
        ),
      ),
    );
  }
}
