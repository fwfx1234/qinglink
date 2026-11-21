/// 通用按钮：图标按钮、主按钮、次按钮。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// 顶栏/卡片通用图标按钮，默认 44×44。
///
/// [tonal] 为真时使用 `bg.icon-btn` 底座（设计稿里的顶栏图标按钮）；
/// [tint] 传色时改用该色的浅底（危险动作的关闭按钮）。
class IconBtn extends StatelessWidget {
  const IconBtn({
    super.key,
    required this.icon,
    this.onPressed,
    this.tooltip,
    this.color,
    this.size = AppMetrics.minTap,
    this.iconSize = 20,
    this.radius = AppRadius.sm,
    this.background,
    this.tonal = false,
    this.tint,
  });

  final IconData icon;
  final VoidCallback? onPressed;
  final String? tooltip;
  final Color? color;
  final double size;
  final double iconSize;
  final double radius;
  final Color? background;
  final bool tonal;
  final Color? tint;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final fill = background ?? tint?.withValues(alpha: 0.12);
    return SizedBox(
      width: size,
      height: size,
      child: IconButton(
        tooltip: tooltip,
        onPressed: onPressed,
        icon: Icon(icon, size: iconSize),
        color: color ?? (tint ?? palette.textMuted),
        disabledColor: palette.textSubtle.withValues(alpha: 0.4),
        padding: EdgeInsets.zero,
        style: IconButton.styleFrom(
          backgroundColor: fill ?? (tonal ? palette.iconBtn : Colors.transparent),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(radius),
          ),
        ),
      ),
    );
  }
}

/// 主按钮：品牌色填充，默认高度 52。
class PrimaryButton extends StatelessWidget {
  const PrimaryButton({
    super.key,
    required this.label,
    this.onPressed,
    this.icon,
    this.height = AppMetrics.primaryButton,
    this.expand = false,
    this.small = false,
    this.danger = false,
    this.busy = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final double height;
  final bool expand;
  final bool small;
  final bool danger;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final enabled = onPressed != null && !busy;
    final background = danger ? palette.danger : palette.brand;
    // 危险色在暗色下是亮红、亮色下是深红，两种都配 `on-brand` 才够对比度。
    final foreground = palette.onBrand;
    final child = busy
        ? SizedBox(
            width: 18,
            height: 18,
            child: CircularProgressIndicator(
              strokeWidth: 2,
              color: foreground,
            ),
          )
        : Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (icon != null) ...[
                Icon(icon, size: 18),
                const SizedBox(width: 8),
              ],
              Flexible(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          );
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(AppRadius.md),
        boxShadow: enabled
            ? [
                BoxShadow(
                  color: background.withValues(alpha: 0.28),
                  blurRadius: 18,
                  offset: const Offset(0, 6),
                  spreadRadius: -6,
                ),
              ]
            : null,
      ),
      child: SizedBox(
        height: height,
        width: expand ? double.infinity : null,
        child: FilledButton(
          onPressed: enabled ? onPressed : null,
          style: FilledButton.styleFrom(
            backgroundColor: background,
            foregroundColor: foreground,
            disabledBackgroundColor: background.withValues(alpha: 0.4),
            disabledForegroundColor: foreground.withValues(alpha: 0.7),
            padding: EdgeInsets.symmetric(horizontal: small ? 14 : 18),
            minimumSize: Size(0, height),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(AppRadius.md),
            ),
            textStyle: const TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}

/// 次按钮：`bg.raised` 底色 + `line.strong` 描边。
class SecondaryButton extends StatelessWidget {
  const SecondaryButton({
    super.key,
    required this.label,
    this.onPressed,
    this.icon,
    this.height = 46,
    this.expand = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final double height;
  final bool expand;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: height,
      width: expand ? double.infinity : null,
      child: OutlinedButton(
        onPressed: onPressed,
        style: OutlinedButton.styleFrom(
          backgroundColor: palette.surface2,
          foregroundColor: palette.textPrimary,
          side: BorderSide(color: palette.lineStrong),
          padding: const EdgeInsets.symmetric(horizontal: 16),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.md),
          ),
          textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (icon != null) ...[
              Icon(icon, size: 18),
              const SizedBox(width: 8),
            ],
            Flexible(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
