/// 「库」列表里的行与卡片：常用条目、目录条目、已屏蔽条目。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';
import '../widgets/app_icon.dart';
import '../widgets/app_ui.dart';
import 'library_entry_utils.dart';

/// 常用条目：整行点击启动，管理动作收进一个菜单。
class LibraryAppTile extends StatelessWidget {
  const LibraryAppTile({
    super.key,
    required this.item,
    required this.launching,
    required this.iconRevision,
    required this.onLaunch,
    required this.onMenu,
  });

  final LauncherItem item;
  final bool launching;
  final int iconRevision;
  final VoidCallback onLaunch;
  final VoidCallback onMenu;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 68,
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      onTap: launching ? null : onLaunch,
      child: Row(
        children: [
          AppIcon(
            id: item.id,
            size: 40,
            radius: 10,
            revision: iconRevision,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  item.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.cardTitle.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 3),
                Text(
                  item.pinned ? '已置顶到首页' : '点击启动',
                  style: AppText.meta.copyWith(
                    color: item.pinned ? AppColors.primary : palette.textSubtle,
                  ),
                ),
              ],
            ),
          ),
          if (launching)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 11),
              child: SizedBox(
                width: 16,
                height: 16,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            )
          else
            IconBtn(
              icon: Icons.play_arrow_rounded,
              tooltip: '启动',
              size: 38,
              iconSize: 18,
              radius: 9,
              color: AppColors.primary,
              background: AppColors.primary.withValues(alpha: 0.13),
              onPressed: onLaunch,
            ),
          IconBtn(
            icon: Icons.more_horiz_rounded,
            tooltip: '管理',
            size: 38,
            iconSize: 17,
            radius: 9,
            onPressed: onMenu,
          ),
        ],
      ),
    );
  }
}

/// 目录条目：整行点击启动，「＋ / 已添加」一个按钮，其余动作收进菜单。
class LibraryCatalogTile extends StatelessWidget {
  const LibraryCatalogTile({
    super.key,
    required this.item,
    required this.added,
    required this.launching,
    required this.onLaunch,
    required this.onAdd,
    this.onRemove,
    this.onMenu,
    this.iconRevision = 0,
  });

  final LauncherItem item;
  final bool added;
  final bool launching;
  final VoidCallback onLaunch;
  final VoidCallback onAdd;
  final VoidCallback? onRemove;
  final VoidCallback? onMenu;
  final int iconRevision;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 68,
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      onTap: launching ? null : onLaunch,
      child: Row(
        children: [
          AppIcon(
            id: item.id,
            initialBase64: item.iconBase64,
            size: 40,
            radius: 10,
            revision: iconRevision,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  children: [
                    Flexible(
                      child: Text(
                        item.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: AppText.cardTitle.copyWith(
                          color: palette.textPrimary,
                        ),
                      ),
                    ),
                    if (item.iconLooksWrong) ...[
                      const SizedBox(width: 6),
                      const StatusTag('图标待确认', color: AppColors.amber),
                    ],
                  ],
                ),
                const SizedBox(height: 3),
                Text(
                  librarySourceLabel(item.source),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          if (launching)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 11),
              child: SizedBox(
                width: 16,
                height: 16,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            )
          else
            IconBtn(
              icon: Icons.play_arrow_rounded,
              tooltip: '启动',
              size: 38,
              iconSize: 18,
              radius: 9,
              color: AppColors.primary,
              background: AppColors.primary.withValues(alpha: 0.13),
              onPressed: onLaunch,
            ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: added
                ? Tooltip(
                    message: '点击取消添加',
                    child: GestureDetector(
                      onTap: launching ? null : onRemove,
                      behavior: HitTestBehavior.opaque,
                      child: const StatusTag('已添加'),
                    ),
                  )
                : IconBtn(
                    icon: Icons.add_rounded,
                    tooltip: '添加到常用',
                    size: 38,
                    iconSize: 18,
                    radius: 9,
                    color: palette.textPrimary,
                    onPressed: onAdd,
                  ),
          ),
          if (onMenu != null)
            IconBtn(
              icon: Icons.more_horiz_rounded,
              tooltip: '管理',
              size: 38,
              iconSize: 17,
              radius: 9,
              onPressed: onMenu,
            ),
        ],
      ),
    );
  }
}

/// 已屏蔽条目：只有一个「恢复」动作。
class LibraryBlockedTile extends StatelessWidget {
  const LibraryBlockedTile({
    super.key,
    required this.item,
    required this.onRestore,
    this.iconRevision = 0,
  });

  final LauncherItem item;
  final VoidCallback onRestore;
  final int iconRevision;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 62,
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      child: Row(
        children: [
          AppIcon(id: item.id, size: 36, radius: 9, revision: iconRevision),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              item.name,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: AppText.body.copyWith(color: palette.textPrimary),
            ),
          ),
          TextButton(
            onPressed: onRestore,
            child: const Text('恢复'),
          ),
        ],
      ),
    );
  }
}
