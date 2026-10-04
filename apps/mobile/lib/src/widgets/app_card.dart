/// 基础卡片容器与图标底衬。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// 卡片容器：`surface` 背景 + `line` 边框 + 圆角。
class AppCard extends StatelessWidget {
  const AppCard({
    super.key,
    required this.child,
    this.padding,
    this.radius = AppRadius.md,
    this.color,
    this.borderColor,
    this.onTap,
    this.minHeight,
    this.gradient,
    this.boxShadow,
  });

  final Widget child;
  final EdgeInsetsGeometry? padding;
  final double radius;
  final Color? color;
  final Color? borderColor;
  final VoidCallback? onTap;
  final double? minHeight;
  final Gradient? gradient;
  final List<BoxShadow>? boxShadow;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final content = Container(
      constraints: minHeight != null
          ? BoxConstraints(minHeight: minHeight!)
          : null,
      padding: padding,
      decoration: BoxDecoration(
        color: gradient == null ? (color ?? palette.surface) : null,
        gradient: gradient,
        border: Border.all(
          color: borderColor ?? palette.line.withValues(alpha: 0.7),
        ),
        borderRadius: BorderRadius.circular(radius),
        boxShadow:
            boxShadow ??
            [
              BoxShadow(
                color: palette.shadow,
                blurRadius: 18,
                offset: const Offset(0, 8),
                spreadRadius: -12,
              ),
            ],
      ),
      child: child,
    );
    if (onTap == null) return content;
    return Material(
      color: Colors.transparent,
      borderRadius: BorderRadius.circular(radius),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(radius),
        child: content,
      ),
    );
  }
}

/// 彩色圆角图标底衬（quick tile / menu icon / app icon 等）。
class TileIcon extends StatelessWidget {
  const TileIcon({
    super.key,
    required this.icon,
    required this.color,
    this.size = 33,
    this.radius = 9,
    this.iconSize = 17,
    this.alpha = 0.13,
    this.filled = true,
  });

  final IconData icon;
  final Color color;
  final double size;
  final double radius;
  final double iconSize;
  final double alpha;
  final bool filled;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: filled ? null : Colors.transparent,
        gradient: filled
            ? LinearGradient(
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
                colors: [
                  color.withValues(alpha: (alpha * 1.35).clamp(0, 1)),
                  color.withValues(alpha: (alpha * 0.65).clamp(0, 1)),
                ],
              )
            : null,
        borderRadius: BorderRadius.circular(radius),
      ),
      child: Icon(icon, size: iconSize, color: color),
    );
  }
}
