/// 远端桌面控制：电源按钮、窗口操作按钮与当前窗口卡片。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_buttons.dart';
import 'app_card.dart';
import 'app_text_styles.dart';

/// 电源操作按钮（睡眠/重启/关机）。
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
    final tone = danger ? AppColors.red : AppColors.primary;
    return AppCard(
      radius: 9,
      onTap: onPressed,
      child: SizedBox(
        height: height,
        child: layout == PowerButtonLayout.vertical
            ? Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 17, color: tone),
                  const SizedBox(height: 6),
                  Text(
                    label,
                    style: AppText.meta.copyWith(
                      color: danger ? AppColors.red : palette.textMuted,
                    ),
                  ),
                ],
              )
            : Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 17, color: tone),
                  const SizedBox(width: 8),
                  Text(
                    label,
                    style: AppText.body.copyWith(
                      color: danger ? AppColors.red : palette.textMuted,
                    ),
                  ),
                ],
              ),
      ),
    );
  }
}

/// 窗口操作图标按钮（列表/最小化/最大化/关闭）。
class WindowActionButton extends StatelessWidget {
  const WindowActionButton({
    super.key,
    required this.icon,
    required this.tooltip,
    this.onPressed,
    this.danger = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  final bool danger;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return IconBtn(
      icon: icon,
      tooltip: tooltip,
      size: 36,
      iconSize: 16,
      radius: 8,
      color: danger ? AppColors.red : palette.textMuted,
      onPressed: onPressed,
    );
  }
}

/// 当前窗口卡片：图标 + 标题/元信息 + 窗口操作按钮组。
class ActiveWindowCard extends StatelessWidget {
  const ActiveWindowCard({
    super.key,
    required this.title,
    required this.meta,
    required this.enabled,
    required this.onWindows,
    required this.onMinimize,
    required this.onMaximize,
    required this.onClose,
    this.icon = Icons.sports_esports_rounded,
  });

  final String title;
  final String meta;
  final bool enabled;
  final VoidCallback onWindows;
  final VoidCallback onMinimize;
  final VoidCallback onMaximize;
  final VoidCallback onClose;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 64,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      child: Row(
        children: [
          TileIcon(
            icon: icon,
            color: AppColors.violet,
            size: 38,
            radius: 10,
            iconSize: 18,
            alpha: 0.16,
          ),
          const SizedBox(width: 9),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.body.copyWith(
                    fontWeight: FontWeight.w700,
                    color: palette.textPrimary,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  meta,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          WindowActionButton(
            icon: Icons.grid_view_rounded,
            tooltip: '窗口列表',
            onPressed: enabled ? onWindows : null,
          ),
          WindowActionButton(
            icon: Icons.remove_rounded,
            tooltip: '最小化',
            onPressed: enabled ? onMinimize : null,
          ),
          WindowActionButton(
            icon: Icons.crop_square_rounded,
            tooltip: '最大化',
            onPressed: enabled ? onMaximize : null,
          ),
          WindowActionButton(
            icon: Icons.close_rounded,
            tooltip: '关闭',
            danger: true,
            onPressed: enabled ? onClose : null,
          ),
        ],
      ),
    );
  }
}
