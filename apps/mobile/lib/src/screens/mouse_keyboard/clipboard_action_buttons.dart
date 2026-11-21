import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';

/// 远程剪贴板动作。
///
/// 只向 PC 发送组合键，**不读写任何一端的剪贴板**：
/// 「复制」= PC 上按 Ctrl+C（把 PC 当前选中内容放进 PC 剪贴板），
/// 「粘贴」= PC 上按 Ctrl+V（把 PC 剪贴板内容贴到 PC 光标处）。
/// 手机与 PC 之间不做剪贴板同步，避免跨设备权限与隐私面。
enum ClipboardAction {
  copy('复制', 'Ctrl + C', Icons.content_copy_rounded, '已复制'),
  paste('粘贴', 'Ctrl + V', Icons.content_paste_rounded, '已粘贴'),
  selectAll('全选', 'Ctrl + A', Icons.select_all_rounded, '已全选');

  const ClipboardAction(this.label, this.shortcut, this.icon, this.doneText);

  /// 按钮文字。
  final String label;

  /// 交给 `InputController.sendShortcut` 的快捷键标签。
  final String shortcut;

  /// 按钮图标。
  final IconData icon;

  /// 成功后的提示文案。
  final String doneText;
}

/// 单个剪贴板动作按钮（图标 + 文字，高度与主按钮一致）。
class ClipboardActionButton extends StatelessWidget {
  const ClipboardActionButton({
    super.key,
    required this.action,
    required this.enabled,
    required this.onPressed,
  });

  final ClipboardAction action;
  final bool enabled;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: 48,
      child: Tooltip(
        message: '${action.label}（${action.shortcut}，作用于 PC）',
        child: OutlinedButton.icon(
          onPressed: enabled ? onPressed : null,
          icon: Icon(action.icon, size: 18),
          label: Text(
            action.label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          style: OutlinedButton.styleFrom(
            padding: const EdgeInsets.symmetric(horizontal: 10),
            foregroundColor: palette.textPrimary,
            backgroundColor: palette.surface2,
            side: BorderSide(color: palette.lineStrong),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(AppRadius.md),
            ),
            textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
          ),
        ),
      ),
    );
  }
}

/// 一行等宽排布的剪贴板动作按钮，主控制页与键盘面板共用。
class ClipboardActionRow extends StatelessWidget {
  const ClipboardActionRow({
    super.key,
    required this.actions,
    required this.enabled,
    required this.onAction,
  });

  final List<ClipboardAction> actions;
  final bool enabled;
  final ValueChanged<ClipboardAction> onAction;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        for (var index = 0; index < actions.length; index++) ...[
          if (index > 0) const SizedBox(width: 8),
          Expanded(
            child: ClipboardActionButton(
              action: actions[index],
              enabled: enabled,
              onPressed: () => onAction(actions[index]),
            ),
          ),
        ],
      ],
    );
  }
}
