/// S10 电视遥控面板（对应设计稿 `S10-tv-remote-*.png`）。
///
/// 自上而下：圆形方向盘（含确认键）→ 音量/电源/静音/频道 → 返回·首页·菜单。
/// 面板只负责布局，按键是否可用由 [profile] 中是否存在对应指令决定。
library;

import 'package:flutter/material.dart';

import '../../models/ir_remote.dart';
import '../../theme/app_theme.dart';
import '../../utils/tv_power_follow.dart';
import '../../widgets/app_ui.dart';

/// 电视遥控面板：方向盘 + 音量频道摇杆 + 底部三键。
class TvRemotePad extends StatelessWidget {
  const TvRemotePad({
    super.key,
    required this.profile,
    required this.onCommand,
  });

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: Center(
            child: LayoutBuilder(
              builder: (context, constraints) {
                // 方向盘宽度取屏幕的一半左右（设计稿是 390 画布上的 195），
                // 上限 200：再大就会把摇杆挤到屏幕底部。
                final size = [
                  constraints.maxWidth,
                  constraints.maxHeight,
                  MediaQuery.sizeOf(context).width * 0.56,
                  200.0,
                ].reduce((a, b) => a < b ? a : b);
                return _DirectionWheel(
                  size: size,
                  profile: profile,
                  onCommand: onCommand,
                );
              },
            ),
          ),
        ),
        const SizedBox(height: AppSpacing.xl),
        _ControlRow(profile: profile, onCommand: onCommand),
        const SizedBox(height: AppSpacing.xl),
        _BottomKeys(profile: profile, onCommand: onCommand),
      ],
    );
  }
}

/// 圆形方向盘：外圈 + 上下左右箭头 + 中央确认键。
class _DirectionWheel extends StatelessWidget {
  const _DirectionWheel({
    required this.size,
    required this.profile,
    required this.onCommand,
  });

  final double size;
  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final center = size * 0.40;
    final arrowExtent = size * 0.20;
    return SizedBox(
      width: size,
      height: size,
      child: Stack(
        alignment: Alignment.center,
        children: [
          Container(
            decoration: BoxDecoration(
              color: palette.surface,
              shape: BoxShape.circle,
              border: Border.all(color: palette.line),
            ),
          ),
          _WheelArrow(
            alignment: const Alignment(0, -0.74),
            extent: arrowExtent,
            icon: Icons.keyboard_arrow_up_rounded,
            profile: profile,
            commandId: 'up',
            onCommand: onCommand,
          ),
          _WheelArrow(
            alignment: const Alignment(0, 0.74),
            extent: arrowExtent,
            icon: Icons.keyboard_arrow_down_rounded,
            profile: profile,
            commandId: 'down',
            onCommand: onCommand,
          ),
          _WheelArrow(
            alignment: const Alignment(-0.74, 0),
            extent: arrowExtent,
            icon: Icons.keyboard_arrow_left_rounded,
            profile: profile,
            commandId: 'left',
            onCommand: onCommand,
          ),
          _WheelArrow(
            alignment: const Alignment(0.74, 0),
            extent: arrowExtent,
            icon: Icons.keyboard_arrow_right_rounded,
            profile: profile,
            commandId: 'right',
            onCommand: onCommand,
          ),
          _WheelOk(
            size: center,
            profile: profile,
            onCommand: onCommand,
          ),
        ],
      ),
    );
  }
}

/// 方向盘上的方向箭头：命中区比图形大，保证可点。
class _WheelArrow extends StatelessWidget {
  const _WheelArrow({
    required this.alignment,
    required this.extent,
    required this.icon,
    required this.profile,
    required this.commandId,
    required this.onCommand,
  });

  final Alignment alignment;
  final double extent;
  final IconData icon;
  final IrRemoteProfile profile;
  final String commandId;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final command = findIrCommand(profile, commandId);
    return Align(
      alignment: alignment,
      child: SizedBox(
        width: extent,
        height: extent,
        child: InkResponse(
          onTap: command == null ? null : () => onCommand(command),
          radius: extent / 2,
          child: Icon(
            icon,
            size: extent * 0.62,
            color: command == null ? palette.textSubtle : palette.textPrimary,
          ),
        ),
      ),
    );
  }
}

/// 方向盘中央的确认键。
class _WheelOk extends StatelessWidget {
  const _WheelOk({
    required this.size,
    required this.profile,
    required this.onCommand,
  });

