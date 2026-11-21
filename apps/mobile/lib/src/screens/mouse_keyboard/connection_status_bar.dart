import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

/// 页面顶部的连接状态行：状态点 + 状态文案 + 鼠标设置入口。
///
/// 设计稿里这一行直接落在页面背景上（不套卡片），高 32。
class ConnectionStatusBar extends StatelessWidget {
  const ConnectionStatusBar({
    super.key,
    required this.statusText,
    required this.statusColor,
    required this.connected,
    required this.onOpenSettings,
  });

  final String statusText;
  final Color statusColor;
  final bool connected;

  /// 打开鼠标设置面板。
  final VoidCallback onOpenSettings;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: 32,
      child: Row(
        children: [
          StatusDot(color: statusColor, glow: connected),
          const SizedBox(width: 8),
          Text(
            statusText,
            style: AppText.label.copyWith(
              color: statusColor,
              fontWeight: FontWeight.w600,
            ),
          ),
          const Spacer(),
          IconBtn(
            icon: Icons.tune_rounded,
            tooltip: '设置',
            size: 32,
            iconSize: 17,
            background: palette.iconBtn,
            onPressed: onOpenSettings,
          ),
        ],
      ),
    );
  }
}

/// 状态行下方的单行错误提示。
class ErrorTextLine extends StatelessWidget {
  const ErrorTextLine({super.key, required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.centerLeft,
      child: Padding(
        padding: const EdgeInsets.only(top: 4),
        child: Text(
          message,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: AppText.meta.copyWith(color: AppColors.error),
        ),
      ),
    );
  }
}
