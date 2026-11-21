/// 页面与区块骨架：标题区、空状态、设置分组卡片。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_card.dart';
import 'app_text_styles.dart';

/// 区块标题：`标题 + 可选右侧说明`。
class SectionHeading extends StatelessWidget {
  const SectionHeading({
    super.key,
    required this.title,
    this.trailing,
    this.bottom = AppSpacing.md,
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

/// 空状态：图标 + 标题 + 说明 + 引导动作（设计规则 8「空态即引导」）。
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
            width: 52,
            height: 52,
            decoration: BoxDecoration(
              color: palette.surface2,
              borderRadius: BorderRadius.circular(AppRadius.md),
              border: Border.all(color: palette.line),
            ),
            child: Icon(icon, size: 24, color: palette.textSubtle),
          ),
          const SizedBox(height: AppSpacing.lg),
          Text(
            title,
            style: AppText.cardTitle.copyWith(color: palette.textPrimary),
          ),
          if (message != null) ...[
            const SizedBox(height: AppSpacing.sm),
            Text(
              message!,
              textAlign: TextAlign.center,
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
          ],
          if (action != null) ...[const SizedBox(height: AppSpacing.lg), action!],
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
                  const SizedBox(height: 2),
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

/// 设置分组卡片：标题 + 右侧值 + 内容。
class SettingsGroup extends StatelessWidget {
  const SettingsGroup({
    super.key,
    required this.title,
    required this.children,
    this.trailing,
    this.trailingColor,
    this.padding = const EdgeInsets.all(AppMetrics.cardPadding),
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
