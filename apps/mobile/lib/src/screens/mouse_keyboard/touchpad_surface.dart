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
  });

  /// 手势识别控制器。
  final TouchpadInputController controller;

  /// 未连接时禁用全部指针事件。
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final scrollZoneWidth = _scrollZoneWidthFor(constraints.maxWidth);
        return _TouchpadSurface(
          enabled: enabled,
          scrollZoneWidth: scrollZoneWidth,
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
    required this.onPointerDown,
    required this.onPointerMove,
    required this.onPointerUp,
    required this.onPointerCancel,
  });

  final bool enabled;
  final double scrollZoneWidth;
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

/// 触控板中央的图标与提示文案。
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
          size: 24,
          color: enabled ? palette.textSubtle : palette.line,
        ),
        const SizedBox(height: 6),
        Text(
          '滑动移动',
          style: AppText.meta.copyWith(
            fontSize: 12,
            color: enabled ? palette.textSubtle : palette.line,
          ),
        ),
      ],
    );
  }
}

/// 触控板右缘的滚动区提示。
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
            color: palette.surface2.withValues(alpha: .82),
            border: Border(left: BorderSide(color: palette.line)),
          ),
          child: Center(
            child: Icon(
              Icons.unfold_more_rounded,
              size: 20,
              color: palette.textSubtle,
            ),
          ),
        ),
      ),
    );
  }
}

double _scrollZoneWidthFor(double surfaceWidth) {
  return (surfaceWidth * 0.12).clamp(36.0, 44.0);
}
