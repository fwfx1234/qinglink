/// 顶部轻提示：从状态栏下方滑入、数秒后自动收起，长文本按宽度换行。
///
/// 不用 `SnackBar`：`SnackBarBehavior.floating` 只能从底部往上推，
/// 想让它出现在屏幕上方就得给一个接近屏高的 `margin`。提示高度一旦超过
/// 剩下的空间，顶部就会被状态栏裁掉——表现是"弹出提示了但看不到内容"。
/// 这里改用根 `Overlay`：提示钉在安全区顶部，高度随内容增长，永远不被裁剪。
library;

import 'dart:async';

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_text_styles.dart';

/// 当前展示中的提示条目；同一时刻只保留一条。
OverlayEntry? _currentEntry;
Timer? _dismissTimer;

/// 在屏幕顶部弹出一条轻提示；已有提示会被立刻替换。
///
/// [context] 只用来找根 Overlay，因此可以在弹层、对话框里调用。
void showTopToast(
  BuildContext context, {
  required String message,
  required Color color,
  required IconData icon,
  Duration duration = const Duration(seconds: 2),
}) {
  final overlay = Overlay.maybeOf(context, rootOverlay: true);
  if (overlay == null) return;
  dismissTopToast();
  final entry = OverlayEntry(
    builder: (_) => _TopToast(message: message, color: color, icon: icon),
  );
  _currentEntry = entry;
  overlay.insert(entry);
  _dismissTimer = Timer(duration, dismissTopToast);
}

/// 收起当前提示；没有提示时什么也不做。
void dismissTopToast() {
  _dismissTimer?.cancel();
  _dismissTimer = null;
  final entry = _currentEntry;
  _currentEntry = null;
  if (entry != null && entry.mounted) entry.remove();
}

/// 提示本体：安全区顶端 + 最大宽度约束，文本不限行数。
class _TopToast extends StatelessWidget {
  const _TopToast({
    required this.message,
    required this.color,
    required this.icon,
  });

  final String message;
  final Color color;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Positioned(
      top: 0,
      left: 0,
      right: 0,
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(
            AppSpacing.lg,
            AppSpacing.sm,
            AppSpacing.lg,
            0,
          ),
          child: IgnorePointer(
            child: Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 560),
                child: TweenAnimationBuilder<double>(
                  tween: Tween<double>(begin: 0, end: 1),
                  duration: const Duration(milliseconds: 180),
                  curve: Curves.easeOutCubic,
                  builder: (context, value, child) => Opacity(
                    opacity: value,
                    child: Transform.translate(
                      offset: Offset(0, (1 - value) * -8),
                      child: child,
                    ),
                  ),
                  child: Material(
                    color: color,
                    elevation: 0,
                    borderRadius: BorderRadius.circular(AppRadius.md),
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: AppSpacing.md,
                        vertical: 10,
                      ),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Padding(
                            padding: const EdgeInsets.only(top: 1),
                            child: Icon(icon, color: Colors.white, size: 18),
                          ),
                          const SizedBox(width: 8),
                          Flexible(
                            child: Text(
                              message,
                              // 不设 maxLines：提示的高度随内容增长，
                              // 这正是它比"底部 margin 硬顶到上方"可靠的地方。
                              softWrap: true,
                              style: AppText.body.copyWith(
                                color: Colors.white,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
