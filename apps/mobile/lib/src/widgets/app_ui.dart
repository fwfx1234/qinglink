import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// 统一文本样式 —— 对齐原型 `styles.css` 的字号/字重。
///
/// 颜色由调用方通过 `.copyWith(color: ...)` 注入当前调色板。
abstract final class AppText {
  static const TextStyle eyebrow = TextStyle(
    fontSize: 10,
    letterSpacing: 1.1,
    height: 1.2,
  );
  static const TextStyle pageTitle = TextStyle(
    fontSize: 22,
    height: 1.15,
    letterSpacing: -0.4,
    fontWeight: FontWeight.w700,
  );
  static const TextStyle sectionTitle = TextStyle(
    fontSize: 14,
    fontWeight: FontWeight.w600,
  );
  static const TextStyle cardTitle = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w700,
  );
  static const TextStyle rowTitle = TextStyle(
    fontSize: 12,
    fontWeight: FontWeight.w700,
  );
  static const TextStyle body = TextStyle(fontSize: 12);
  static const TextStyle label = TextStyle(fontSize: 11);
  static const TextStyle meta = TextStyle(fontSize: 10);
  static const TextStyle tiny = TextStyle(fontSize: 9);
}

/// 卡片容器：`surface` 背景 + `line` 边框 + 圆角。
class AppCard extends StatelessWidget {
  const AppCard({
    super.key,
    required this.child,
    this.padding,
    this.radius = AppRadius.md,
    this.color,
    this.borderColor,
    this.onTap,
    this.minHeight,
    this.gradient,
    this.boxShadow,
  });

  final Widget child;
  final EdgeInsetsGeometry? padding;
  final double radius;
  final Color? color;
  final Color? borderColor;
  final VoidCallback? onTap;
  final double? minHeight;
  final Gradient? gradient;
  final List<BoxShadow>? boxShadow;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final content = Container(
      constraints: minHeight != null
          ? BoxConstraints(minHeight: minHeight!)
          : null,
      padding: padding,
      decoration: BoxDecoration(
        color: gradient == null ? (color ?? palette.surface) : null,
        gradient: gradient,
        border: Border.all(
          color: borderColor ?? palette.line.withValues(alpha: 0.7),
        ),
        borderRadius: BorderRadius.circular(radius),
        boxShadow:
            boxShadow ??
            [
              BoxShadow(
                color: palette.shadow,
                blurRadius: 18,
                offset: const Offset(0, 8),
                spreadRadius: -12,
              ),
            ],
      ),
      child: child,
    );
    if (onTap == null) return content;
    return Material(
      color: Colors.transparent,
      borderRadius: BorderRadius.circular(radius),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(radius),
        child: content,
      ),
    );
  }
}

/// 彩色圆角图标底衬（quick tile / menu icon / app icon 等）。
class TileIcon extends StatelessWidget {
  const TileIcon({
    super.key,
    required this.icon,
    required this.color,
    this.size = 33,
    this.radius = 9,
    this.iconSize = 17,
    this.alpha = 0.13,
    this.filled = true,
  });

  final IconData icon;
  final Color color;
  final double size;
  final double radius;
  final double iconSize;
  final double alpha;
  final bool filled;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: filled ? null : Colors.transparent,
        gradient: filled
            ? LinearGradient(
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
                colors: [
                  color.withValues(alpha: (alpha * 1.35).clamp(0, 1)),
                  color.withValues(alpha: (alpha * 0.65).clamp(0, 1)),
                ],
              )
            : null,
        borderRadius: BorderRadius.circular(radius),
      ),
      child: Icon(icon, size: iconSize, color: color),
    );
  }
}

/// 顶栏/卡片通用图标按钮，默认 44×44。
class IconBtn extends StatelessWidget {
  const IconBtn({
    super.key,
    required this.icon,
    this.onPressed,
    this.tooltip,
    this.color,
    this.size = 44,
    this.iconSize = 19,
    this.radius = 10,
    this.background,
  });

  final IconData icon;
  final VoidCallback? onPressed;
  final String? tooltip;
  final Color? color;
  final double size;
  final double iconSize;
  final double radius;
  final Color? background;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final button = SizedBox(
      width: size,
      height: size,
      child: IconButton(
        tooltip: tooltip,
        onPressed: onPressed,
        icon: Icon(icon, size: iconSize),
        color: color ?? palette.textMuted,
        disabledColor: palette.textSubtle.withValues(alpha: 0.4),
        padding: EdgeInsets.zero,
        style: IconButton.styleFrom(
          backgroundColor: background ?? Colors.transparent,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(radius),
          ),
        ),
      ),
    );
    return button;
  }
}

