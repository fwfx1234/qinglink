/// 「库」列表里的行与卡片：常用条目、目录条目、已屏蔽条目。
///
/// 设计稿里库列表是**一张卡片包住多行、行间用 1px 分隔线**，而不是每行一张卡，
/// 行右侧只留一个「…」管理入口（启动靠整行点击）。为了保留 ListView 的懒加载，
/// 这里用 [LibraryRowShell] 逐行拼出卡片外观：首行画上圆角与上边框，
/// 末行画下圆角与下边框，中间行只画左右边框和一条分隔线。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';
import '../widgets/app_icon.dart';
import '../widgets/app_ui.dart';
import 'library_entry_utils.dart';

/// 行外壳：把一行画成「大卡片的一段」。
class LibraryRowShell extends StatelessWidget {
  const LibraryRowShell({
    super.key,
    required this.child,
    required this.first,
    required this.last,
  });

  final Widget child;

  /// 是否列表首行（画上圆角与上边框）。
  final bool first;

  /// 是否列表末行（画下圆角与下边框，且不再画分隔线）。
  final bool last;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    // 注意：这里不要给 decoration 加 borderRadius —— 该组合在本项目当前
    // Flutter 版本上会把子内容整块吞掉（首末行变成空白）。圆角改用外层
    // ClipRRect 裁，描边仍走非圆角的 Border。
    final box = DecoratedBox(
      decoration: BoxDecoration(
        color: palette.surface,
        border: Border(
          left: BorderSide(color: palette.line),
          right: BorderSide(color: palette.line),
          top: BorderSide(color: first ? palette.line : Colors.transparent),
          bottom: BorderSide(color: last ? palette.line : Colors.transparent),
        ),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          child,
          if (!last)
            Divider(
              height: 1,
              thickness: 1,
              color: palette.line,
              indent: AppSpacing.md,
              endIndent: AppSpacing.md,
            ),
        ],
      ),
    );
    if (!first && !last) return box;
    return ClipRRect(
      borderRadius: BorderRadius.vertical(
        top: Radius.circular(first ? AppRadius.lg : 0),
        bottom: Radius.circular(last ? AppRadius.lg : 0),
      ),
      child: box,
    );
  }
}

/// 行内容：图标 + 标题/副标题 + 右侧动作，整行可点。
class _LibraryRow extends StatelessWidget {
  const _LibraryRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.subtitleColor,
    this.onTap,
    this.trailing,
  });

  final Widget icon;
  final String title;
  final String subtitle;
  final Color subtitleColor;
  final VoidCallback? onTap;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(AppRadius.lg),
        child: Container(
          constraints: const BoxConstraints(minHeight: 56),
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.md,
            vertical: AppSpacing.sm,
          ),
          child: Row(
            children: [
              icon,
              const SizedBox(width: AppSpacing.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.rowTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                    if (subtitle.isNotEmpty) ...[
                      const SizedBox(height: 2),
                      Text(
                        subtitle,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: AppText.meta.copyWith(color: subtitleColor),
                      ),
                    ],
                  ],
                ),
              ),
              if (trailing != null) ...[
                const SizedBox(width: AppSpacing.sm),
                trailing!,
              ],
            ],
          ),
        ),
    );
  }
}

/// 常用条目：整行点击启动，管理动作只留一个「…」。
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
    return _LibraryRow(
      icon: AppIcon(
        id: item.id,
        size: 40,
        radius: AppRadius.sm,
        revision: iconRevision,
      ),
      title: item.name,
      subtitle: item.pinned
          ? '已置顶到首页'
          : librarySourceLabel(item.source),
      subtitleColor: item.pinned ? palette.brand : palette.textMuted,
      onTap: launching ? null : onLaunch,
      trailing: launching
          ? const SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : IconBtn(
              icon: Icons.more_horiz_rounded,
              tooltip: '管理',
              size: 36,
              iconSize: 18,
              radius: AppRadius.sm,
              onPressed: onMenu,
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
    return _LibraryRow(
      icon: AppIcon(
        id: item.id,
        initialBase64: item.iconBase64,
        size: 44,
        radius: AppRadius.md,
        revision: iconRevision,
      ),
      title: item.name,
      subtitle: librarySourceLabel(item.source),
      subtitleColor: palette.textMuted,
      onTap: launching ? null : onLaunch,
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (launching)
            const SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          else
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: AppSpacing.xs),
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
                      size: 36,
                      iconSize: 18,
                      radius: AppRadius.sm,
                      color: palette.textPrimary,
                      onPressed: onAdd,
                    ),
            ),
          if (onMenu != null)
            IconBtn(
              icon: Icons.more_horiz_rounded,
              tooltip: '管理',
              size: 36,
              iconSize: 18,
              radius: AppRadius.sm,
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
    return _LibraryRow(
      icon: AppIcon(
        id: item.id,
        size: 44,
        radius: AppRadius.md,
        revision: iconRevision,
      ),
      title: item.name,
      subtitle: '已从列表隐藏',
      subtitleColor: palette.textMuted,
      trailing: TextButton(
        onPressed: onRestore,
        child: const Text('恢复'),
      ),
    );
  }
}
