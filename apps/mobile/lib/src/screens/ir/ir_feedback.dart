/// IR 页面的反馈：统一 SnackBar 提示与红外能力提示条。
library;

import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

/// 统一在 IR 各页面弹出一条提示。
///
/// 会先清掉上一条，避免连续发送时提示排队堆积、看起来一直不消失。
/// [error] 为 true 时停留更久，方便用户看清失败原因。
void showIrMessage(BuildContext context, String text, {bool error = false}) {
  if (!context.mounted) return;
  final messenger = ScaffoldMessenger.of(context);
  messenger.clearSnackBars();
  messenger.showSnackBar(
    SnackBar(
      content: Text(text),
      duration: Duration(milliseconds: error ? 2800 : 1000),
      behavior: SnackBarBehavior.floating,
      margin: const EdgeInsets.fromLTRB(16, 0, 16, 20),
    ),
  );
}

/// 手机不支持红外时的提示条；支持时不占空间。
class IrCapabilityBanner extends StatelessWidget {
  const IrCapabilityBanner({super.key, required this.supported});

  final bool supported;

  @override
  Widget build(BuildContext context) {
    if (supported) return const SizedBox.shrink();
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(13),
      color: AppColors.warning.withValues(alpha: 0.08),
      borderColor: AppColors.warning.withValues(alpha: 0.35),
      boxShadow: const [],
      child: Row(
        children: [
          const Icon(Icons.warning_amber_rounded, size: 19, color: AppColors.warning),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  '当前手机不支持红外发射',
                  style: AppText.cardTitle.copyWith(
                    color: context.palette.textPrimary,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  '仍可编辑遥控器数据；请使用带红外发射器的 Android 手机发送。',
                  style: AppText.meta.copyWith(color: context.palette.textMuted),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
