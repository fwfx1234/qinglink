/// 基础卡片容器与图标底衬。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// 卡片容器：`surface` 背景 + `line` 描边 + 圆角 14。
///
/// 设计稿里的卡片是平面块，默认不带投影；需要悬浮感的少数位置
/// （状态卡、窗口 Dock）显式传 [elevated]。
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
    this.elevated = false,
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
  final bool elevated;

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
        border: Border.all(color: borderColor ?? palette.line),
        borderRadius: BorderRadius.circular(radius),
        boxShadow: boxShadow ?? (elevated ? _shadow(palette) : null),
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

  List<BoxShadow> _shadow(AppPalette palette) => [
    BoxShadow(
      color: palette.shadow,
      blurRadius: 24,
      offset: const Offset(0, 12),
      spreadRadius: -14,
    ),
  ];
}

/// 彩色圆角图标底衬（quick tile / menu icon / app icon 等）。
///
/// 设计稿用的是平涂浅底（不是渐变），圆角 12。
class TileIcon extends StatelessWidget {
  const TileIcon({
    super.key,
    required this.icon,
    required this.color,
    this.size = 40,
    this.radius = 12,
    this.iconSize = 20,
    this.alpha = 0.14,
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
        color: filled ? color.withValues(alpha: alpha) : Colors.transparent,
        borderRadius: BorderRadius.circular(radius),
      ),
      child: Icon(icon, size: iconSize, color: color),
    );
  }
}
