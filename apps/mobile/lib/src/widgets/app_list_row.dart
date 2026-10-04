/// 通用列表行，供设置行、菜单行与信息行复用。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_card.dart';
import 'app_text_styles.dart';

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
