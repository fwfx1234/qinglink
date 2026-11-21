/// 首页「常用应用」区：4 列应用网格与空态引导。
///
/// 数据来源与启动动作由 `home_tab.dart` 接线。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';
import '../widgets/app_icon.dart';
import '../widgets/app_ui.dart';

/// 首页应用区：最近启动项优先，最多展示 12 个，空态引导去库里添加。
class HomeCommonAppsSection extends StatelessWidget {
  const HomeCommonAppsSection({
    super.key,
    required this.apps,
    required this.online,
    required this.onLaunch,
    this.onOpenLibrary,
  });

  final List<LauncherItem> apps;
  final bool online;
  final ValueChanged<LauncherItem> onLaunch;
  final VoidCallback? onOpenLibrary;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeading(
          title: '常用应用',
          bottom: AppSpacing.md,
          trailing: apps.isEmpty
              ? null
              : GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: onOpenLibrary,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 2),
                    child: Text(
                      '管理',
                      style: AppText.meta.copyWith(color: palette.textMuted),
                    ),
                  ),
                ),
        ),
        if (apps.isEmpty)
          _EmptyApps(onTap: onOpenLibrary)
        else
          GridView.builder(
            padding: EdgeInsets.zero,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: apps.take(12).length,
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 4,
              mainAxisSpacing: 10,
              crossAxisSpacing: 10,
              // 48 图标 + 8 间隙 + 16 文字行高 = 72，行距合计 84（对齐设计稿）。
              mainAxisExtent: 74,
            ),
            itemBuilder: (context, index) {
              final item = apps[index];
              return _PinnedApp(
                item: item,
                enabled: online,
                onTap: () => onLaunch(item),
              );
            },
          ),
      ],
    );
  }
}

/// 没有置顶应用时的空态：一行说明 + 去库里的入口。
class _EmptyApps extends StatelessWidget {
  const _EmptyApps({this.onTap});
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      constraints: const BoxConstraints(minHeight: 56),
      padding: const EdgeInsets.symmetric(horizontal: 14),
      decoration: BoxDecoration(
        border: Border.all(color: palette.line),
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Row(
        children: [
          Expanded(
            child: Text(
              '还没有常用应用',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: AppText.meta.copyWith(color: palette.textMuted),
            ),
          ),
          TextButton(
            onPressed: onTap,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('去库里添加', style: AppText.label.copyWith(color: palette.brand)),
                const SizedBox(width: 4),
                Icon(Icons.north_east_rounded, size: 14, color: palette.brand),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 单个应用入口：真实图标 + 名称（设计稿为 4 列图标网格，无卡片底）。
class _PinnedApp extends StatelessWidget {
  const _PinnedApp({
    required this.item,
    required this.enabled,
    required this.onTap,
  });

  final LauncherItem item;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Opacity(
      opacity: enabled ? 1 : 0.5,
      child: PressableScale(
        enabled: enabled,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: enabled ? onTap : null,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              AppIcon(
                id: item.id,
                size: 48,
                radius: AppRadius.md,
                tone: palette.brand,
                fallback: Icons.sports_esports_rounded,
              ),
              const SizedBox(height: 8),
              Text(
                item.name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                textAlign: TextAlign.center,
                style: AppText.meta.copyWith(color: palette.textMuted),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
