/// 「库」的添加与扫描动作：挑选程序文件、重新扫描开始菜单 + Steam、扫描文件夹。
///
/// 扫描期间要禁用「＋」并显示进度条，所以扫描标志由宿主持有，这里通过 [scanning]
/// 读取、通过 [onScanningChanged] 写回。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../app/routes.dart';
import '../providers/favorites_controller.dart';
import '../providers/launcher_controller.dart';
import '../providers/library_controller.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/file_picker_sheet.dart';
import 'library_dialogs.dart';

/// 添加 / 扫描动作的执行者，由 `LibraryTab` 注入 [`WidgetRef`]、刷新回调与扫描标志。
class LibraryScanActions {
  const LibraryScanActions({
    required this.ref,
    required this.reload,
    required this.scanning,
    required this.onScanningChanged,
    required this.serverKind,
  });

  /// 由宿主注入：`LibraryTab` 的 [`WidgetRef`]。
  final WidgetRef ref;

  /// 由宿主注入：重新同步常用、最近与当前分区。
  final Future<void> Function() reload;

  /// 由宿主注入：当前是否已在扫描，重复触发直接忽略。
  final bool Function() scanning;

  /// 由宿主注入：写回扫描标志，用于禁用「＋」并显示进度条。
  final ValueChanged<bool> onScanningChanged;

  /// 由宿主注入：当前分区的服务端分类，扫描出结果后要刷新它。
  final String? Function() serverKind;

  /// 从文件树挑一个 .exe 加进库，并默认加入常用。
  Future<void> addFromFile(BuildContext context) async {
    final path = await pickExecutable(context, ref);
    if (path == null) return;
    if (!context.mounted) return;
    final base = path.split(RegExp(r'[\\/]')).last;
    final suggested = base.toLowerCase().endsWith('.exe')
        ? base.substring(0, base.length - 4)
        : base;

    final name = await promptLibraryItemName(context, suggested);
    if (!context.mounted) return;
    final finalName = name?.trim();
    if (finalName == null || finalName.isEmpty) return;

    final id = await ref.read(launcherControllerProvider).add(
      name: finalName,
      exePath: path,
    );
    if (!context.mounted) return;
    if (id == null) {
      context.showErrorSnackBar('添加失败');
      return;
    }
    await ref.read(favoritesProvider.notifier).add(id);
    await reload();
    if (context.mounted) context.showSuccessSnackBar('已添加 $finalName');
  }

  /// 重新扫描开始菜单 + Steam。
  Future<void> scanAll(BuildContext context) async {
    if (scanning()) return;
    onScanningChanged(true);
    try {
      final launcher = ref.read(launcherControllerProvider);
      await launcher.refreshCatalog('app');
      await launcher.refreshCatalog('game');
      if (context.mounted) {
        context.showSuccessSnackBar('已开始扫描：开始菜单 + Steam');
      }
    } catch (error) {
      if (context.mounted) context.showErrorSnackBar('扫描失败: $error');
    } finally {
      onScanningChanged(false);
    }
  }

  /// 打开「扫描文件夹」页面；[kind] 是本次扫描的目录类型（game / app）。
  Future<void> scanFolder(BuildContext context, {String kind = 'game'}) async {
    final added = await context.push<bool>('${AppRoutes.scanFolder}?kind=$kind');
    if (!context.mounted) return;
    if (added == true) {
      final current = serverKind();
      if (current != null) {
        await ref
            .read(libraryControllerProvider.notifier)
            .refresh(kind: current);
      }
    }
    await reload();
  }
}
