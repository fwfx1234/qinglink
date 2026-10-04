/// 主壳的提示投递：底部 SnackBar 样式统一，以及「Windows 已解锁」提示的去重与
/// 前后台延迟结算。
///
/// 主壳只把解锁状态变化与前后台切换转交进来，见 `home_screen.dart`；
/// 触发提示的连接事件在 `home_connection.dart`。
library;

import 'package:flutter/material.dart';

import '../models/system_status.dart';
import '../theme/app_theme.dart';

/// 主壳提示投递器：统一底部浮动提示样式，并结算解锁成功提示。
class HomeNotificationPresenter {
  /// [contextOf] 取宿主 `BuildContext`，宿主未挂载时返回 null（等同挂载检查）。
  HomeNotificationPresenter({required this.contextOf});

  final BuildContext? Function() contextOf;

  bool _pendingUnlockNotification = false;
  DateTime? _lastUnlockNotificationAt;

  /// 解锁状态变化：只有真正的「已锁定 → 已解锁」迁移才提示。
  ///
  /// 连接后首次查询到的锁定状态保持静默；后台期间发生的解锁先挂起，
  /// 回到前台再由 [flushPendingUnlock] 补发。
  void handleUnlockStatus(
    UnlockStatus? previous,
    UnlockStatus? next, {
    required bool foregrounded,
  }) {
    // Only report an actual locked -> unlocked transition. The initial
    // status query after connecting must stay silent.
    if (previous?.locked != true || next?.locked != false) return;
    if (!foregrounded) {
      _pendingUnlockNotification = true;
      return;
    }
    _notifyUnlockSuccess();
  }

  /// 回到前台时补发后台期间挂起的解锁提示。
  void flushPendingUnlock() {
    if (!_pendingUnlockNotification) return;
    _pendingUnlockNotification = false;
    _notifyUnlockSuccess();
  }

  /// 底部浮动提示：文案含「上线」或「解锁」时用成功图标，否则用错误图标。
  void showNotification(String message, Color color) {
    final context = contextOf();
    if (context == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Icon(
              message.contains('上线') || message.contains('解锁')
                  ? Icons.check_circle_rounded
                  : Icons.error_rounded,
              color: Colors.white,
              size: 18,
            ),
            const SizedBox(width: 8),
            Text(message),
          ],
        ),
        behavior: SnackBarBehavior.floating,
        backgroundColor: color,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        margin: const EdgeInsets.all(16),
        duration: const Duration(seconds: 2),
      ),
    );
  }

  /// 「Windows 已解锁」提示：2 秒内去重，避免连续状态变化重复弹提示。
  void _notifyUnlockSuccess() {
    final now = DateTime.now();
    if (_lastUnlockNotificationAt != null &&
        now.difference(_lastUnlockNotificationAt!) <
            const Duration(seconds: 2)) {
      return;
    }
    _lastUnlockNotificationAt = now;
    showNotification('Windows已解锁', AppColors.success);
  }
}
