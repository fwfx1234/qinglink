import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';
import '../../widgets/app_text_styles.dart';

/// 打开远程软键盘面板的按钮。
///
/// 控制页底部主入口用品牌浅底 + 品牌描边（设计稿的「输入文字」），
/// [compact] 为真时退化成 48×48 的图标按钮（底部三键左侧的键盘键）。
class KeyboardEntryButton extends StatelessWidget {
  const KeyboardEntryButton({
    super.key,
    required this.enabled,
    required this.onPressed,
    this.compact = false,
  });

  final bool enabled;
  final VoidCallback onPressed;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    if (compact) {
      return SizedBox(
        height: 48,
        width: 48,
        child: Tooltip(
          message: '输入文字',
          child: IconButton(
            onPressed: enabled ? onPressed : null,
            icon: const Icon(Icons.keyboard_alt_outlined, size: 20),
            color: palette.textPrimary,
            style: IconButton.styleFrom(
              backgroundColor: palette.surface2,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(AppRadius.md),
                side: BorderSide(color: palette.lineStrong),
              ),
            ),
          ),
        ),
      );
    }
    return SizedBox(
      height: 48,
      width: double.infinity,
      child: Tooltip(
        message: '输入文字',
        child: OutlinedButton.icon(
          onPressed: enabled ? onPressed : null,
          icon: const Icon(Icons.keyboard_outlined, size: 18),
          label: Text(
            '输入文字',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: AppText.body.copyWith(color: palette.brand),
          ),
          style: OutlinedButton.styleFrom(
            foregroundColor: palette.brand,
            backgroundColor: palette.brandTint,
            side: BorderSide(color: palette.brand.withValues(alpha: 0.45)),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(AppRadius.md),
            ),
          ),
        ),
      ),
    );
  }
}
