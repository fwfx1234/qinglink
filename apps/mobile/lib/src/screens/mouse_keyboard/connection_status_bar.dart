import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

/// 页面顶部的连接状态行：状态点 + 状态文案 + 鼠标设置入口。
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
    return Padding(
      padding: const EdgeInsets.only(left: 4, right: 2),
      child: Row(
        children: [
          StatusDot(color: statusColor, glow: connected),
          const SizedBox(width: 7),
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
            tooltip: '鼠标设置',
            size: 34,
            iconSize: 18,
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
        padding: const EdgeInsets.only(left: 4, top: 2),
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
