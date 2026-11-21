/// 快捷键方案编辑器：用软键盘按钮自由组合修饰键与主键。
library;

import 'package:flutter/material.dart';

import '../providers/settings_models.dart';
import '../services/shortcut_mapper.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import '../widgets/app_sheet.dart';

class ShortcutPresetEditResult {
  const ShortcutPresetEditResult({
    required this.preset,
    required this.homeShortcuts,
  });
  final ShortcutPreset preset;
  final List<String> homeShortcuts;
}

Future<ShortcutPresetEditResult?> showShortcutPresetSheet(
  BuildContext context, {
  ShortcutPreset? preset,
  List<String> homeShortcuts = const [],
}) => showAppSheet<ShortcutPresetEditResult>(
  context,
  isScrollControlled: true,
  useSafeArea: true,
  builder: (_) => FractionallySizedBox(
    heightFactor: 0.92,
    child: _ShortcutPresetSheet(preset: preset, homeShortcuts: homeShortcuts),
  ),
);

class _ShortcutPresetSheet extends StatefulWidget {
  const _ShortcutPresetSheet({this.preset, required this.homeShortcuts});
  final ShortcutPreset? preset;
  final List<String> homeShortcuts;

  @override
  State<_ShortcutPresetSheet> createState() => _ShortcutPresetSheetState();
}

class _ShortcutPresetSheetState extends State<_ShortcutPresetSheet> {
  static const _maxKeys = 12;
  late final TextEditingController _nameController;
  final _keys = <String>[];
  final _modifiers = <String>{};
  late final Set<String> _homeShortcuts;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.preset?.name ?? '');
    _keys.addAll(widget.preset?.shortcuts ?? const []);
    _homeShortcuts = widget.homeShortcuts.toSet();
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  bool get _canSave =>
      _nameController.text.trim().isNotEmpty && _keys.isNotEmpty;

  void _toggleModifier(String modifier) => setState(() {
    if (!_modifiers.add(modifier)) _modifiers.remove(modifier);
  });

  void _chooseKey(String key) => setState(() {
    final label = ShortcutMapper.compose(_modifiers, key);
    if (!_keys.contains(label) && _keys.length < _maxKeys) _keys.add(label);
    _modifiers.clear();
  });

  void _toggleHome(String label) => setState(() {
    if (!_homeShortcuts.remove(label) && _homeShortcuts.length < 12) {
      _homeShortcuts.add(label);
    }
  });

  void _removeKey(String label) => setState(() {
    _keys.remove(label);
  });

  void _save() {
    if (!_canSave) return;
    final base = widget.preset;
    Navigator.of(context).pop(
      ShortcutPresetEditResult(
        preset: ShortcutPreset(
          id: base?.id ?? 'custom_${DateTime.now().millisecondsSinceEpoch}',
          name: _nameController.text.trim(),
          shortcuts: List.unmodifiable(_keys),
          builtin: base?.builtin ?? false,
        ),
        homeShortcuts: normalizeHomeShortcuts(_homeShortcuts),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 0, 18, 4),
          child: Text(
            widget.preset == null ? '新增快捷键方案' : '编辑快捷键方案',
            style: AppText.cardTitle.copyWith(color: palette.textPrimary),
          ),
        ),
        Expanded(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(18, 8, 18, 8),
            children: [
              TextField(
                controller: _nameController,
                onChanged: (_) => setState(() {}),
                decoration: const InputDecoration(
                  labelText: '方案名称',
                  hintText: '例如：剪辑软件',
                  isDense: true,
                ),
              ),
              const SizedBox(height: 14),
              SectionHeading(
                title: '已创建快捷键',
                trailing: Text('${_keys.length}/$_maxKeys'),
              ),
              if (_keys.isEmpty)
                Text('先选择修饰键，再点击一个主键。', style: AppText.meta)
              else
                ..._keys.map(
                  (label) => _ShortcutRow(
                    label: label,
                    onHome: () => _toggleHome(label),
                    onRemove: () => _removeKey(label),
                    onHomeEnabled: _homeShortcuts.contains(label),
                  ),
                ),
              const SizedBox(height: 14),
              SectionHeading(
                title: '修饰键',
                trailing: Text(
                  _modifiers.isEmpty ? '可选' : _modifiers.join(' + '),
                  style: AppText.meta.copyWith(color: AppColors.primary),
                ),
              ),
              Wrap(
                spacing: 8,
                children: [
                  for (final modifier in ShortcutMapper.modifierLabels)
                    FilterChip(
                      label: Text(modifier),
                      selected: _modifiers.contains(modifier),
                      onSelected: (_) => _toggleModifier(modifier),
                    ),
                ],
              ),
              for (final group in ShortcutMapper.keyGroups.entries) ...[
                const SizedBox(height: 14),
                SectionHeading(title: group.key),
                Wrap(
                  spacing: 7,
                  runSpacing: 7,
                  children: [
                    for (final key in group.value)
                      OutlinedButton(
                        onPressed: () => _chooseKey(key),
                        child: Text(key),
                      ),
                  ],
                ),
              ],
              const SizedBox(height: 18),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 4, 18, 12),
          child: Row(
            children: [
              Expanded(
                child: SecondaryButton(
                  label: '取消',
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: PrimaryButton(
                  label: '保存',
                  onPressed: _canSave ? _save : null,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _ShortcutRow extends StatelessWidget {
  const _ShortcutRow({
    required this.label,
    required this.onHome,
    required this.onRemove,
    required this.onHomeEnabled,
  });
  final String label;
  final VoidCallback onHome;
  final VoidCallback onRemove;
  final bool onHomeEnabled;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 6),
    child: Row(
      children: [
        Expanded(child: Text(label, style: AppText.body)),
        IconBtn(
          icon: onHomeEnabled ? Icons.home_rounded : Icons.home_outlined,
          tooltip: onHomeEnabled ? '移出首页' : '加入首页',
          onPressed: onHome,
          size: 34,
          iconSize: 17,
        ),
        IconBtn(
          icon: Icons.close_rounded,
          tooltip: '删除',
          onPressed: onRemove,
          size: 34,
          iconSize: 17,
        ),
      ],
    ),
  );
}
