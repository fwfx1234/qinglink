/// 控制流程编辑对话框：名称 + 有序步骤（按键与延时）。
library;

import 'package:flutter/material.dart';

import '../../models/ir_remote.dart';

/// 添加/编辑控制流程对话框。
class IrMacroDialog extends StatefulWidget {
  const IrMacroDialog({super.key, required this.profile, this.macro});

  final IrRemoteProfile profile;
  final IrMacro? macro;

  @override
  State<IrMacroDialog> createState() => _IrMacroDialogState();
}

class _IrMacroDialogState extends State<IrMacroDialog> {
  late final TextEditingController _name;
  late List<String> _commandIds;
  late List<TextEditingController> _delays;

  @override
  void initState() {
    super.initState();
    _name = TextEditingController(text: widget.macro?.name ?? '新流程');
    final initial = widget.macro?.steps ?? const <IrMacroStep>[];
    _commandIds = initial.isEmpty
        ? [widget.profile.commands.first.id]
        : initial.map((step) => step.commandId).toList();
    _delays =
        (initial.isEmpty
                ? const <IrMacroStep>[
                    IrMacroStep(commandId: '', delayAfterMs: 300),
                  ]
                : initial)
            .map((step) => TextEditingController(text: '${step.delayAfterMs}'))
            .toList();
  }

  @override
  void dispose() {
    _name.dispose();
    for (final controller in _delays) {
      controller.dispose();
    }
    super.dispose();
  }

  /// 把第 [index] 步沿 [direction]（-1 上移 / 1 下移）移动一位。
  void _move(int index, int direction) {
    final target = index + direction;
    if (target < 0 || target >= _commandIds.length) return;
    setState(() {
      final command = _commandIds.removeAt(index);
      _commandIds.insert(target, command);
      final delay = _delays.removeAt(index);
      _delays.insert(target, delay);
    });
  }

  /// 追加一个默认步骤（按键取第一个，延时 300ms）。
  void _addStep() {
    setState(() {
      _commandIds.add(widget.profile.commands.first.id);
      _delays.add(TextEditingController(text: '300'));
    });
  }

  /// 删除第 [index] 步，并释放它的延时输入控制器。
  void _removeStep(int index) {
    setState(() {
      _commandIds.removeAt(index);
      _delays[index].dispose();
      _delays.removeAt(index);
    });
  }

  void _save() {
    if (_name.text.trim().isEmpty ||
        _commandIds.isEmpty ||
        _commandIds.length > 32) {
      return;
    }
    final delays = _delays
        .map((controller) => int.tryParse(controller.text.trim()))
        .toList();
    if (delays.any((value) => value == null || value < 0 || value > 5000)) {
      return;
    }
    Navigator.pop(
      context,
      IrMacro(
        id:
            widget.macro?.id ??
            'macro_${DateTime.now().microsecondsSinceEpoch}',
        name: _name.text.trim(),
        profileId: widget.profile.id,
        steps: List.generate(
          _commandIds.length,
          (index) => IrMacroStep(
            commandId: _commandIds[index],
            delayAfterMs: delays[index]!,
          ),
        ),
      ),
    );
  }

  /// 构建第 [index] 步的编辑行。
  Widget _buildStep(int index) {
    final commandId = _commandIds[index];
    return _MacroStepRow(
      profile: widget.profile,
      index: index,
      commandId: commandId,
      delay: _delays[index],
      canRemove: _commandIds.length != 1,
      onChanged: (value) =>
          setState(() => _commandIds[index] = value ?? commandId),
      onMoveUp: () => _move(index, -1),
      onMoveDown: () => _move(index, 1),
      onRemove: () => _removeStep(index),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.macro == null ? '新增控制流程' : '编辑控制流程'),
    content: SizedBox(
      width: 460,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: _name,
              decoration: const InputDecoration(labelText: '流程名称'),
            ),
            const SizedBox(height: 12),
            ...List.generate(_commandIds.length, _buildStep),
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                onPressed: _commandIds.length >= 32 ? null : _addStep,
                icon: const Icon(Icons.add),
                label: const Text('添加步骤'),
              ),
            ),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('取消'),
      ),
      FilledButton(onPressed: _save, child: const Text('保存')),
    ],
  );
}

/// 控制流程中的单步：按键下拉框、延时输入与上移/下移/删除按钮。
class _MacroStepRow extends StatelessWidget {
  const _MacroStepRow({
    required this.profile,
    required this.index,
    required this.commandId,
    required this.delay,
    required this.canRemove,
    required this.onChanged,
    required this.onMoveUp,
    required this.onMoveDown,
    required this.onRemove,
  });

  final IrRemoteProfile profile;
  final int index;
  final String commandId;
  final TextEditingController delay;

  /// 仅剩一步时不允许删除。
  final bool canRemove;
  final ValueChanged<String?> onChanged;
  final VoidCallback onMoveUp;
  final VoidCallback onMoveDown;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: DropdownButtonFormField<String>(
            initialValue: profile.commands.any((item) => item.id == commandId)
                ? commandId
                : null,
            decoration: InputDecoration(labelText: '第 ${index + 1} 步'),
            items: profile.commands
                .map(
                  (item) => DropdownMenuItem(
                    value: item.id,
                    child: Text(item.name),
                  ),
                )
                .toList(),
            onChanged: onChanged,
          ),
        ),
        const SizedBox(width: 8),
        SizedBox(
          width: 78,
          child: TextField(
            controller: delay,
            keyboardType: TextInputType.number,
            decoration: const InputDecoration(labelText: '延时 ms'),
          ),
        ),
        IconButton(
          onPressed: onMoveUp,
          icon: const Icon(Icons.arrow_upward),
          tooltip: '上移',
        ),
        IconButton(
          onPressed: onMoveDown,
          icon: const Icon(Icons.arrow_downward),
          tooltip: '下移',
        ),
        IconButton(
          onPressed: canRemove ? onRemove : null,
          icon: const Icon(Icons.remove_circle_outline),
          tooltip: '删除步骤',
        ),
      ],
    );
  }
}
