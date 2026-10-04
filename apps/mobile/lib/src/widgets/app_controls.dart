/// 交互控件：筛选标签、开关、搜索框与按压反馈。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_buttons.dart';

/// 胶囊筛选标签，对齐原型 `.filter-chip`。
class FilterChipX extends StatelessWidget {
  const FilterChipX({
    super.key,
    required this.label,
    required this.active,
    this.onTap,
  });

  final String label;
  final bool active;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        height: 36,
        padding: const EdgeInsets.symmetric(horizontal: 14),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: active ? AppColors.primary : palette.surface,
          border: Border.all(color: active ? AppColors.primary : palette.line),
          borderRadius: BorderRadius.circular(AppRadius.pill),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 11,
            fontWeight: active ? FontWeight.w700 : FontWeight.w400,
            color: active ? AppColors.onPrimary : palette.textMuted,
          ),
        ),
      ),
    );
  }
}

/// 开关：对齐原型 `.toggle`（42×24）。
class AppToggle extends StatelessWidget {
  const AppToggle({super.key, required this.value, this.onChanged});

  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return GestureDetector(
      onTap: onChanged == null ? null : () => onChanged!(!value),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 180),
        width: 42,
        height: 24,
        padding: const EdgeInsets.all(3),
        decoration: BoxDecoration(
          color: value ? AppColors.primaryDark : palette.surface3,
          borderRadius: BorderRadius.circular(20),
        ),
        child: AnimatedAlign(
          duration: const Duration(milliseconds: 180),
          alignment: value ? Alignment.centerRight : Alignment.centerLeft,
          child: Container(
            width: 18,
            height: 18,
            decoration: BoxDecoration(
              color: value ? const Color(0xFFEFFFF6) : const Color(0xFF7C8A82),
              shape: BoxShape.circle,
            ),
          ),
        ),
      ),
    );
  }
}

/// 搜索框，对齐原型 `.search-field`（高度 44）。
class SearchField extends StatefulWidget {
  const SearchField({
    super.key,
    required this.controller,
    this.hint = '搜索',
    this.onChanged,
    this.onClear,
    this.focusNode,
  });

  final TextEditingController controller;
  final String hint;
  final ValueChanged<String>? onChanged;
  final VoidCallback? onClear;
  final FocusNode? focusNode;

  @override
  State<SearchField> createState() => _SearchFieldState();
}

class _SearchFieldState extends State<SearchField> {
  late final FocusNode _node;
  FocusNode? _owned;
  bool _focused = false;
  bool _hasText = false;

  @override
  void initState() {
    super.initState();
    _owned = widget.focusNode == null ? FocusNode() : null;
    _node = widget.focusNode ?? _owned!;
    _node.addListener(_onFocusChanged);
    widget.controller.addListener(_onTextChanged);
    _hasText = widget.controller.text.isNotEmpty;
  }

  @override
  void didUpdateWidget(covariant SearchField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_onTextChanged);
      widget.controller.addListener(_onTextChanged);
      _hasText = widget.controller.text.isNotEmpty;
    }
  }

  @override
  void dispose() {
    _node.removeListener(_onFocusChanged);
    widget.controller.removeListener(_onTextChanged);
    _owned?.dispose();
    super.dispose();
  }

  void _onFocusChanged() => setState(() => _focused = _node.hasFocus);

  void _onTextChanged() =>
      setState(() => _hasText = widget.controller.text.isNotEmpty);

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Material(
      color: Colors.transparent,
      borderRadius: BorderRadius.circular(AppRadius.md),
      child: Container(
        height: 44,
        decoration: BoxDecoration(
          color: palette.surface,
          border: Border.all(
            color: _focused ? AppColors.primaryDark : palette.line,
          ),
          borderRadius: BorderRadius.circular(AppRadius.md),
        ),
        child: Row(
          children: [
            const SizedBox(width: 11),
            Icon(
              Icons.search_rounded,
              size: 17,
              color: _focused ? AppColors.primary : palette.textSubtle,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: TextField(
                controller: widget.controller,
                focusNode: _node,
                onChanged: widget.onChanged,
                style: TextStyle(fontSize: 12, color: palette.textPrimary),
                cursorColor: AppColors.primary,
                decoration: InputDecoration(
                  hintText: widget.hint,
                  hintStyle: TextStyle(fontSize: 12, color: palette.textSubtle),
                  isCollapsed: true,
                  filled: false,
                  border: InputBorder.none,
                  enabledBorder: InputBorder.none,
                  focusedBorder: InputBorder.none,
                  disabledBorder: InputBorder.none,
                  errorBorder: InputBorder.none,
                  focusedErrorBorder: InputBorder.none,
                  contentPadding: EdgeInsets.zero,
                ),
              ),
            ),
            if (_hasText)
              IconBtn(
                icon: Icons.close_rounded,
                size: 32,
                iconSize: 15,
                color: palette.textMuted,
                onPressed: widget.onClear,
              )
            else
              const SizedBox(width: 11),
          ],
        ),
      ),
    );
  }
}

/// 按下时的轻微缩放反馈；使用 [Listener] 不参与手势竞技，不影响内部点击。
class PressableScale extends StatefulWidget {
  const PressableScale({
    super.key,
    required this.child,
    this.pressedScale = 0.97,
    this.enabled = true,
  });

  final Widget child;
  final double pressedScale;
  final bool enabled;

  @override
  State<PressableScale> createState() => _PressableScaleState();
}

class _PressableScaleState extends State<PressableScale> {
  bool _pressed = false;

  void _set(bool value) {
    if (!widget.enabled || _pressed == value) return;
    setState(() => _pressed = value);
  }

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerDown: widget.enabled ? (_) => _set(true) : null,
      onPointerUp: widget.enabled ? (_) => _set(false) : null,
      onPointerCancel: widget.enabled ? (_) => _set(false) : null,
      child: AnimatedScale(
        scale: _pressed ? widget.pressedScale : 1,
        duration: const Duration(milliseconds: 110),
        curve: Curves.easeOut,
        child: widget.child,
      ),
    );
  }
}