/// 状态圆点，对齐原型 `.status-dot`。
class StatusDot extends StatelessWidget {
  const StatusDot({super.key, this.color, this.glow = false, this.size = 7});

  final Color? color;
  final bool glow;
  final double size;

  @override
  Widget build(BuildContext context) {
    final dotColor = color ?? context.palette.textSubtle;
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: dotColor,
        shape: BoxShape.circle,
        boxShadow: glow
            ? [
                BoxShadow(
                  color: dotColor.withValues(alpha: 0.12),
                  spreadRadius: 4,
                ),
              ]
            : null,
      ),
    );
  }
}

/// 主按钮：primary 背景，默认高度 52。
class PrimaryButton extends StatelessWidget {
  const PrimaryButton({
    super.key,
    required this.label,
    this.onPressed,
    this.icon,
    this.height = 52,
    this.expand = false,
    this.small = false,
    this.danger = false,
    this.busy = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final double height;
  final bool expand;
  final bool small;
  final bool danger;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final enabled = onPressed != null && !busy;
    final child = busy
        ? const SizedBox(
            width: 18,
            height: 18,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        : Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (icon != null) ...[
                Icon(icon, size: 17),
                const SizedBox(width: 8),
              ],
              Text(label),
            ],
          );
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(AppRadius.md),
        boxShadow: enabled
            ? [
                BoxShadow(
                  color: (danger ? AppColors.red : AppColors.primary)
                      .withValues(alpha: 0.32),
                  blurRadius: 18,
                  offset: const Offset(0, 6),
                  spreadRadius: -6,
                ),
              ]
            : null,
      ),
      child: SizedBox(
        height: height,
        width: expand ? double.infinity : null,
        child: FilledButton(
          onPressed: enabled ? onPressed : null,
          style: FilledButton.styleFrom(
            backgroundColor: danger ? AppColors.red : AppColors.primary,
            foregroundColor: danger
                ? const Color(0xFF321010)
                : AppColors.onPrimary,
            disabledBackgroundColor:
                (danger ? AppColors.red : AppColors.primary).withValues(
                  alpha: 0.4,
                ),
            disabledForegroundColor:
                (danger ? const Color(0xFF321010) : AppColors.onPrimary)
                    .withValues(alpha: 0.7),
            padding: EdgeInsets.symmetric(horizontal: small ? 14 : 18),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(AppRadius.md),
            ),
            textStyle: const TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w700,
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}

/// 次按钮：surface 背景 + line 边框。
class SecondaryButton extends StatelessWidget {
  const SecondaryButton({
    super.key,
    required this.label,
    this.onPressed,
    this.icon,
    this.height = 46,
    this.expand = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final double height;
  final bool expand;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: height,
      width: expand ? double.infinity : null,
      child: OutlinedButton(
        onPressed: onPressed,
        style: OutlinedButton.styleFrom(
          backgroundColor: palette.surface,
          foregroundColor: palette.textPrimary,
          side: BorderSide(color: palette.line),
          padding: const EdgeInsets.symmetric(horizontal: 16),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(9)),
          textStyle: const TextStyle(fontSize: 12),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (icon != null) ...[
              Icon(icon, size: 17),
              const SizedBox(width: 8),
            ],
            Text(label),
          ],
        ),
      ),
    );
  }
}

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

/// 区块标题：`标题 + 可选右侧说明`。
class SectionHeading extends StatelessWidget {
  const SectionHeading({
    super.key,
    required this.title,
    this.trailing,
    this.bottom = 11,
  });

  final String title;
  final Widget? trailing;
  final double bottom;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: EdgeInsets.only(bottom: bottom),
      child: Row(
        children: [
          Text(
            title,
            style: AppText.sectionTitle.copyWith(color: palette.textPrimary),
          ),
          const Spacer(),
          ?trailing,
        ],
      ),
    );
  }
}

