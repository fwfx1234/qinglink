/// 窗口控制组件：窗口操作按钮组、当前窗口卡与首页底部 Dock 条。
///
/// 窗口控制（列表/最小化/最大化/关闭）是高频操作，设计稿把它放在拇指区：
/// 首页是常驻底部 Dock 条，控制页在触控区下方。两种形态共用本库的按钮组。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_text_styles.dart';

/// 窗口操作图标按钮：40×36，`bg.raised` 底座，关闭键用危险浅底。
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

  static const double width = 36;
  static const double height = 32;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final tone = danger ? palette.danger : palette.textMuted;
    return SizedBox(
      width: width,
      height: height,
      child: IconButton(
        tooltip: tooltip,
        onPressed: onPressed,
        icon: Icon(icon, size: 18),
        color: tone,
        disabledColor: palette.textSubtle.withValues(alpha: 0.4),
        padding: EdgeInsets.zero,
        style: IconButton.styleFrom(
          backgroundColor: danger ? palette.dangerTint : palette.surface2,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.sm),
          ),
        ),
      ),
    );
  }
}

/// 窗口操作按钮组：列表 / 最小化 / 最大化 / 关闭。
class WindowActionCluster extends StatelessWidget {
  const WindowActionCluster({
    super.key,
    required this.enabled,
    required this.onWindows,
    required this.onMinimize,
    required this.onMaximize,
    required this.onClose,
  });

  final bool enabled;
  final VoidCallback onWindows;
  final VoidCallback onMinimize;
  final VoidCallback onMaximize;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        WindowActionButton(
          icon: Icons.grid_view_rounded,
          tooltip: '窗口列表',
          onPressed: enabled ? onWindows : null,
        ),
        const SizedBox(width: 6),
        WindowActionButton(
          icon: Icons.remove_rounded,
          tooltip: '最小化',
          onPressed: enabled ? onMinimize : null,
        ),
        const SizedBox(width: 6),
        WindowActionButton(
          icon: Icons.crop_square_rounded,
          tooltip: '最大化',
          onPressed: enabled ? onMaximize : null,
        ),
        const SizedBox(width: 6),
        WindowActionButton(
          icon: Icons.close_rounded,
          tooltip: '关闭',
          danger: true,
          onPressed: enabled ? onClose : null,
        ),
      ],
    );
  }
}

/// 当前窗口卡：真实程序图标 + 标题/元信息 + 窗口操作按钮组。
class ActiveWindowCard extends StatelessWidget {
  const ActiveWindowCard({
    super.key,
    required this.title,
    required this.meta,
    required this.icon,
    required this.enabled,
    required this.onWindows,
    required this.onMinimize,
    required this.onMaximize,
    required this.onClose,
  });

  final String title;
  final String meta;

  /// 真实程序图标（进程图标或窗口图标），由调用方提供。
  final Widget icon;
  final bool enabled;
  final VoidCallback onWindows;
  final VoidCallback onMinimize;
  final VoidCallback onMaximize;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      constraints: const BoxConstraints(minHeight: 58),
      padding: const EdgeInsets.fromLTRB(10, 8, 8, 8),
      decoration: BoxDecoration(
        color: palette.surface,
        border: Border.all(color: palette.line),
        borderRadius: BorderRadius.circular(AppRadius.lg),
      ),
      child: Row(
        children: [
          icon,
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.rowTitle.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 1),
                Text(
                  meta,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          WindowActionCluster(
            enabled: enabled,
            onWindows: onWindows,
            onMinimize: onMinimize,
            onMaximize: onMaximize,
            onClose: onClose,
          ),
        ],
      ),
    );
  }
}

/// 首页常驻底部 Dock 条：当前窗口标题 + 窗口控制按钮。
class WindowDock extends StatelessWidget {
  const WindowDock({
    super.key,
    required this.title,
    required this.active,
    required this.enabled,
    required this.onWindows,
    required this.onMinimize,
    required this.onMaximize,
    required this.onClose,
  });

  final String title;
  final bool active;
  final bool enabled;
  final VoidCallback onWindows;
  final VoidCallback onMinimize;
  final VoidCallback onMaximize;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      height: AppMetrics.dockHeight,
      padding: const EdgeInsets.fromLTRB(14, 8, 8, 8),
      decoration: BoxDecoration(
        color: palette.surface,
        border: Border.all(color: palette.line),
        borderRadius: BorderRadius.circular(AppRadius.lg),
        boxShadow: [
          BoxShadow(
            color: palette.shadow,
            blurRadius: 24,
            offset: const Offset(0, 10),
            spreadRadius: -12,
          ),
        ],
      ),
      child: Row(
        children: [
          Container(
            width: 7,
            height: 7,
            decoration: BoxDecoration(
              color: active ? palette.brand : palette.textSubtle,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 9),
          Expanded(
            child: Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: AppText.rowTitle.copyWith(color: palette.textPrimary),
            ),
          ),
          const SizedBox(width: 8),
          WindowActionCluster(
            enabled: enabled,
            onWindows: onWindows,
            onMinimize: onMinimize,
            onMaximize: onMaximize,
            onClose: onClose,
          ),
        ],
      ),
    );
  }
}
