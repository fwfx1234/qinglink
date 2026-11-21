/// 远端桌面控制：电源按钮。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_card.dart';
import 'app_text_styles.dart';

/// 电源操作按钮（睡眠/重启/关机）的排布方向。
enum PowerButtonLayout { vertical, horizontal }

class PowerActionButton extends StatelessWidget {
  const PowerActionButton({
    super.key,
    required this.icon,
    required this.label,
    this.onPressed,
    this.danger = false,
    this.layout = PowerButtonLayout.vertical,
    this.height = 62,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onPressed;
  final bool danger;
  final PowerButtonLayout layout;
  final double height;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final tone = danger ? palette.danger : palette.textMuted;
    final labelColor = danger ? palette.danger : palette.textMuted;
    return AppCard(
      radius: AppRadius.md,
      color: danger ? palette.dangerTint : palette.surface2,
      borderColor: danger
          ? palette.danger.withValues(alpha: 0.28)
          : palette.line,
      onTap: onPressed,
      child: SizedBox(
        height: height,
        child: layout == PowerButtonLayout.vertical
            ? Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 18, color: tone),
                  const SizedBox(height: 6),
                  Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AppText.meta.copyWith(color: labelColor),
                  ),
                ],
              )
            : Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 18, color: tone),
                  const SizedBox(width: 8),
                  Flexible(
                    child: Text(
                      label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.body.copyWith(color: labelColor),
                    ),
                  ),
                ],
              ),
      ),
    );
  }
}
