import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../widgets/top_toast.dart';

/// 全局统一的轻提示反馈。
///
/// 所有页面反馈统一走这里：顶部滑入、圆角 12、图标 + 语义色、时长 2s。
/// 位置由 [showTopToast] 决定：提示钉在安全区顶部、高度随内容增长，
/// 因此不会像"用超大底部 margin 顶到上方"那样把内容顶出屏幕外。
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
    showTopToast(this, message: message, color: color, icon: icon);
  }
}
