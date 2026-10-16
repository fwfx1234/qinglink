import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import 'shortcut_preset_sheet.dart';

class ShortcutsScreen extends ConsumerStatefulWidget {
  const ShortcutsScreen({super.key});

  @override
  ConsumerState<ShortcutsScreen> createState() => _ShortcutsScreenState();
}

class _ShortcutsScreenState extends ConsumerState<ShortcutsScreen> {
  static const _descriptions = <String, String>{
    'common': 'Tab、方向键和复制粘贴',
    'opencode': '会话、提交和中断',
    'codex': '命令、提交和中断',
  };

  Future<void> _addPreset() async {
    final settings = ref.read(settingsProvider);
    final result = await showShortcutPresetSheet(
      context,
      homeShortcuts: settings.homeShortcuts,
    );
    if (result == null || !mounted) return;
    final updated = [...settings.shortcutPresets, result.preset];
    await ref.read(settingsProvider.notifier).saveShortcutPresets(updated);
    await ref
        .read(settingsProvider.notifier)
        .saveHomeShortcuts(_pruneHomeShortcuts(updated, result.homeShortcuts));
  }

  Future<void> _editPreset(ShortcutPreset preset) async {
    final result = await showShortcutPresetSheet(
      context,
      preset: preset,
      homeShortcuts: ref.read(settingsProvider).homeShortcuts,
    );
    if (result == null || !mounted) return;
    final settings = ref.read(settingsProvider);
    final updated = settings.shortcutPresets
        .map((item) => item.id == preset.id ? result.preset : item)
        .toList();
    await ref.read(settingsProvider.notifier).saveShortcutPresets(updated);
    await ref
        .read(settingsProvider.notifier)
        .saveHomeShortcuts(_pruneHomeShortcuts(updated, result.homeShortcuts));
  }

  List<String> _pruneHomeShortcuts(
    List<ShortcutPreset> presets,
    List<String> homeShortcuts,
  ) {
    final available = presets.expand((item) => item.shortcuts).toSet();
    return homeShortcuts.where(available.contains).toList(growable: false);
  }

  Future<void> _toggleHomeShortcut(String label) async {
    final notifier = ref.read(settingsProvider.notifier);
    final current = ref.read(settingsProvider).homeShortcuts;
    final updated = [...current];
    if (!updated.remove(label)) {
      if (updated.length >= 12) {
        context.showInfoSnackBar('首页最多固定 12 个快捷键');
        return;
      }
      updated.add(label);
    }
    try {
      await notifier.saveHomeShortcuts(updated);
    } catch (error) {
      if (mounted) context.showErrorSnackBar('保存首页快捷键失败: $error');
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final settings = ref.watch(settingsProvider);
    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(18, 20, 18, 28),
          children: [
            PageHeading(
              title: '快捷键方案',
              trailing: IconBtn(
                icon: Icons.close_rounded,
                tooltip: '返回',
                onPressed: () => Navigator.of(context).maybePop(),
              ),
            ),
            ...settings.shortcutPresets.map(
              (preset) => Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: _ShortcutSetCard(
                  preset: preset,
                  active: preset.id == settings.activeShortcutPresetId,
                  description: _descriptions[preset.id] ?? '自定义工作流',
                  homeShortcuts: settings.homeShortcuts,
                  onSelect: () => ref
                      .read(settingsProvider.notifier)
                      .setActiveShortcutPreset(preset.id),
                  onEdit: () => _editPreset(preset),
                  onToggleHome: _toggleHomeShortcut,
                ),
              ),
            ),
            const SizedBox(height: 6),
            SecondaryButton(
              label: '新增方案',
              icon: Icons.add_rounded,
              expand: true,
              onPressed: _addPreset,
            ),
            const SizedBox(height: 8),
            Text(
              '输入面板显示当前方案的快捷键；点按方案中的快捷键标签可固定或取消固定首页按钮。',
              style: AppText.meta.copyWith(color: palette.textSubtle),
            ),
          ],
        ),
      ),
    );
  }
}

class _ShortcutSetCard extends StatelessWidget {
  const _ShortcutSetCard({
    required this.preset,
    required this.active,
    required this.description,
    required this.homeShortcuts,
    required this.onSelect,
    required this.onEdit,
    required this.onToggleHome,
  });

  final ShortcutPreset preset;
  final bool active;
  final String description;
  final List<String> homeShortcuts;
  final VoidCallback onSelect;
  final VoidCallback onEdit;
  final ValueChanged<String> onToggleHome;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 66,
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      color: active
          ? AppColors.primary.withValues(alpha: 0.06)
          : palette.surface,
      borderColor: active ? AppColors.primaryDark : palette.line,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              Expanded(
                child: InkWell(
                  onTap: onSelect,
                  child: Row(
                    children: [
                      TileIcon(
                        icon: preset.id == 'codex'
                            ? Icons.terminal_rounded
                            : Icons.keyboard_command_key_rounded,
                        color: AppColors.violet,
                        size: 34,
                        radius: 9,
                        iconSize: 17,
                      ),
                      const SizedBox(width: 9),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              preset.name,
                              style: AppText.body.copyWith(
                                fontWeight: FontWeight.w700,
                                color: palette.textPrimary,
                              ),
                            ),
                            const SizedBox(height: 3),
                            Text(
                              description,
                              style: AppText.meta.copyWith(
                                color: palette.textSubtle,
                              ),
                            ),
                          ],
                        ),
                      ),
                      if (active) const StatusTag('使用中'),
                    ],
                  ),
                ),
              ),
              IconBtn(
                icon: Icons.edit_outlined,
                tooltip: '编辑',
                size: 34,
                iconSize: 16,
                onPressed: onEdit,
              ),
            ],
          ),
          if (preset.shortcuts.isNotEmpty) ...[
            const SizedBox(height: 8),
            Wrap(
              spacing: 4,
              runSpacing: 4,
              children: [
                for (final shortcut in preset.shortcuts)
                  FilterChip(
                    visualDensity: VisualDensity.compact,
                    label: Text(shortcut),
                    selected: homeShortcuts.contains(shortcut),
                    onSelected: (_) => onToggleHome(shortcut),
                    avatar: Icon(
                      homeShortcuts.contains(shortcut)
                          ? Icons.home_rounded
                          : Icons.home_outlined,
                      size: 15,
                    ),
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}
