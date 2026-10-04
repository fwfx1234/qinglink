/// 遥控按键的统一样式：默认 tonal，重点键用 `accent` 描色。
library;

import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';

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
