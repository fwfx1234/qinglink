/// 首页「常用应用」区：置顶应用网格、列表行与空态。
///
/// 数据来源与启动动作由 `home_tab.dart` 接线。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';
import '../widgets/app_icon.dart';
import '../widgets/app_ui.dart';

/// 常用（置顶）应用区：最多展示 4 个，空态引导去库里添加。
class HomeCommonAppsSection extends StatelessWidget {
  const HomeCommonAppsSection({
    super.key,
    required this.apps,
    required this.lastUsedId,
    required this.online,
    required this.onLaunch,
    this.onOpenLibrary,
  });

  final List<LauncherItem> apps;
  final String? lastUsedId;
  final bool online;
  final ValueChanged<LauncherItem> onLaunch;
  final VoidCallback? onOpenLibrary;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 5),
          child: Row(
            children: [
              Text(
                '常用应用',
                style: AppText.sectionTitle.copyWith(
                  color: palette.textPrimary,
                ),
              ),
              const Spacer(),
              if (apps.isNotEmpty)
                GestureDetector(
                  onTap: onOpenLibrary,
                  child: Text(
                    '管理',
                    style: AppText.meta.copyWith(color: AppColors.primary),
                  ),
                ),
            ],
          ),
        ),
        if (apps.isEmpty)
          _EmptyApps(onTap: onOpenLibrary)
        else
          GridView.count(
            crossAxisCount: 2,
            padding: EdgeInsets.zero,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            mainAxisSpacing: 8,
            crossAxisSpacing: 8,
            childAspectRatio: 2.7,
            children: apps
                .take(4)
                .map(
                  (item) => _PinnedApp(
                    item: item,
                    lastUsed: lastUsedId == item.id,
                    enabled: online,
                    onTap: () => onLaunch(item),
                  ),
                )
                .toList(),
          ),
      ],
    );
  }
}

/// 没有置顶应用时的占位行。
class _EmptyApps extends StatelessWidget {
  const _EmptyApps({this.onTap});
  final VoidCallback? onTap;
  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      constraints: const BoxConstraints(minHeight: 56),
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: BoxDecoration(
        border: Border.all(color: palette.line),
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Row(
        children: [
          Text(
            '还没有置顶应用',
            style: AppText.label.copyWith(color: palette.textSubtle),
          ),
          const Spacer(),
          TextButton(
            onPressed: onTap,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  '去库里添加',
                  style: AppText.body.copyWith(color: AppColors.primary),
                ),
                const Icon(
                  Icons.north_east_rounded,
                  size: 14,
                  color: AppColors.primary,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 单个置顶应用卡片：图标、名称与来源标签。
class _PinnedApp extends StatelessWidget {
  const _PinnedApp({
    required this.item,
    required this.lastUsed,
    required this.enabled,
    required this.onTap,
  });
  final LauncherItem item;
  final bool lastUsed;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final meta = lastUsed ? '上次使用' : '程序';
    return Opacity(
      opacity: enabled ? 1 : 0.5,
      child: PressableScale(
        enabled: enabled,
        child: AppCard(
          minHeight: 58,
          padding: const EdgeInsets.all(9),
          radius: AppRadius.lg,
          onTap: enabled ? onTap : null,
          child: Row(
            children: [
              AppIcon(id: item.id, size: 34, radius: 9),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      item.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.label.copyWith(
                        fontWeight: FontWeight.w700,
                        color: palette.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      meta,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.tiny.copyWith(color: palette.textSubtle),
                    ),
                  ],
                ),
              ),
              const Icon(
                Icons.play_arrow_rounded,
                size: 14,
                color: AppColors.primary,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
