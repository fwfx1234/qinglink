/// 主壳顶部导航：应用标识（eyebrow）+ 页面标题 + 右侧图标按钮。
///
/// 设计规则 1：顶栏只承载“我在哪”和只读状态，不放主操作。主按钮与高频入口
/// 一律落在拇指区，因此这里只保留主题切换这类全局次要动作。
library;

import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/theme_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';

/// 主壳顶部栏：毛玻璃底 + 「轻联 QingLink」标识 + 页面标题 + 主题切换。
class HomeTopBar extends ConsumerWidget {
  const HomeTopBar({super.key, required this.title, this.trailing});

  /// 当前页面标题（首页 / 库 / 更多）。
  final String title;

  /// 追加在主题切换按钮右侧的动作（可选）。
  final Widget? trailing;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 14, sigmaY: 14),
        child: Container(
          height: AppMetrics.navHeight + MediaQuery.of(context).padding.top,
          padding: EdgeInsets.only(
            top: MediaQuery.of(context).padding.top,
            left: AppMetrics.pagePadding,
            right: AppMetrics.pagePadding,
          ),
          color: palette.topbarBackground,
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(
                      '轻联 QingLink',
                      style: AppText.eyebrow.copyWith(color: palette.textSubtle),
                    ),
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.pageTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: AppSpacing.sm),
              IconBtn(
                icon: isDark
                    ? Icons.light_mode_rounded
                    : Icons.dark_mode_rounded,
                tooltip: isDark ? '切换到浅色' : '切换到深色',
                tonal: true,
                color: palette.textMuted,
                onPressed: () => ref.read(themeModeProvider.notifier).toggle(),
              ),
              ?trailing,
            ],
          ),
        ),
      ),
    );
  }
}
