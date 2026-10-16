/// 「库」里针对单个条目的管理动作：加入/移出常用、置顶、屏蔽、重命名、启动参数、删除。
///
/// 这些动作形状一致（确认或输入 → 调 PC → 反馈 → 重新同步列表），且只依赖 `ref`、
/// `BuildContext` 与宿主注入的回调，因此不必挤在 `LibraryTab` 的分区状态里。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/favorites_controller.dart';
import '../providers/launcher_controller.dart';
import '../providers/library_controller.dart';
import '../providers/system_controller.dart';
import '../utils/snackbar_extension.dart';
import 'library_dialogs.dart';
import 'library_entry_utils.dart';
import 'library_launch_mode_sheet.dart';

/// 条目管理动作的执行者，由 `LibraryTab` 注入 [`WidgetRef`] 与刷新回调后使用。
class LibraryItemActions {
  const LibraryItemActions({required this.ref, required this.reload});

  /// 由宿主注入：`LibraryTab` 的 [`WidgetRef`]。
  final WidgetRef ref;

  /// 由宿主注入：重新同步常用、最近与当前分区。
  final Future<void> Function() reload;

  /// 加入常用。
  Future<void> addToFavorites(BuildContext context, LauncherItem item) async {
    final ok = await ref.read(favoritesProvider.notifier).add(item.id);
    if (!context.mounted) return;
    if (ok) {
      context.showSuccessSnackBar('已添加到常用');
    } else {
      context.showErrorSnackBar('添加到常用失败');
    }
  }

  /// Remove a user-added program from the PC catalog. 常用 标记随目录行一起
  /// 消失，不需要单独清理。
  Future<void> deleteItem(BuildContext context, LauncherItem item) async {
    final confirmed = await confirmLibraryItemDeletion(context, item);
    if (!confirmed) return;
    if (!context.mounted) return;
    final ok = await ref.read(launcherControllerProvider).delete(item.id);
    if (!context.mounted) return;
    if (ok) {
      context.showSuccessSnackBar('已删除 ${item.name}');
      await reload();
    } else {
      context.showErrorSnackBar('删除失败');
    }
  }

  /// 移出常用；[confirm] 为真时先弹确认框。
  Future<void> removeFromFavorites(
    BuildContext context,
    LauncherItem item, {
    bool confirm = false,
  }) async {
    if (confirm) {
      final confirmed = await confirmLibraryFavoriteRemoval(context, item);
      if (!confirmed) return;
    }
    final ok = await ref.read(favoritesProvider.notifier).remove(item.id);
    if (!context.mounted) return;
    if (ok) {
      context.showInfoSnackBar(
        confirm ? '已从常用移除 ${item.name}' : '已取消添加 ${item.name}',
      );
    } else {
      context.showErrorSnackBar('操作失败');
    }
  }

  /// 置顶 / 取消置顶到首页。
  Future<void> toggleHome(BuildContext context, LauncherItem item) async {
    final ok = await ref
        .read(favoritesProvider.notifier)
        .setPinned(item.id, !item.pinned);
    if (!context.mounted) return;
    if (ok) {
      context.showSuccessSnackBar(item.pinned ? '已从首页取消置顶' : '已置顶到首页');
    } else {
      context.showErrorSnackBar('操作失败');
    }
  }

  /// 屏蔽条目：数据仍留在 PC 上，只是不再出现在手机列表里。
  ///
  /// 这是解决"程序太多"的主要手段——手机端不该为了找三个程序而翻两百条。
  ///
  /// [serverKind] 由宿主提供，屏蔽成功后要刷新当前分区。
  Future<void> blockItem(
    BuildContext context,
    LauncherItem item, {
    required String? Function() serverKind,
  }) async {
    final ok = await ref
        .read(launcherControllerProvider)
        .setHidden(item.id, true);
    if (!context.mounted) return;
    if (!ok) {
      context.showErrorSnackBar('屏蔽失败');
      return;
    }
    context.showSuccessSnackBar('已屏蔽 ${item.name}');
    final kind = serverKind();
    if (kind != null) {
      await ref.read(libraryControllerProvider.notifier).refresh(kind: kind);
    }
    await ref.read(favoritesProvider.notifier).refresh();
  }

  /// Rename a PC catalog entry.
  ///
  /// 重命名直接写服务端，常用列表读的是同一条记录，所以不需要再同步本地副本。
  Future<void> renameItem(BuildContext context, LauncherItem item) async {
    final newName = await promptLibraryRename(context, item.name);
    if (newName == null) return;
    final ok = await ref.read(launcherControllerProvider).rename(
          item.id,
          item.originalName,
          newName,
        );
    if (!context.mounted) return;
    if (!ok) {
      context.showErrorSnackBar('重命名失败');
      return;
    }
    context.showSuccessSnackBar('已重命名为 $newName');
    await reload();
  }

  /// Edit the extra launch arguments for a scanner entry.
  Future<void> editLaunchArgs(BuildContext context, LauncherItem item) async {
    final value = await promptLibraryLaunchArgs(
      context,
      formatLaunchArgs(item.args),
    );
    if (value == null) return;
    if (!context.mounted) return;
    final args = parseLaunchArgs(value);
    if (args == null) {
      context.showErrorSnackBar('启动参数引号不完整');
      return;
    }
    final ok = await ref.read(launcherControllerProvider).update(
          id: item.id,
          name: item.name,
          exePath: item.executablePath,
          args: args,
        );
    if (!context.mounted) return;
    if (!ok) {
      context.showErrorSnackBar('保存启动参数失败');
      return;
    }
    if (!context.mounted) return;
    context.showSuccessSnackBar(args.isEmpty ? '已清除启动参数' : '已保存启动参数');
    await reload();
  }

  /// 配置游戏启动前使用的主显示器模式。
  Future<void> editLaunchMode(BuildContext context, LauncherItem item) async {
    final selection = await showLibraryLaunchModeSheet(
      context,
      item: item,
      controller: ref.read(systemControllerProvider.notifier),
    );
    if (selection == null) return;
    final ok = await ref.read(launcherControllerProvider).update(
      id: item.id,
      name: item.name,
      exePath: item.executablePath,
      launchMode: selection.mode,
      clearLaunchMode: selection.clear,
    );
    if (!context.mounted) return;
    if (!ok) {
      context.showErrorSnackBar('保存启动分辨率失败');
      return;
    }
    context.showSuccessSnackBar(
      selection.clear ? '已清除启动分辨率' : '已保存启动分辨率',
    );
    await reload();
  }
}
