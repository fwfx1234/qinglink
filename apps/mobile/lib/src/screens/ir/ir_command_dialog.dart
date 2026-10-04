/// 按键编辑对话框：名称、载波频率、微秒时序与重复次数。
library;

import 'package:flutter/material.dart';

import '../../models/ir_remote.dart';

/// 添加/编辑按键（载波 + 微秒时序）对话框。
class IrCommandDialog extends StatefulWidget {
  const IrCommandDialog({super.key, this.command});

  final IrCommand? command;

  @override
  State<IrCommandDialog> createState() => _IrCommandDialogState();
}

class _IrCommandDialogState extends State<IrCommandDialog> {
  late final TextEditingController _name;
  late final TextEditingController _carrier;
  late final TextEditingController _pattern;
  late final TextEditingController _repeat;

  @override
  void initState() {
    super.initState();
    final command = widget.command;
    _name = TextEditingController(text: command?.name ?? '电源切换');
    _carrier = TextEditingController(text: '${command?.carrierHz ?? 38000}');
    _pattern = TextEditingController(
      text: command?.patternUs.join(', ') ?? '9000, 4500, 560, 560',
    );
    _repeat = TextEditingController(text: '${command?.repeatCount ?? 1}');
  }

  @override
  void dispose() {
    _name.dispose();
    _carrier.dispose();
    _pattern.dispose();
    _repeat.dispose();
    super.dispose();
  }

  void _save() {
    final values = _pattern.text
        .trim()
        .split(RegExp(r'[\s,]+'))
        .where((value) => value.isNotEmpty)
        .map(int.tryParse)
        .toList();
    if (values.any((value) => value == null)) return;
    Navigator.pop(
      context,
      IrCommand(
        id:
            widget.command?.id ??
            'command_${DateTime.now().microsecondsSinceEpoch}',
        name: _name.text.trim(),
        carrierHz: int.tryParse(_carrier.text.trim()) ?? 0,
        patternUs: values.cast<int>(),
        repeatCount: int.tryParse(_repeat.text.trim()) ?? 0,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.command == null ? '添加按键' : '编辑按键'),
    content: SingleChildScrollView(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _name,
            decoration: const InputDecoration(labelText: '按键名称'),
          ),
          TextField(
            controller: _carrier,
            keyboardType: TextInputType.number,
            decoration: const InputDecoration(labelText: '载波频率 (Hz)'),
          ),
          TextField(
            controller: _pattern,
            maxLines: 4,
            keyboardType: TextInputType.number,
            decoration: const InputDecoration(
              labelText: '微秒时序',
              hintText: '9000, 4500, 560, 560',
            ),
          ),
          TextField(
            controller: _repeat,
            keyboardType: TextInputType.number,
            decoration: const InputDecoration(labelText: '重复次数 (1-5)'),
          ),
        ],
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
