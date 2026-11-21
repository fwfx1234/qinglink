/// S9 快捷键方案（对应设计稿 `S09-shortcuts-*.png`）。
///
/// 结构：顶栏（返回 + 标题 + 新增）→ 同步说明 → 方案卡列表（名称 / 说明 /
/// 快捷键行 / 使用中标记）。点按方案切换，长按可编辑或删除。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import 'shortcut_preset_sheet.dart';
import '../widgets/app_sheet.dart';

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

  /// 删除方案：至少保留一个，避免输入面板没有可用按键。
  Future<void> _deletePreset(ShortcutPreset preset) async {
    final settings = ref.read(settingsProvider);
    if (settings.shortcutPresets.length <= 1) {
      context.showInfoSnackBar('至少保留一个快捷键方案');
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('删除方案'),
        content: Text('删除「${preset.name}」？该方案的按键将从首页移除。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('取消'),
          ),
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(
              '删除',
              style: TextStyle(color: context.palette.danger),
            ),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    final updated = settings.shortcutPresets
        .where((item) => item.id != preset.id)
        .toList();
    await ref.read(settingsProvider.notifier).saveShortcutPresets(updated);
    if (settings.activeShortcutPresetId == preset.id && updated.isNotEmpty) {
      await ref
          .read(settingsProvider.notifier)
          .setActiveShortcutPreset(updated.first.id);
    }
    await ref
        .read(settingsProvider.notifier)
        .saveHomeShortcuts(_pruneHomeShortcuts(updated, settings.homeShortcuts));
    if (mounted) context.showSuccessSnackBar('已删除方案');
  }

  /// 长按方案卡：编辑或删除。
  Future<void> _showPresetActions(ShortcutPreset preset) async {
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
              preset.name,
              style: AppText.cardTitle.copyWith(color: palette.textPrimary),
            ),
            const SizedBox(height: AppSpacing.md),
            AppListRow(
              icon: Icons.edit_outlined,
              title: '编辑方案',
              showChevron: true,
              onTap: () => Navigator.of(sheetContext).pop('edit'),
            ),
            AppListRow(
              icon: Icons.delete_outline_rounded,
              title: '删除方案',
              iconTone: palette.danger,
              showChevron: true,
              onTap: () => Navigator.of(sheetContext).pop('delete'),
            ),
          ],
        ),
      ),
    );
    if (!mounted) return;
    if (action == 'edit') await _editPreset(preset);
    if (action == 'delete') await _deletePreset(preset);
  }

  List<String> _pruneHomeShortcuts(
    List<ShortcutPreset> presets,
    List<String> homeShortcuts,
  ) {
    final available = presets.expand((item) => item.shortcuts).toSet();
    return homeShortcuts.where(available.contains).toList(growable: false);
  }

  /// 切换当前方案：首页「快捷操作」同步换成该方案的按键（最多 8 个）。
  Future<void> _selectPreset(ShortcutPreset preset) async {
    final notifier = ref.read(settingsProvider.notifier);
    try {
      await notifier.setActiveShortcutPreset(preset.id);
      await notifier.saveHomeShortcuts(
        preset.shortcuts.take(8).toList(growable: false),
      );
      if (mounted) context.showSuccessSnackBar('已切换到「${preset.name}」');
    } catch (error) {
      if (mounted) context.showErrorSnackBar('切换方案失败: $error');
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final settings = ref.watch(settingsProvider);
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
                      '快捷键方案',
                      style: AppText.pageTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                  ),
                  SizedBox(
                    height: 36,
                    child: FilledButton.icon(
                      onPressed: _addPreset,
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
              '选中方案后，首页「快捷操作」按键会同步切换',
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
            const SizedBox(height: AppSpacing.md),
            for (final preset in settings.shortcutPresets)
              Padding(
                padding: const EdgeInsets.only(bottom: AppSpacing.md),
                child: _PresetCard(
                  preset: preset,
                  active: preset.id == settings.activeShortcutPresetId,
                  description: _descriptions[preset.id] ?? '自定义工作流',
                  onSelect: () => _selectPreset(preset),
                  onLongPress: () => _showPresetActions(preset),
                ),
              ),
            const SizedBox(height: AppSpacing.xs),
            Text(
              '长按方案可编辑或删除。',
              style: AppText.meta.copyWith(color: palette.textSubtle),
            ),
          ],
        ),
      ),
    );
  }
}

/// 方案卡：名称 + 使用中标记 + 说明 + 快捷键行。
class _PresetCard extends StatelessWidget {
  const _PresetCard({
    required this.preset,
    required this.active,
    required this.description,
    required this.onSelect,
    required this.onLongPress,
  });

  final ShortcutPreset preset;
  final bool active;
  final String description;
  final VoidCallback onSelect;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return GestureDetector(
      onLongPress: onLongPress,
      child: AppCard(
        minHeight: 88,
        radius: AppRadius.lg,
        padding: const EdgeInsets.all(AppMetrics.cardPadding),
        color: active ? palette.brandTint.withValues(alpha: 0.45) : null,
        borderColor: active ? palette.brand.withValues(alpha: 0.45) : null,
        onTap: onSelect,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    preset.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AppText.sectionTitle.copyWith(
                      color: palette.textPrimary,
                    ),
                  ),
                ),
                if (active) ...[
                  const SizedBox(width: AppSpacing.sm),
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
                  ),
                ],
              ],
            ),
            const SizedBox(height: 2),
            Text(
              description,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
            if (preset.shortcuts.isNotEmpty) ...[
              const SizedBox(height: AppSpacing.md),
              Text(
                preset.shortcuts.join(' · '),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: AppText.label.copyWith(
                  color: palette.textMuted,
                  letterSpacing: 0.2,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
