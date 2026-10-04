import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';

/// 打开远程软键盘面板的主按钮。
class KeyboardEntryButton extends StatelessWidget {
  const KeyboardEntryButton({
    super.key,
    required this.enabled,
    required this.onPressed,
  });

  final bool enabled;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      width: double.infinity,
      height: 46,
      child: Tooltip(
        message: '输入文字',
        child: OutlinedButton.icon(
          onPressed: enabled ? onPressed : null,
          icon: const Icon(Icons.keyboard_outlined, size: 18),
          label: const Text('输入文字'),
          style: OutlinedButton.styleFrom(
            foregroundColor: palette.textPrimary,
            backgroundColor: palette.surface2,
            side: BorderSide(color: palette.line),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(AppRadius.md),
            ),
          ),
        ),
      ),
    );
  }
}
