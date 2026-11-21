/// 底部弹层的统一外壳与表头。
///
/// 规格来自 `docs/design/DESIGN-SPEC.md` 第 6 节：顶部圆角 22、把手 36×4、
/// 内容左右 20、底部安全区额外 +16、遮罩取 `scrim` token。
/// 所有弹层都应通过 [showAppSheet] 打开，不要各自调用 `showModalBottomSheet`。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_text_styles.dart';

/// 打开统一规格的底部弹层。
///
/// 宽度自适应屏幕，高度由内容决定；需要固定高度的弹层可在 [builder] 里
/// 套 `FractionallySizedBox` 或 `SizedBox`。
Future<T?> showAppSheet<T>(
  BuildContext context, {
  required WidgetBuilder builder,
  bool isScrollControlled = true,
  bool useSafeArea = true,
  bool showHandle = true,
}) {
  final palette = context.palette;
  return showModalBottomSheet<T>(
    context: context,
    isScrollControlled: isScrollControlled,
    useSafeArea: useSafeArea,
    showDragHandle: showHandle,
    backgroundColor: palette.dialogBackground,
    barrierColor: palette.scrim,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadius.xl)),
    ),
    builder: builder,
  );
}

/// 弹层内容容器：负责左右 20、底部安全区 +16 的留白。
class AppSheetBody extends StatelessWidget {
  const AppSheetBody({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.fromLTRB(
      AppSpacing.xl,
      0,
      AppSpacing.xl,
      AppSpacing.lg,
    ),
    this.scrollable = false,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;

  /// 内容可能超出一屏时置 true，底部安全区只补一次。
  final bool scrollable;

  @override
  Widget build(BuildContext context) {
    final bottom = MediaQuery.of(context).viewPadding.bottom + AppSpacing.lg;
    return Padding(
      padding: EdgeInsets.only(
        left: AppSpacing.xl,
        right: AppSpacing.xl,
        bottom: bottom,
      ),
      child: scrollable
          ? SingleChildScrollView(child: child)
          : Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [child],
            ),
    );
  }
}

/// 弹层表头：标题 14 SemiBold + 说明 12 + 可选右侧动作。
class SheetHeader extends StatelessWidget {
  const SheetHeader({
    super.key,
    required this.title,
    this.description,
    this.trailing,
  });

  final String title;
  final String? description;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.only(bottom: AppSpacing.md),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  style: AppText.rowTitle.copyWith(
                    color: palette.textPrimary,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                if (description != null) ...[
                  const SizedBox(height: 2),
                  Text(
                    description!,
                    style: AppText.meta.copyWith(color: palette.textMuted),
                  ),
                ],
              ],
            ),
          ),
          ?trailing,
        ],
      ),
    );
  }
}

/// 弹层里的选项行：图标 + 标题 + 可选副标题，高度 56。
class SheetOption extends StatelessWidget {
  const SheetOption({
    super.key,
    required this.title,
    this.icon,
    this.subtitle,
    this.danger = false,
    this.trailing,
    this.onTap,
  });

  final String title;
  final IconData? icon;
  final String? subtitle;
  final bool danger;
  final Widget? trailing;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final color = danger ? palette.danger : palette.textPrimary;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(AppRadius.md),
      child: Container(
        constraints: const BoxConstraints(minHeight: 56),
        padding: const EdgeInsets.symmetric(vertical: AppSpacing.sm),
        child: Row(
          children: [
            if (icon != null) ...[
              Icon(
                icon,
                size: 20,
                color: danger ? palette.danger : palette.textMuted,
              ),
              const SizedBox(width: AppSpacing.md),
            ],
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    title,
                    style: AppText.body.copyWith(color: color),
                  ),
                  if (subtitle != null) ...[
                    const SizedBox(height: 2),
                    Text(
                      subtitle!,
                      style: AppText.meta.copyWith(color: palette.textMuted),
                    ),
                  ],
                ],
              ),
            ),
            ?trailing,
          ],
        ),
      ),
    );
  }
}
