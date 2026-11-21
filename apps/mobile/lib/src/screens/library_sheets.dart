/// 「库」里的底部菜单：目录条目管理、常用条目管理与「添加 / 扫描」。
///
/// 菜单只负责收集用户的选择并返回动作枚举，真正的操作由宿主的动作方法执行。
/// 版式统一走 `showAppSheet` + `SheetHeader` / `SheetOption`：标题 14、选项行 56、
/// 危险项置底，符合设计稿第 6 节。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../widgets/app_sheet.dart';
import '../widgets/icon_picker_sheet.dart';
import 'library_entry_utils.dart';

/// 目录条目管理菜单的动作。
enum LibraryCatalogAction { add, unadd, icon, rename, args, launchMode, block, delete }

/// 常用条目管理菜单的动作。
enum LibraryFavoriteAction { home, icon, rename, args, launchMode, remove }

/// 「添加 / 扫描」菜单的动作。
enum LibraryAddAction { rescan, folderGame, folderApp, file }

/// 目录条目的管理菜单。启动本身是整行点击，所以这里只放"改这个条目"的动作。
///
/// [added] 表示该条目已在常用里，决定第一项是「添加到常用」还是「从常用中移除」。
Future<LibraryCatalogAction?> showLibraryCatalogMenu(
  BuildContext context, {
  required LauncherItem item,
  required bool added,
}) {
  return showAppSheet<LibraryCatalogAction>(
    context,
    builder: (ctx) => AppSheetBody(
      scrollable: true,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SheetHeader(title: item.name, description: '管理这个条目的启动方式'),
          SheetOption(
            icon: added ? Icons.check_circle_rounded : Icons.add_circle_outline,
            title: added ? '从常用中移除' : '添加到常用',
            subtitle: '常用数据保存在 PC 上，换设备也一致',
            onTap: () => Navigator.pop(
              ctx,
              added ? LibraryCatalogAction.unadd : LibraryCatalogAction.add,
            ),
          ),
          SheetOption(
            icon: Icons.image_outlined,
            title: '更换图标',
            subtitle: iconSourceLabel(item.iconSource),
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.icon),
          ),
          SheetOption(
            icon: Icons.drive_file_rename_outline,
            title: '重命名',
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.rename),
          ),
          SheetOption(
            icon: Icons.tune_rounded,
            title: '启动参数',
            subtitle: item.args.isEmpty ? '未设置' : item.args.join(' '),
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.args),
          ),
          if (item.category.toLowerCase() == 'game')
            SheetOption(
              icon: Icons.display_settings_outlined,
              title: '启动分辨率',
              subtitle: item.launchMode?.label ?? '跟随当前分辨率',
              onTap: () => Navigator.pop(ctx, LibraryCatalogAction.launchMode),
            ),
          SheetOption(
            icon: Icons.visibility_off_outlined,
            title: '屏蔽（不再出现在列表里）',
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.block),
          ),
          if (isUserAddedLibraryItem(item.source))
            SheetOption(
              icon: Icons.delete_outline_rounded,
              title: '删除',
              danger: true,
              onTap: () => Navigator.pop(ctx, LibraryCatalogAction.delete),
            ),
        ],
      ),
    ),
  );
}

/// 常用条目的管理菜单。条目本身就是目录条目，所以动作直接落到 PC。
Future<LibraryFavoriteAction?> showLibraryFavoriteMenu(
  BuildContext context, {
  required LauncherItem item,
}) {
  return showAppSheet<LibraryFavoriteAction>(
    context,
    builder: (ctx) => AppSheetBody(
      scrollable: true,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SheetHeader(title: item.name, description: '常用条目'),
          SheetOption(
            icon: item.pinned
                ? Icons.push_pin_rounded
                : Icons.push_pin_outlined,
            title: item.pinned ? '取消置顶到首页' : '置顶到首页',
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.home),
          ),
          SheetOption(
            icon: Icons.image_outlined,
            title: '更换图标',
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.icon),
          ),
          SheetOption(
            icon: Icons.drive_file_rename_outline,
            title: '重命名',
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.rename),
          ),
          SheetOption(
            icon: Icons.tune_rounded,
            title: '启动参数',
            subtitle: item.args.isEmpty ? '未设置' : formatLaunchArgs(item.args),
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.args),
          ),
          if (item.category.toLowerCase() == 'game')
            SheetOption(
              icon: Icons.display_settings_outlined,
              title: '启动分辨率',
              subtitle: item.launchMode?.label ?? '跟随当前分辨率',
              onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.launchMode),
            ),
          SheetOption(
            icon: Icons.delete_outline_rounded,
            title: '从库中移除',
            danger: true,
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.remove),
          ),
        ],
      ),
    ),
  );
}

/// 「添加 / 扫描」菜单：重新扫描、扫描文件夹、挑选单个程序文件。
Future<LibraryAddAction?> showLibraryAddMenu(BuildContext context) {
  return showAppSheet<LibraryAddAction>(
    context,
    builder: (ctx) => AppSheetBody(
      scrollable: true,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SheetHeader(
            title: '添加程序',
            description: '扫描在电脑上执行，结果回到手机确认',
          ),
          SheetOption(
            icon: Icons.refresh_rounded,
            title: '重新扫描',
            subtitle: '开始菜单 + Steam',
            onTap: () => Navigator.pop(ctx, LibraryAddAction.rescan),
          ),
          SheetOption(
            icon: Icons.sports_esports_outlined,
            title: '扫描游戏文件夹',
            subtitle: '每个游戏目录只推荐一个主程序',
            onTap: () => Navigator.pop(ctx, LibraryAddAction.folderGame),
          ),
          SheetOption(
            icon: Icons.create_new_folder_outlined,
            title: '扫描软件文件夹',
            subtitle: '选择目录，批量添加其中的程序',
            onTap: () => Navigator.pop(ctx, LibraryAddAction.folderApp),
          ),
          SheetOption(
            icon: Icons.desktop_windows_outlined,
            title: '选择程序文件',
            subtitle: '从文件树选择单个 .exe',
            onTap: () => Navigator.pop(ctx, LibraryAddAction.file),
          ),
        ],
      ),
    ),
  );
}
