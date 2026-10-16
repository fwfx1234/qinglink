import 'package:flutter/material.dart';

import '../../theme/app_theme.dart';

/// 远程软键盘面板：快捷键条 + 文字输入框 + 退格/删除/回车。
///
/// 面板只负责展示与回调，IME 桥接与指令发送由调用方完成。
class KeyboardInputSheet extends StatelessWidget {
  const KeyboardInputSheet({
    super.key,
    required this.textController,
    required this.focusNode,
    required this.enabled,
    required this.shortcuts,
    required this.onShortcut,
    required this.onChanged,
    required this.onSubmitted,
    required this.onBackspace,
    required this.onDelete,
    required this.onSendEnter,
  });

  final TextEditingController textController;
  final FocusNode focusNode;
  final bool enabled;
  final List<String> shortcuts;

  /// 点击快捷键芯片。
  final ValueChanged<String> onShortcut;

  /// 输入框内容变化（已提交文本的差异交给 IME 桥处理）。
  final ValueChanged<String> onChanged;

  /// 输入法回车键提交。
  final VoidCallback onSubmitted;

  final VoidCallback onBackspace;
  final VoidCallback onDelete;
  final VoidCallback onSendEnter;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.fromLTRB(
        16,
        12,
        16,
        12 + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _ShortcutChipBar(
            shortcuts: shortcuts,
            enabled: enabled,
            onShortcut: onShortcut,
          ),
          const SizedBox(height: 14),
          _buildDraftField(),
          const SizedBox(height: 10),
          _SheetKeyRow(
            enabled: enabled,
            onBackspace: onBackspace,
            onDelete: onDelete,
            onSendEnter: onSendEnter,
          ),
        ],
      ),
    );
  }

  Widget _buildDraftField() {
    return TextField(
      controller: textController,
      focusNode: focusNode,
      autofocus: true,
      minLines: 1,
      maxLines: 3,
      maxLength: 4096,
      onChanged: onChanged,
      textInputAction: TextInputAction.send,
      // Keep the keyboard sheet active after sending so the user
      // can enter another message without reopening it.
      onEditingComplete: () {},
      onSubmitted: (_) => onSubmitted(),
      decoration: const InputDecoration(
        hintText: '输入要发送的文字',
        counterText: '',
        border: OutlineInputBorder(),
      ),
    );
  }
}

/// 监听弹层自身路由的退场动画。
///
/// 宿主用「输入法收起」判断用户想关掉键盘面板，但用户也可能直接下拉或点外部关闭；
/// 后一种情况下面板的退场动画已经开始，宿主若再 pop 一次就会把整页弹掉。这个监听器
/// 在面板开始退场时通知宿主取消那次延迟 pop。
class KeyboardSheetRouteWatcher extends StatefulWidget {
  const KeyboardSheetRouteWatcher({
    super.key,
    required this.onDismissStarted,
    required this.child,
  });

  /// 弹层开始退场（反向动画或已消失）时回调，可能被调用多次。
  final VoidCallback onDismissStarted;

  final Widget child;

  @override
  State<KeyboardSheetRouteWatcher> createState() =>
      _KeyboardSheetRouteWatcherState();
}

class _KeyboardSheetRouteWatcherState extends State<KeyboardSheetRouteWatcher> {
  ModalRoute<dynamic>? _route;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final route = ModalRoute.of(context);
    if (identical(route, _route)) return;
    _route?.animation?.removeStatusListener(_handleStatus);
    _route = route;
    _route?.animation?.addStatusListener(_handleStatus);
  }

  void _handleStatus(AnimationStatus status) {
    if (status == AnimationStatus.reverse ||
        status == AnimationStatus.dismissed) {
      widget.onDismissStarted();
    }
  }

  @override
  void dispose() {
    _route?.animation?.removeStatusListener(_handleStatus);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

/// 面板顶部的横向快捷键条。
class _ShortcutChipBar extends StatelessWidget {
  const _ShortcutChipBar({
    required this.shortcuts,
    required this.enabled,
    required this.onShortcut,
  });

  final List<String> shortcuts;
  final bool enabled;
  final ValueChanged<String> onShortcut;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 40,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        itemCount: shortcuts.length,
        separatorBuilder: (_, _) => const SizedBox(width: 6),
        itemBuilder: (_, index) {
          final label = shortcuts[index];
          return ActionChip(
            label: Text(label),
            onPressed: enabled ? () => onShortcut(label) : null,
          );
        },
      ),
    );
  }
}

/// 面板底部的退格 / 删除 / 回车按键行。
class _SheetKeyRow extends StatelessWidget {
  const _SheetKeyRow({
    required this.enabled,
    required this.onBackspace,
    required this.onDelete,
    required this.onSendEnter,
  });

  final bool enabled;
  final VoidCallback onBackspace;
  final VoidCallback onDelete;
  final VoidCallback onSendEnter;

  @override
  Widget build(BuildContext context) {
    // Explicit keys so text that was not entered through this IME
    // bridge (typed on the PC, pasted, previous session) can still be
    // deleted, and Enter can be sent without composing text.
    return Row(
      children: [
        Expanded(
          child: _SheetKeyButton(
            icon: Icons.backspace_outlined,
            label: '退格',
            enabled: enabled,
            onPressed: onBackspace,
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: _SheetKeyButton(
            icon: Icons.delete_outline_rounded,
            label: '删除',
            enabled: enabled,
            onPressed: onDelete,
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: _SheetKeyButton(
            icon: Icons.keyboard_return_rounded,
            label: '回车',
            enabled: enabled,
            onPressed: onSendEnter,
          ),
        ),
      ],
    );
  }
}

/// A wide key button inside the remote keyboard sheet.
class _SheetKeyButton extends StatelessWidget {
  const _SheetKeyButton({
    required this.icon,
    required this.label,
    required this.enabled,
    required this.onPressed,
  });

  final IconData icon;
  final String label;
  final bool enabled;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return OutlinedButton.icon(
      onPressed: enabled ? onPressed : null,
      icon: Icon(icon, size: 18),
      label: Text(label),
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(0, 46),
        foregroundColor: palette.textPrimary,
        side: BorderSide(color: palette.line),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
    );
  }
}
