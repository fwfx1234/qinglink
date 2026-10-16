/// 「库」里的底部菜单：目录条目管理、常用条目管理与「添加 / 扫描」。
///
/// 菜单只负责收集用户的选择并返回动作枚举，真正的操作由宿主的动作方法执行。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';
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
  return showModalBottomSheet<LibraryCatalogAction>(
    context: context,
    showDragHandle: true,
    builder: (ctx) => SafeArea(
      child: Wrap(
        children: [
          ListTile(
            leading: Icon(
              added ? Icons.check_circle_rounded : Icons.add_circle_outline,
            ),
            title: Text(added ? '从常用中移除' : '添加到常用'),
            subtitle: const Text('常用数据保存在 PC 上，换设备也一致'),
            onTap: () => Navigator.pop(
              ctx,
              added ? LibraryCatalogAction.unadd : LibraryCatalogAction.add,
            ),
          ),
          ListTile(
            leading: const Icon(Icons.image_outlined),
            title: const Text('更换图标'),
            subtitle: Text(
              iconSourceLabel(item.iconSource),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.icon),
          ),
          ListTile(
            leading: const Icon(Icons.drive_file_rename_outline),
            title: const Text('重命名'),
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.rename),
          ),
          ListTile(
            leading: const Icon(Icons.tune_rounded),
            title: const Text('启动参数'),
            subtitle: Text(
              item.args.isEmpty ? '未设置' : item.args.join(' '),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.args),
          ),
          if (item.category.toLowerCase() == 'game')
            ListTile(
              leading: const Icon(Icons.display_settings_outlined),
              title: const Text('启动分辨率'),
              subtitle: Text(item.launchMode?.label ?? '跟随当前分辨率'),
              onTap: () => Navigator.pop(ctx, LibraryCatalogAction.launchMode),
            ),
          ListTile(
            leading: const Icon(Icons.visibility_off_outlined),
            title: const Text('屏蔽（不再出现在列表里）'),
            onTap: () => Navigator.pop(ctx, LibraryCatalogAction.block),
          ),
          if (isUserAddedLibraryItem(item.source))
            ListTile(
              leading: const Icon(Icons.delete_outline, color: AppColors.red),
              title: const Text('删除'),
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
  return showModalBottomSheet<LibraryFavoriteAction>(
    context: context,
    showDragHandle: true,
    builder: (ctx) => SafeArea(
      child: Wrap(
        children: [
          ListTile(
            leading: Icon(
              item.pinned ? Icons.push_pin_rounded : Icons.push_pin_outlined,
            ),
            title: Text(item.pinned ? '取消置顶到首页' : '置顶到首页'),
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.home),
          ),
          ListTile(
            leading: const Icon(Icons.image_outlined),
            title: const Text('更换图标'),
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.icon),
          ),
          ListTile(
            leading: const Icon(Icons.drive_file_rename_outline),
            title: const Text('重命名'),
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.rename),
          ),
          ListTile(
            leading: const Icon(Icons.tune_rounded),
            title: const Text('启动参数'),
            subtitle: Text(
              item.args.isEmpty ? '未设置' : formatLaunchArgs(item.args),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.args),
          ),
          if (item.category.toLowerCase() == 'game')
            ListTile(
              leading: const Icon(Icons.display_settings_outlined),
              title: const Text('启动分辨率'),
              subtitle: Text(item.launchMode?.label ?? '跟随当前分辨率'),
              onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.launchMode),
            ),
          ListTile(
            leading: const Icon(Icons.delete_outline, color: AppColors.red),
            title: const Text('从库中移除'),
            onTap: () => Navigator.pop(ctx, LibraryFavoriteAction.remove),
          ),
        ],
      ),
    ),
  );
}

/// 「添加 / 扫描」菜单：重新扫描、扫描文件夹、挑选单个程序文件。
Future<LibraryAddAction?> showLibraryAddMenu(BuildContext context) {
  return showModalBottomSheet<LibraryAddAction>(
    context: context,
    showDragHandle: true,
    builder: (ctx) => SafeArea(
      child: Wrap(
        children: [
          ListTile(
            leading: const Icon(Icons.refresh_rounded),
            title: const Text('重新扫描'),
            subtitle: const Text('开始菜单 + Steam'),
            onTap: () => Navigator.pop(ctx, LibraryAddAction.rescan),
          ),
          ListTile(
            leading: const Icon(Icons.sports_esports_outlined),
            title: const Text('扫描游戏文件夹'),
            subtitle: const Text('每个游戏目录只推荐一个主程序'),
            onTap: () => Navigator.pop(ctx, LibraryAddAction.folderGame),
          ),
          ListTile(
            leading: const Icon(Icons.create_new_folder_outlined),
            title: const Text('扫描软件文件夹'),
            subtitle: const Text('选择目录，批量添加其中的程序'),
            onTap: () => Navigator.pop(ctx, LibraryAddAction.folderApp),
          ),
          ListTile(
            leading: const Icon(Icons.desktop_windows_outlined),
            title: const Text('选择程序文件'),
            subtitle: const Text('从文件树选择单个 .exe'),
            onTap: () => Navigator.pop(ctx, LibraryAddAction.file),
          ),
        ],
      ),
    ),
  );
}
