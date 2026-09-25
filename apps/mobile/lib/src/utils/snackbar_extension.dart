import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// 全局统一的 Snackbar 反馈。
///
/// 所有页面反馈统一走这里：圆角 12、边距 16、图标 + 语义色、时长 2s。
extension SnackBarExtension on BuildContext {
  void showSuccessSnackBar(String message) {
    _show(message, AppColors.success, Icons.check_circle_rounded);
  }

  void showErrorSnackBar(String message) {
    _show(message, AppColors.error, Icons.error_rounded);
  }

  void showWarningSnackBar(String message) {
    _show(message, AppColors.warning, Icons.warning_amber_rounded);
  }

  void showInfoSnackBar(String message) {
    _show(message, AppColors.primary, Icons.info_rounded);
  }

  void _show(String message, Color color, IconData icon) {
    final media = MediaQuery.of(this);
    // SnackBars are anchored to the bottom; a large bottom margin lifts this
    // one near the top so it is not covered by the bottom navigation bar or
    // on-screen controls.
    final bottomMargin = (media.size.height - media.padding.top - 96)
        .clamp(AppSpacing.lg, media.size.height)
        .toDouble();
    ScaffoldMessenger.of(this).showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Icon(icon, color: Colors.white, size: 18),
            const SizedBox(width: 8),
            Expanded(child: Text(message)),
          ],
        ),
        behavior: SnackBarBehavior.floating,
        backgroundColor: color,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
        ),
        margin: EdgeInsets.fromLTRB(
          AppSpacing.lg,
          AppSpacing.lg,
          AppSpacing.lg,
          bottomMargin,
        ),
        duration: const Duration(seconds: 2),
      ),
    );
  }
}
