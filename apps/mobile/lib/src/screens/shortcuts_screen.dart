import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';

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
    final nameController = TextEditingController();
    final keysController = TextEditingController(text: 'Tab, Esc, Enter');
    final result = await showDialog<ShortcutPreset>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('新增快捷键方案'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: nameController,
              decoration: const InputDecoration(labelText: '方案名称'),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: keysController,
              decoration: const InputDecoration(labelText: '快捷键（逗号分隔）'),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () {
              final name = nameController.text.trim();
              final keys = keysController.text
                  .split(',')
                  .map((e) => e.trim())
                  .where((e) => e.isNotEmpty)
                  .toList();
              if (name.isNotEmpty && keys.isNotEmpty) {
                Navigator.pop(
                  context,
                  ShortcutPreset(
                    id: 'custom_${DateTime.now().millisecondsSinceEpoch}',
                    name: name,
                    shortcuts: keys,
                  ),
                );
              }
            },
            child: const Text('添加'),
          ),
        ],
      ),
    );
    nameController.dispose();
    keysController.dispose();
    if (result == null || !mounted) return;
    final settings = ref.read(settingsProvider);
    await ref.read(settingsProvider.notifier).saveShortcutPresets([
      ...settings.shortcutPresets,
      result,
    ]);
  }

  Future<void> _editPreset(ShortcutPreset preset) async {
    final controller = TextEditingController(text: preset.shortcuts.join('\n'));
    final saved = await showDialog<List<String>>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('编辑 ${preset.name}'),
        content: TextField(
          controller: controller,
          maxLines: 5,
          decoration: const InputDecoration(
            hintText: '每行一个快捷键，例如：Ctrl + Enter',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () {
              final keys = controller.text
                  .split(RegExp(r'[\n,]'))
                  .map((e) => e.trim())
                  .where((e) => e.isNotEmpty)
                  .take(8)
                  .toList();
              Navigator.pop(context, keys);
            },
            child: const Text('保存'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (saved == null || saved.isEmpty || !mounted) return;
    final settings = ref.read(settingsProvider);
    final updated = settings.shortcutPresets
        .map(
          (item) => item.id == preset.id
              ? ShortcutPreset(
                  id: item.id,
                  name: item.name,
                  shortcuts: saved,
                  builtin: item.builtin,
                )
              : item,
        )
        .toList();
    await ref.read(settingsProvider.notifier).saveShortcutPresets(updated);
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
                  onSelect: () => ref
                      .read(settingsProvider.notifier)
                      .setActiveShortcutPreset(preset.id),
                  onEdit: () => _editPreset(preset),
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
              '在输入面板上方显示这里启用的快捷键。',
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
    required this.onSelect,
    required this.onEdit,
  });

  final ShortcutPreset preset;
  final bool active;
  final String description;
  final VoidCallback onSelect;
  final VoidCallback onEdit;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 66,
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      color: active ? AppColors.primary.withValues(alpha: 0.06) : palette.surface,
      borderColor: active ? AppColors.primaryDark : palette.line,
      child: Row(
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
    );
  }
}
