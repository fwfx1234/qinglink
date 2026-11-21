/// 「库」各分区的列表视图：常用、最近、目录（含空态与加载态）。
///
/// 视图只负责布局与空态；启动、菜单、加入/移出常用等动作由宿主通过
/// [LibraryViewActions] 注入，视图不持有任何可变状态。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../providers/favorites_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import 'library_filter.dart';
import 'library_tiles.dart';

/// 列表视图与宿主之间的动作集合。
class LibraryViewActions {
  const LibraryViewActions({
    required this.isLaunching,
    required this.iconRevision,
    required this.onLaunch,
    required this.onCatalogMenu,
    required this.onFavoriteMenu,
    required this.onAdd,
    required this.onRemove,
  });

  /// 条目是否正在启动中（用于禁用整行并换成转圈）。
  final bool Function(String id) isLaunching;

  /// 图标版本号：图标被替换后自增，用于让图标重新拉取。
  final int Function(String id) iconRevision;

  /// 启动目录条目。
  final void Function(LauncherItem item) onLaunch;

  /// 打开目录条目的管理菜单。
  final void Function(LauncherItem item) onCatalogMenu;

  /// 打开常用条目的管理菜单。
  final void Function(LauncherItem item) onFavoriteMenu;

  /// 把条目加入常用。
  final void Function(LauncherItem item) onAdd;

  /// 把条目移出常用。
  final void Function(LauncherItem item) onRemove;
}

/// 「常用」分区：整行点击启动，管理动作收进菜单。
class LibraryCommonView extends StatelessWidget {
  const LibraryCommonView({
    super.key,
    required this.favorites,
    required this.actions,
    required this.onRefresh,
  });

  /// 服务端下发的常用列表（含加载态与错误信息）。
  final FavoritesState favorites;

  /// 行内动作集合。
  final LibraryViewActions actions;

  /// 下拉刷新。
  final Future<void> Function() onRefresh;

  @override
  Widget build(BuildContext context) {
    final apps = favorites.items;
    if (apps.isEmpty) {
      return EmptyState(
        icon: Icons.library_add_outlined,
        title: favorites.loading ? '正在读取…' : '常用还是空的',
        message: favorites.error ??
            '去「软件」或「游戏」里挑选程序加入常用；列表太长时用菜单里的「屏蔽」收起来。',
      );
    }
    return RefreshIndicator(
      onRefresh: onRefresh,
      color: context.palette.brand,
      child: ListView.builder(
        padding: const EdgeInsets.fromLTRB(
          AppMetrics.pagePadding,
          0,
          AppMetrics.pagePadding,
          AppMetrics.tabBarInset,
        ),
        itemCount: apps.length,
        itemBuilder: (context, index) {
          final item = apps[index];
          return LibraryRowShell(
            first: index == 0,
            last: index == apps.length - 1,
            child: LibraryAppTile(
              item: item,
              launching: actions.isLaunching(item.id),
              iconRevision: actions.iconRevision(item.id),
              onLaunch: () => actions.onLaunch(item),
              onMenu: () => actions.onFavoriteMenu(item),
            ),
          );
        },
      ),
    );
  }
}

/// 「最近」分区：和目录条目同一套行，额外标出是否已在常用里。
class LibraryRecentView extends StatelessWidget {
  const LibraryRecentView({
    super.key,
    required this.items,
    required this.favorites,
    required this.actions,
    required this.onRefresh,
  });

  /// 最近使用的条目。
  final List<LauncherItem> items;

  /// 常用列表，用来判断行上的「＋ / 已添加」。
  final FavoritesState favorites;

  /// 行内动作集合。
  final LibraryViewActions actions;

  /// 下拉刷新。
  final Future<void> Function() onRefresh;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const EmptyState(
        icon: Icons.history_rounded,
        title: '还没有最近使用',
        message: '启动程序后会出现在这里。',
      );
    }
    return RefreshIndicator(
      onRefresh: onRefresh,
      color: context.palette.brand,
      child: ListView.builder(
        padding: const EdgeInsets.fromLTRB(
          AppMetrics.pagePadding,
          0,
          AppMetrics.pagePadding,
          AppMetrics.tabBarInset,
        ),
        itemCount: items.length,
        itemBuilder: (context, index) {
          final item = items[index];
          final added = favorites.items.any((e) => e.id == item.id);
          return RepaintBoundary(
            child: LibraryRowShell(
              first: index == 0,
              last: index == items.length - 1,
              child: LibraryCatalogTile(
                item: item,
                added: added,
                launching: actions.isLaunching(item.id),
                iconRevision: actions.iconRevision(item.id),
                onMenu: () => actions.onCatalogMenu(item),
                onLaunch: () => actions.onLaunch(item),
                onAdd: () => actions.onAdd(item),
                onRemove: () => actions.onRemove(item),
              ),
            ),
          );
        },
      ),
    );
  }
}

/// 「软件 / 游戏」分区：分页列表，底部有「加载更多」转圈。
class LibraryCatalogView extends StatelessWidget {
  const LibraryCatalogView({
    super.key,
    required this.items,
    required this.favorites,
    required this.filter,
    required this.hasMore,
    required this.loading,
    required this.controller,
    required this.actions,
    required this.onRefresh,
  });

  /// 当前服务端库返回的条目。
  final List<LauncherItem> items;

  /// 常用列表，用来判断行上的「＋ / 已添加」。
  final FavoritesState favorites;

  /// 当前分区，决定空态的文案与图标。
  final LibraryFilter filter;

  /// 是否还有下一页（用于在列表末尾显示转圈）。
  final bool hasMore;

  /// 是否正在加载（空态时提示「正在读取…」）。
  final bool loading;

  /// 滚动控制器，由宿主持有以便触底加载下一页。
  final ScrollController controller;

  /// 行内动作集合。
  final LibraryViewActions actions;

  /// 下拉刷新。
  final Future<void> Function() onRefresh;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return EmptyState(
        icon: filter == LibraryFilter.games
            ? Icons.sports_esports_outlined
            : Icons.search_off_rounded,
        title: loading ? '正在读取…' : '没有找到程序',
        message: switch (filter) {
          LibraryFilter.games => '未检测到游戏，可点右上角「＋」重新扫描或扫描游戏文件夹。',
          LibraryFilter.software => '试试搜索，或用右上角「＋」扫描开始菜单 / 软件文件夹。',
          _ => '试试搜索，或用右上角「＋」扫描。',
        },
      );
    }
    return RefreshIndicator(
      onRefresh: onRefresh,
      color: context.palette.brand,
      child: ListView.builder(
        controller: controller,
        padding: const EdgeInsets.fromLTRB(
          AppMetrics.pagePadding,
          0,
          AppMetrics.pagePadding,
          AppMetrics.tabBarInset,
        ),
        itemCount: items.length + (hasMore ? 1 : 0),
        itemBuilder: (context, index) {
          if (index >= items.length) {
            return const Padding(
              padding: EdgeInsets.all(16),
              child: Center(
                child: SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            );
          }
          final item = items[index];
          final added = favorites.items.any((e) => e.id == item.id);
          return RepaintBoundary(
            child: LibraryRowShell(
              first: index == 0,
              last: index == items.length - 1,
              child: LibraryCatalogTile(
                item: item,
                added: added,
                launching: actions.isLaunching(item.id),
                iconRevision: actions.iconRevision(item.id),
                onMenu: () => actions.onCatalogMenu(item),
                onLaunch: () => actions.onLaunch(item),
                onAdd: () => actions.onAdd(item),
                onRemove: () => actions.onRemove(item),
              ),
            ),
          );
        },
      ),
    );
  }
}
