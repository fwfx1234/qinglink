/// 标准遥控面板外壳：状态头 + 重点键 + 方向键 + 音量频道 + 数字键盘。
library;

import 'package:flutter/material.dart';

import '../../models/ir_remote.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import 'ir_remote_key.dart';

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

  @override
  Widget build(BuildContext context) {
    return AppCard(
      radius: AppRadius.xl,
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _PanelHeader(profile: profile, capability: capability),
          const SizedBox(height: 14),
          _PowerRow(profile: profile, onCommand: onCommand),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                child: _DirectionPad(profile: profile, onCommand: onCommand),
              ),
              const SizedBox(width: 12),
              SizedBox(
                width: 74,
                child: _VolumeChannelColumn(
                  profile: profile,
                  onCommand: onCommand,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          _NumberPad(profile: profile, onCommand: onCommand),
        ],
      ),
    );
  }

  /// 面板未覆盖的按键，供「更多按键」区域使用。
  static List<IrCommand> extraCommands(IrRemoteProfile profile) => profile.commands
      .where((command) => !_panelIds.contains(command.id))
      .toList(growable: false);
}

/// 面板标题行：遥控器名称与红外能力状态。
class _PanelHeader extends StatelessWidget {
  const _PanelHeader({required this.profile, required this.capability});

  final IrRemoteProfile profile;
  final bool capability;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
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
            style: AppText.cardTitle.copyWith(color: palette.textPrimary),
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
    );
  }
}

/// 电源 / 信号源 / 静音三个重点键。
class _PowerRow extends StatelessWidget {
  const _PowerRow({required this.profile, required this.onCommand});

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: _PanelKey(
            profile: profile,
            onCommand: onCommand,
            commandId: 'power',
            label: '电源',
            icon: Icons.power_settings_new_rounded,
            accent: true,
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: _PanelKey(
            profile: profile,
            onCommand: onCommand,
            commandId: 'input',
            label: '信号源',
            icon: Icons.input_rounded,
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: _PanelKey(
            profile: profile,
            onCommand: onCommand,
            commandId: 'mute',
            label: '静音',
            icon: Icons.volume_off_rounded,
          ),
        ),
      ],
    );
  }
}

/// 方向键簇：上 / 左 / 确认 / 右 / 下。
class _DirectionPad extends StatelessWidget {
  const _DirectionPad({required this.profile, required this.onCommand});

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'up',
          label: '上',
          icon: Icons.keyboard_arrow_up_rounded,
        ),
        const SizedBox(height: 6),
        Row(
          children: [
            Expanded(
              child: _PanelKey(
                profile: profile,
                onCommand: onCommand,
                commandId: 'left',
                label: '左',
                icon: Icons.keyboard_arrow_left_rounded,
              ),
            ),
            const SizedBox(width: 6),
            Expanded(
              child: _PanelKey(
                profile: profile,
                onCommand: onCommand,
                commandId: 'ok',
                label: '确认',
                accent: true,
              ),
            ),
            const SizedBox(width: 6),
            Expanded(
              child: _PanelKey(
                profile: profile,
                onCommand: onCommand,
                commandId: 'right',
                label: '右',
                icon: Icons.keyboard_arrow_right_rounded,
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'down',
          label: '下',
          icon: Icons.keyboard_arrow_down_rounded,
        ),
      ],
    );
  }
}

/// 右侧竖排：音量加减与频道加减。
class _VolumeChannelColumn extends StatelessWidget {
  const _VolumeChannelColumn({required this.profile, required this.onCommand});

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'volume_up',
          label: '音量 +',
          icon: Icons.add_rounded,
        ),
        const SizedBox(height: 6),
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'volume_down',
          label: '音量 -',
          icon: Icons.remove_rounded,
        ),
        const SizedBox(height: 6),
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'channel_up',
          label: '频道 +',
          icon: Icons.keyboard_arrow_up_rounded,
        ),
        const SizedBox(height: 6),
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'channel_down',
          label: '频道 -',
          icon: Icons.keyboard_arrow_down_rounded,
        ),
      ],
    );
  }
}

/// 数字键网格，末尾补菜单键与返回键。
class _NumberPad extends StatelessWidget {
  const _NumberPad({required this.profile, required this.onCommand});

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    return GridView.count(
      crossAxisCount: 3,
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      childAspectRatio: 2.5,
      mainAxisSpacing: 6,
      crossAxisSpacing: 6,
      children: [
        for (var index = 1; index <= 9; index++)
          _PanelKey(
            profile: profile,
            onCommand: onCommand,
            commandId: 'num_$index',
            label: '$index',
          ),
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'menu',
          label: '菜单',
        ),
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'num_0',
          label: '0',
        ),
        _PanelKey(
          profile: profile,
          onCommand: onCommand,
          commandId: 'back',
          label: '返回',
        ),
      ],
    );
  }
}

/// 面板按键：按 [commandId] 在 [profile] 中查表，缺失时置灰不可点。
class _PanelKey extends StatelessWidget {
  const _PanelKey({
    required this.profile,
    required this.onCommand,
    required this.commandId,
    required this.label,
    this.icon,
    this.accent = false,
  });

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;
  final String commandId;
  final String label;
  final IconData? icon;
  final bool accent;

  @override
  Widget build(BuildContext context) {
    final command = _commandOf(profile, commandId);
    return IrRemoteKey(
      label: label,
      icon: icon,
      accent: accent,
      onPressed: command == null ? null : () => onCommand(command),
    );
  }
}

/// 在 [profile] 中查找 id 为 [id] 的按键；不存在时返回 null。
IrCommand? _commandOf(IrRemoteProfile profile, String id) {
  for (final command in profile.commands) {
    if (command.id == id) return command;
  }
  return null;
}