/// 空状态。
class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    this.message,
    this.action,
  });

  final IconData icon;
  final String title;
  final String? message;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      constraints: const BoxConstraints(minHeight: 200),
      alignment: Alignment.center,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            width: 46,
            height: 46,
            decoration: BoxDecoration(
              color: palette.surface,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Icon(icon, size: 22, color: palette.textSubtle),
          ),
          const SizedBox(height: 14),
          Text(
            title,
            style: AppText.cardTitle.copyWith(color: palette.textPrimary),
          ),
          if (message != null) ...[
            const SizedBox(height: 7),
            Text(
              message!,
              textAlign: TextAlign.center,
              style: AppText.body.copyWith(color: palette.textSubtle),
            ),
          ],
          if (action != null) ...[const SizedBox(height: 14), action!],
        ],
      ),
    );
  }
}

/// 页面标题区：eyebrow + 大标题 + 右侧操作。
///
/// [eyebrow] 可选，用于子页面；Tab 根页面通常不需要标题。
class PageHeading extends StatelessWidget {
  const PageHeading({
    super.key,
    this.eyebrow,
    required this.title,
    this.trailing,
    this.compact = false,
  });

  final String? eyebrow;
  final String title;
  final Widget? trailing;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: EdgeInsets.only(bottom: compact ? 14 : 18),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                if (eyebrow != null) ...[
                  Text(
                    eyebrow!,
                    style: AppText.eyebrow.copyWith(color: palette.textSubtle),
                  ),
                  const SizedBox(height: 3),
                ],
                Text(
                  title,
                  style: AppText.pageTitle.copyWith(color: palette.textPrimary),
                ),
              ],
            ),
          ),
          ?trailing,
        ],
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

/// 小号状态标签（如「使用中」「系统」「主显示器」）。
class StatusTag extends StatelessWidget {
  const StatusTag(this.label, {super.key, this.color});

  final String label;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final tone = color ?? AppColors.primary;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
      decoration: BoxDecoration(
        color: tone.withValues(alpha: 0.11),
        borderRadius: BorderRadius.circular(5),
      ),
      child: Text(
        label,
        style: AppText.tiny.copyWith(color: tone, fontWeight: FontWeight.w700),
      ),
    );
  }
}

/// 设置分组卡片：标题 + 右侧值 + 内容。
class SettingsGroup extends StatelessWidget {
  const SettingsGroup({
    super.key,
    required this.title,
    required this.children,
    this.trailing,
    this.trailingColor,
    this.padding = const EdgeInsets.all(13),
  });

  final String title;
  final List<Widget> children;
  final Widget? trailing;
  final Color? trailingColor;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      radius: AppRadius.lg,
      padding: padding,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                title,
                style: AppText.cardTitle.copyWith(color: palette.textPrimary),
              ),
              const Spacer(),
              ?trailing,
            ],
          ),
          const SizedBox(height: 14),
          ...children,
        ],
      ),
    );
  }
}

/// 通用列表行：可选彩色图标底衬 + 标题/副标题 + 尾部。
///
/// 同时覆盖「设置行」「菜单行」「信息行」等形态，避免各页面重复实现。
class AppListRow extends StatelessWidget {
  const AppListRow({
    super.key,
    this.title,
    this.titleWidget,
    this.subtitle,
    this.subtitleWidget,
    this.icon,
    this.iconTone,
    this.tile = false,
    this.trailing,
    this.onTap,
    this.enabled = true,
    this.showChevron = false,
    this.minHeight = 56,
    this.padding = const EdgeInsets.symmetric(vertical: 6),
    this.border,
  });