  final double size;
  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final command = findIrCommand(profile, 'ok');
    return SizedBox(
      width: size,
      height: size,
      child: Material(
        color: palette.surface2,
        shape: CircleBorder(side: BorderSide(color: palette.line)),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: command == null ? null : () => onCommand(command),
          child: Center(
            child: Text(
              '确定',
              style: AppText.body.copyWith(
                color: command == null
                    ? palette.textSubtle
                    : palette.textPrimary,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// 中段控制区：音量摇杆 ·（电源 + 静音）· 频道摇杆。
class _ControlRow extends StatelessWidget {
  const _ControlRow({required this.profile, required this.onCommand});

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        _Rocker(
          profile: profile,
          onCommand: onCommand,
          upperCommandId: 'volume_up',
          lowerCommandId: 'volume_down',
          upperIcon: Icons.add_rounded,
          lowerIcon: Icons.remove_rounded,
        ),
        const SizedBox(width: AppSpacing.xl),
        Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            _RoundKey(
              profile: profile,
              onCommand: onCommand,
              commandId: 'power',
              icon: Icons.power_settings_new_rounded,
              size: 60,
              danger: true,
            ),
            const SizedBox(height: AppSpacing.md),
            _RoundKey(
              profile: profile,
              onCommand: onCommand,
              commandId: 'mute',
              icon: Icons.volume_off_rounded,
              size: 52,
            ),
          ],
        ),
        const SizedBox(width: AppSpacing.xl),
        _Rocker(
          profile: profile,
          onCommand: onCommand,
          upperCommandId: 'channel_up',
          lowerCommandId: 'channel_down',
          upperIcon: Icons.keyboard_arrow_up_rounded,
          lowerIcon: Icons.keyboard_arrow_down_rounded,
        ),
      ],
    );
  }
}

/// 竖向摇杆：上半 + 下半，中间一条分隔线。
class _Rocker extends StatelessWidget {
  const _Rocker({
    required this.profile,
    required this.onCommand,
    required this.upperCommandId,
    required this.lowerCommandId,
    required this.upperIcon,
    required this.lowerIcon,
  });

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;
  final String upperCommandId;
  final String lowerCommandId;
  final IconData upperIcon;
  final IconData lowerIcon;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      width: 60,
      height: 84,
      decoration: BoxDecoration(
        color: palette.surface2,
        border: Border.all(color: palette.line),
        borderRadius: BorderRadius.circular(AppRadius.pill),
      ),
      child: Column(
        children: [
          Expanded(
            child: _RockerHalf(
              icon: upperIcon,
              command: findIrCommand(profile, upperCommandId),
              onCommand: onCommand,
            ),
          ),
          Container(
            height: 1,
            margin: const EdgeInsets.symmetric(horizontal: 14),
            color: palette.line,
          ),
          Expanded(
            child: _RockerHalf(
              icon: lowerIcon,
              command: findIrCommand(profile, lowerCommandId),
              onCommand: onCommand,
            ),
          ),
        ],
      ),
    );
  }
}

class _RockerHalf extends StatelessWidget {
  const _RockerHalf({
    required this.icon,
    required this.command,
    required this.onCommand,
  });

  final IconData icon;
  final IrCommand? command;
  final ValueChanged<IrCommand> onCommand;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return InkWell(
      onTap: command == null ? null : () => onCommand(command!),
      borderRadius: BorderRadius.circular(AppRadius.pill),
      child: Center(
        child: Icon(
          icon,
          size: 26,
          color: command == null ? palette.textSubtle : palette.textPrimary,
        ),
      ),
    );
  }
}

/// 圆形重点键：电源（危险色）与静音。
class _RoundKey extends StatelessWidget {
  const _RoundKey({
    required this.profile,
    required this.onCommand,
    required this.commandId,
    required this.icon,
    required this.size,
    this.danger = false,
  });

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;
  final String commandId;
  final IconData icon;
  final double size;
  final bool danger;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final command = findIrCommand(profile, commandId);
    final enabled = command != null;
    final color = !enabled
        ? palette.textSubtle
        : danger
        ? palette.danger
        : palette.textPrimary;
    return SizedBox(
      width: size,
      height: size,
      child: Material(
        color: danger ? palette.dangerTint : palette.surface2,
        shape: CircleBorder(
          side: BorderSide(
            color: danger
                ? palette.danger.withValues(alpha: 0.35)
                : palette.line,
          ),
        ),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: enabled ? () => onCommand(command) : null,
          child: Icon(icon, size: size * 0.4, color: color),
        ),
      ),
    );
  }
}

/// 底部三键：返回 / 首页 / 菜单。
class _BottomKeys extends StatelessWidget {
  const _BottomKeys({required this.profile, required this.onCommand});

  final IrRemoteProfile profile;
  final ValueChanged<IrCommand> onCommand;

  static const _keys = [('back', '返回'), ('home', '首页'), ('menu', '菜单')];

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      children: [
        for (var i = 0; i < _keys.length; i++) ...[
          if (i > 0) const SizedBox(width: AppSpacing.md),
          Expanded(
            child: Builder(
              builder: (context) {
                final command = findIrCommand(profile, _keys[i].$1);
                return SizedBox(
                  height: 52,
                  child: Material(
                    color: palette.surface2,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(AppRadius.md),
                      side: BorderSide(color: palette.line),
                    ),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(AppRadius.md),
                      onTap: command == null
                          ? null
                          : () => onCommand(command),
                      child: Center(
                        child: Text(
                          _keys[i].$2,
                          style: AppText.body.copyWith(
                            color: command == null
                                ? palette.textSubtle
                                : palette.textPrimary,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
        ],
      ],
    );
  }
}
