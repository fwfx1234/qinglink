import 'package:flutter/material.dart';

import '../../models/ir_remote.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

/// 统一在 IR 各页面弹出一条提示。
///
/// 会先清掉上一条，避免连续发送时提示排队堆积、看起来一直不消失。
/// [error] 为 true 时停留更久，方便用户看清失败原因。
void showIrMessage(BuildContext context, String text, {bool error = false}) {
  if (!context.mounted) return;
  final messenger = ScaffoldMessenger.of(context);
  messenger.clearSnackBars();
  messenger.showSnackBar(
    SnackBar(
      content: Text(text),
      duration: Duration(milliseconds: error ? 2800 : 1000),
      behavior: SnackBarBehavior.floating,
      margin: const EdgeInsets.fromLTRB(16, 0, 16, 20),
    ),
  );
}

/// 手机不支持红外时的提示条；支持时不占空间。
class IrCapabilityBanner extends StatelessWidget {
  const IrCapabilityBanner({super.key, required this.supported});

  final bool supported;

  @override
  Widget build(BuildContext context) {
    if (supported) return const SizedBox.shrink();
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(13),
      color: AppColors.warning.withValues(alpha: 0.08),
      borderColor: AppColors.warning.withValues(alpha: 0.35),
      boxShadow: const [],
      child: Row(
        children: [
          const Icon(Icons.warning_amber_rounded, size: 19, color: AppColors.warning),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  '当前手机不支持红外发射',
                  style: AppText.cardTitle.copyWith(
                    color: context.palette.textPrimary,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  '仍可编辑遥控器数据；请使用带红外发射器的 Android 手机发送。',
                  style: AppText.meta.copyWith(color: context.palette.textMuted),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 遥控按键：默认 tonal 样式，[accent] 用于电源/确认等重点键。
class IrRemoteKey extends StatelessWidget {
  const IrRemoteKey({
    super.key,
    required this.label,
    required this.onPressed,
    this.icon,
    this.accent = false,
    this.height = 44,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final bool accent;
  final double height;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final enabled = onPressed != null;
    final color = accent ? AppColors.primary : palette.textPrimary;
    return SizedBox(
      height: height,
      child: FilledButton.tonal(
        onPressed: onPressed,
        style: FilledButton.styleFrom(
          backgroundColor: accent
              ? AppColors.primary.withValues(alpha: 0.16)
              : palette.surface2,
          foregroundColor: enabled ? color : palette.textSubtle,
          disabledBackgroundColor: palette.surface2.withValues(alpha: 0.55),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.sm),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 6),
        ),
        child: icon == null
            ? Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
              )
            : Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 17),
                  const SizedBox(width: 3),
                  Flexible(
                    child: Text(
                      label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ),
      ),
    );
  }
}

/// 标准遥控面板：电源/信号源/静音 + 方向键 + 音量频道 + 数字键盘。
///
/// 面板本身不关心数据来源，只通过 [onCommand] 回传被按下的按键。
class IrRemotePanel extends StatelessWidget {
  const IrRemotePanel({
    super.key,
    required this.profile,
    required this.capability,
    required this.onCommand,
  });

  final IrRemoteProfile profile;
  final bool capability;
  final ValueChanged<IrCommand> onCommand;

  static const _panelIds = {
    'power',
    'input',
    'mute',
    'up',
    'down',
    'left',
    'right',
    'ok',
    'volume_up',
    'volume_down',
    'channel_up',
    'channel_down',
    'menu',
    'back',
    'num_0',
    'num_1',
    'num_2',
    'num_3',
    'num_4',
    'num_5',
    'num_6',
    'num_7',
    'num_8',
    'num_9',
  };

  IrCommand? _command(String id) {
    for (final command in profile.commands) {
      if (command.id == id) return command;
    }
    return null;
  }

  Widget _key(String id, String label, {IconData? icon, bool accent = false}) {
    final command = _command(id);
    return IrRemoteKey(
      label: label,
      icon: icon,
      accent: accent,
      onPressed: command == null ? null : () => onCommand(command),
    );
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      radius: AppRadius.xl,
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              TileIcon(
                icon: Icons.tv_rounded,
                color: AppColors.cyan,
                size: 34,
                radius: 9,
                iconSize: 18,
              ),
              const SizedBox(width: 9),
              Expanded(
                child: Text(
                  profile.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.cardTitle.copyWith(
                    color: palette.textPrimary,
                  ),
                ),
              ),
              StatusDot(
                color: capability ? AppColors.success : AppColors.warning,
                glow: capability,
              ),
              const SizedBox(width: 6),
              Text(
                capability ? '红外就绪' : '仅编辑',
                style: AppText.meta.copyWith(
                  color: capability ? AppColors.success : AppColors.warning,
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),
          Row(
            children: [
              Expanded(
                child: _key(
                  'power',
                  '电源',
                  icon: Icons.power_settings_new_rounded,
                  accent: true,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: _key('input', '信号源', icon: Icons.input_rounded),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: _key('mute', '静音', icon: Icons.volume_off_rounded),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                child: Column(
                  children: [
                    _key('up', '上', icon: Icons.keyboard_arrow_up_rounded),
                    const SizedBox(height: 6),
                    Row(
                      children: [
                        Expanded(
                          child: _key(
                            'left',
                            '左',
                            icon: Icons.keyboard_arrow_left_rounded,
                          ),
                        ),
                        const SizedBox(width: 6),
                        Expanded(child: _key('ok', '确认', accent: true)),
                        const SizedBox(width: 6),
                        Expanded(
                          child: _key(
                            'right',
                            '右',
                            icon: Icons.keyboard_arrow_right_rounded,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 6),
                    _key('down', '下', icon: Icons.keyboard_arrow_down_rounded),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              SizedBox(
                width: 74,
                child: Column(
                  children: [
                    _key('volume_up', '音量 +', icon: Icons.add_rounded),
                    const SizedBox(height: 6),
                    _key('volume_down', '音量 -', icon: Icons.remove_rounded),
                    const SizedBox(height: 6),
                    _key(
                      'channel_up',
                      '频道 +',
                      icon: Icons.keyboard_arrow_up_rounded,
                    ),
                    const SizedBox(height: 6),
                    _key(
                      'channel_down',
                      '频道 -',
                      icon: Icons.keyboard_arrow_down_rounded,
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          GridView.count(
            crossAxisCount: 3,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            childAspectRatio: 2.5,
            mainAxisSpacing: 6,
            crossAxisSpacing: 6,
            children: [
              for (var index = 1; index <= 9; index++)
                _key('num_$index', '$index'),
              _key('menu', '菜单'),
              _key('num_0', '0'),
              _key('back', '返回'),
            ],
          ),
        ],
      ),
    );
  }

  /// 面板未覆盖的按键，供「更多按键」区域使用。
  static List<IrCommand> extraCommands(IrRemoteProfile profile) => profile.commands
      .where((command) => !_panelIds.contains(command.id))
      .toList(growable: false);
}

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
            ...List.generate(_commandIds.length, (index) {
              final command = _commandIds[index];
              return Row(
                children: [
                  Expanded(
                    child: DropdownButtonFormField<String>(
                      initialValue:
                          widget.profile.commands.any(
                            (item) => item.id == command,
                          )
                          ? command
                          : null,
                      decoration: InputDecoration(
                        labelText: '第 ${index + 1} 步',
                      ),
                      items: widget.profile.commands
                          .map(
                            (item) => DropdownMenuItem(
                              value: item.id,
                              child: Text(item.name),
                            ),
                          )
                          .toList(),
                      onChanged: (value) =>
                          setState(() => _commandIds[index] = value ?? command),
                    ),
                  ),
                  const SizedBox(width: 8),
                  SizedBox(
                    width: 78,
                    child: TextField(
                      controller: _delays[index],
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(labelText: '延时 ms'),
                    ),
                  ),
                  IconButton(
                    onPressed: () => _move(index, -1),
                    icon: const Icon(Icons.arrow_upward),
                    tooltip: '上移',
                  ),
                  IconButton(
                    onPressed: () => _move(index, 1),
                    icon: const Icon(Icons.arrow_downward),
                    tooltip: '下移',
                  ),
                  IconButton(
                    onPressed: _commandIds.length == 1
                        ? null
                        : () => setState(() {
                            _commandIds.removeAt(index);
                            _delays[index].dispose();
                            _delays.removeAt(index);
                          }),
                    icon: const Icon(Icons.remove_circle_outline),
                    tooltip: '删除步骤',
                  ),
                ],
              );
            }),
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                onPressed: _commandIds.length >= 32
                    ? null
                    : () => setState(() {
                        _commandIds.add(widget.profile.commands.first.id);
                        _delays.add(TextEditingController(text: '300'));
                      }),
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