  final String? title;
  final Widget? titleWidget;
  final String? subtitle;
  final Widget? subtitleWidget;
  final IconData? icon;
  final Color? iconTone;
  final bool tile;
  final Widget? trailing;
  final VoidCallback? onTap;
  final bool enabled;
  final bool showChevron;
  final double minHeight;
  final EdgeInsetsGeometry padding;
  final Border? border;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final leading = tile && icon != null
        ? TileIcon(icon: icon!, color: iconTone ?? AppColors.primary)
        : (icon != null
              ? Icon(icon, size: 18, color: iconTone ?? palette.textMuted)
              : null);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: enabled ? onTap : null,
        child: Container(
          constraints: BoxConstraints(minHeight: minHeight),
          padding: padding,
          decoration: border != null ? BoxDecoration(border: border) : null,
          child: Row(
            children: [
              if (leading != null) ...[leading, const SizedBox(width: 11)],
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    titleWidget ??
                        Text(
                          title ?? '',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: AppText.body.copyWith(
                            fontWeight: FontWeight.w700,
                            color: enabled
                                ? palette.textPrimary
                                : palette.textMuted,
                          ),
                        ),
                    if (subtitleWidget != null)
                      subtitleWidget!
                    else if (subtitle != null) ...[
                      const SizedBox(height: 4),
                      Text(
                        subtitle!,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: AppText.meta.copyWith(color: palette.textSubtle),
                      ),
                    ],
                  ],
                ),
              ),
              if (onTap != null || showChevron || trailing != null)
                const SizedBox(width: 8),
              ?trailing,
              if (showChevron)
                Icon(
                  Icons.chevron_right_rounded,
                  size: 16,
                  color: palette.textSubtle,
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 电源操作按钮（睡眠/重启/关机）。
enum PowerButtonLayout { vertical, horizontal }

class PowerActionButton extends StatelessWidget {
  const PowerActionButton({
    super.key,
    required this.icon,
    required this.label,
    this.onPressed,
    this.danger = false,
    this.layout = PowerButtonLayout.vertical,
    this.height = 62,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onPressed;
  final bool danger;
  final PowerButtonLayout layout;
  final double height;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final tone = danger ? AppColors.red : AppColors.primary;
    return AppCard(
      radius: 9,
      onTap: onPressed,
      child: SizedBox(
        height: height,
        child: layout == PowerButtonLayout.vertical
            ? Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 17, color: tone),
                  const SizedBox(height: 6),
                  Text(
                    label,
                    style: AppText.meta.copyWith(
                      color: danger ? AppColors.red : palette.textMuted,
                    ),
                  ),
                ],
              )
            : Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 17, color: tone),
                  const SizedBox(width: 8),
                  Text(
                    label,
                    style: AppText.body.copyWith(
                      color: danger ? AppColors.red : palette.textMuted,
                    ),
                  ),
                ],
              ),
      ),
    );
  }
}

/// 窗口操作图标按钮（列表/最小化/最大化/关闭）。
class WindowActionButton extends StatelessWidget {
  const WindowActionButton({
    super.key,
    required this.icon,
    required this.tooltip,
    this.onPressed,
    this.danger = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  final bool danger;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return IconBtn(
      icon: icon,
      tooltip: tooltip,
      size: 36,
      iconSize: 16,
      radius: 8,
      color: danger ? AppColors.red : palette.textMuted,
      onPressed: onPressed,
    );
  }
}

/// 当前窗口卡片：图标 + 标题/元信息 + 窗口操作按钮组。
class ActiveWindowCard extends StatelessWidget {
  const ActiveWindowCard({
    super.key,
    required this.title,
    required this.meta,
    required this.enabled,
    required this.onWindows,
    required this.onMinimize,
    required this.onMaximize,
    required this.onClose,
    this.icon = Icons.sports_esports_rounded,
  });

  final String title;
  final String meta;
  final bool enabled;
  final VoidCallback onWindows;
  final VoidCallback onMinimize;
  final VoidCallback onMaximize;
  final VoidCallback onClose;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 64,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      child: Row(
        children: [
          TileIcon(
            icon: icon,
            color: AppColors.violet,
            size: 38,
            radius: 10,
            iconSize: 18,
            alpha: 0.16,
          ),
          const SizedBox(width: 9),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.body.copyWith(
                    fontWeight: FontWeight.w700,
                    color: palette.textPrimary,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  meta,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          WindowActionButton(
            icon: Icons.grid_view_rounded,
            tooltip: '窗口列表',
            onPressed: enabled ? onWindows : null,
          ),
          WindowActionButton(
            icon: Icons.remove_rounded,
            tooltip: '最小化',
            onPressed: enabled ? onMinimize : null,
          ),
          WindowActionButton(
            icon: Icons.crop_square_rounded,
            tooltip: '最大化',
            onPressed: enabled ? onMaximize : null,
          ),
          WindowActionButton(
            icon: Icons.close_rounded,
            tooltip: '关闭',
            danger: true,
            onPressed: enabled ? onClose : null,
          ),
        ],
      ),
    );
  }
}

/// 不确定进度条，对齐原型的状态卡片进度动画。
class AppProgressBar extends StatelessWidget {
  const AppProgressBar({super.key, this.height = 4});

  final double height;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return ClipRRect(
      borderRadius: BorderRadius.circular(height),
      child: LinearProgressIndicator(
        minHeight: height,
        color: AppColors.primary,
        backgroundColor: palette.textPrimary.withValues(alpha: 0.1),
      ),
    );
  }
}
