/// 状态指示：状态圆点、状态标签与不确定进度条。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_text_styles.dart';

/// 状态圆点，对齐原型 `.status-dot`。
class StatusDot extends StatelessWidget {
  const StatusDot({super.key, this.color, this.glow = false, this.size = 7});

  final Color? color;
  final bool glow;
  final double size;

  @override
  Widget build(BuildContext context) {
    final dotColor = color ?? context.palette.textSubtle;
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: dotColor,
        shape: BoxShape.circle,
        boxShadow: glow
            ? [
                BoxShadow(
                  color: dotColor.withValues(alpha: 0.12),
                  spreadRadius: 4,
                ),
              ]
            : null,
      ),
    );
  }
}

/// 小号状态标签（如「使用中」「系统」「主显示器」）。
class StatusTag extends StatelessWidget {
  const StatusTag(this.label, {super.key, this.color});

  final String label;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final tone = color ?? AppColors.primary;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
      decoration: BoxDecoration(
        color: tone.withValues(alpha: 0.11),
        borderRadius: BorderRadius.circular(5),
      ),
      child: Text(
        label,
        style: AppText.tiny.copyWith(color: tone, fontWeight: FontWeight.w700),
      ),
    );
  }
}

/// 不确定进度条，对齐原型的状态卡片进度动画。
class AppProgressBar extends StatelessWidget {
  const AppProgressBar({super.key, this.height = 4});

  final double height;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return ClipRRect(
      borderRadius: BorderRadius.circular(height),
      child: LinearProgressIndicator(
        minHeight: height,
        color: AppColors.primary,
        backgroundColor: palette.textPrimary.withValues(alpha: 0.1),
      ),
    );
  }
}
