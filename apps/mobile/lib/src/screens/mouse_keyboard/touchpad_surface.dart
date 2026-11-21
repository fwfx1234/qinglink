import 'package:flutter/material.dart';

import '../../services/touchpad_input_controller.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

/// 触控板手势区。
///
/// 把布局约束换算成右缘滚动区宽度，并把指针事件转发给 [TouchpadInputController]。
class TouchpadSection extends StatelessWidget {
  const TouchpadSection({
    super.key,
    required this.controller,
    required this.enabled,
    this.pressedLabel = '',
  });

  /// 手势识别控制器。
  final TouchpadInputController controller;

  /// 未连接时禁用全部指针事件。
  final bool enabled;

  /// PC 侧鼠标正被按住时的角标文案；空串表示没有按键按下。
  final String pressedLabel;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final scrollZoneWidth = _scrollZoneWidthFor(constraints.maxWidth);
        return _TouchpadSurface(
          enabled: enabled,
          scrollZoneWidth: scrollZoneWidth,
          pressedLabel: pressedLabel,
          onPointerDown: (event) => controller.pointerDown(
            event.pointer,
            event.localPosition,
            inScrollZone:
                event.localPosition.dx >=
                constraints.maxWidth - scrollZoneWidth,
          ),
          onPointerMove: (event) =>
              controller.pointerMove(event.pointer, event.localPosition),
          onPointerUp: (event) => controller.pointerUp(event.pointer),
          onPointerCancel: (event) => controller.pointerCancel(event.pointer),
        );
      },
    );
  }
}

class _TouchpadSurface extends StatelessWidget {
  const _TouchpadSurface({
    required this.enabled,
    required this.scrollZoneWidth,
    required this.pressedLabel,
    required this.onPointerDown,
    required this.onPointerMove,
    required this.onPointerUp,
    required this.onPointerCancel,
  });

  final bool enabled;
  final double scrollZoneWidth;
  final String pressedLabel;
  final PointerDownEventListener onPointerDown;
  final PointerMoveEventListener onPointerMove;
  final PointerUpEventListener onPointerUp;
  final PointerCancelEventListener onPointerCancel;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Semantics(
      label: enabled ? '触控板' : '触控板，未连接',
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: enabled ? onPointerDown : null,
        onPointerMove: enabled ? onPointerMove : null,
        onPointerUp: enabled ? onPointerUp : null,
        onPointerCancel: enabled ? onPointerCancel : null,
        child: DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [palette.touchpadStart, palette.touchpadEnd],
            ),
            borderRadius: BorderRadius.circular(AppRadius.lg),
            border: Border.all(
              color: enabled ? palette.touchpadBorder : palette.line,
            ),
          ),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(AppRadius.lg),
            child: Stack(
              children: [
                Center(child: _TouchpadHint(enabled: enabled)),
                if (enabled && pressedLabel.isNotEmpty)
                  Positioned(top: 8, left: 8, child: _PressedBadge(label: pressedLabel)),
                if (enabled && scrollZoneWidth > 0)
                  Positioned(
                    top: 0,
                    bottom: 0,
                    right: 0,
                    width: scrollZoneWidth,
                    child: const _ScrollZoneHint(),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// 触控板中央的图标与手势提示（设计稿：一句主提示 + 一行手势说明）。
class _TouchpadHint extends StatelessWidget {
  const _TouchpadHint({required this.enabled});

  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.touch_app_rounded,
          size: 26,
          color: enabled ? palette.textMuted : palette.line,
        ),
        const SizedBox(height: 10),
        Text(
          '指尖滑动移动指针',
          style: AppText.body.copyWith(
            color: enabled ? palette.textMuted : palette.line,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          '轻点左键 · 长按右键 · 长按后拖动拖选',
          textAlign: TextAlign.center,
          style: AppText.meta.copyWith(color: palette.textSubtle),
        ),
      ],
    );
  }
}

/// 触控板左上角的"PC 侧鼠标正被按住"角标。
class _PressedBadge extends StatelessWidget {
  const _PressedBadge({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return IgnorePointer(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: AppColors.primary.withValues(alpha: .18),
          borderRadius: BorderRadius.circular(999),
          border: Border.all(color: AppColors.primary.withValues(alpha: .45)),
        ),
        child: Text(
          label,
          style: AppText.meta.copyWith(
            fontSize: 11,
            color: AppColors.primary,
            fontWeight: FontWeight.w700,
          ),
        ),
      ),
    );
  }
}

/// 触控板右缘的滚动条提示：上下箭头 + 「滚动」标签（设计稿的窄条）。
class _ScrollZoneHint extends StatelessWidget {
  const _ScrollZoneHint();

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return IgnorePointer(
      child: Semantics(
        label: '右侧滚动区',
        child: Container(
          decoration: BoxDecoration(
            color: palette.surface.withValues(alpha: .55),
            border: Border(left: BorderSide(color: palette.line)),
            borderRadius: const BorderRadius.horizontal(
              right: Radius.circular(AppRadius.lg),
            ),
          ),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.unfold_more_rounded,
                size: 20,
                color: palette.textSubtle,
              ),
              const SizedBox(height: 4),
              Text(
                '滚动',
                style: AppText.tiny.copyWith(color: palette.textSubtle),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

double _scrollZoneWidthFor(double surfaceWidth) {
  return (surfaceWidth * 0.12).clamp(36.0, 44.0);
}
