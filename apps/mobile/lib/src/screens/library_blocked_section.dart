/// 「已屏蔽」分区：自己负责拉取屏蔽列表与恢复条目。
///
/// 列表数据只在进入这个分区时才需要，所以放在分区内部，宿主的 `LibraryTab`
/// 不必为它保存状态。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/launcher_controller.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import 'library_tiles.dart';

/// 「已屏蔽」列表：加载、空态与「恢复」动作都在这里闭环。
class LibraryBlockedSection extends ConsumerStatefulWidget {
  const LibraryBlockedSection({super.key, required this.iconRevision});

  /// 宿主维护的图标版本号：图标被替换后用它触发重绘。
  final int Function(String id) iconRevision;

  @override
  ConsumerState<LibraryBlockedSection> createState() =>
      _LibraryBlockedSectionState();
}

class _LibraryBlockedSectionState extends ConsumerState<LibraryBlockedSection> {
  List<LauncherItem> _blocked = const [];
  bool _blockedLoading = false;

  @override
  void initState() {
    super.initState();
    // 首帧就显示加载态，避免进分区时先闪一下空态。
    _blockedLoading = true;
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadBlocked());
  }

  Future<void> _loadBlocked() async {
    if (!mounted) return;
    setState(() => _blockedLoading = true);
    try {
      final items = await ref.read(launcherControllerProvider).blockedItems();
      if (!mounted) return;
      setState(() {
        _blocked = items;
        _blockedLoading = false;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() => _blockedLoading = false);
      context.showErrorSnackBar('读取屏蔽列表失败: $error');
    }
  }

  /// 恢复条目：数据一直在 PC 上，这里只是让它重新回到列表里。
  Future<void> _restore(LauncherItem item) async {
    final ok = await ref
        .read(launcherControllerProvider)
        .setHidden(item.id, false);
    if (!mounted) return;
    if (!ok) {
      context.showErrorSnackBar('恢复失败');
      return;
    }
    context.showSuccessSnackBar('已恢复 ${item.name}');
    await _loadBlocked();
  }

  @override
  Widget build(BuildContext context) {
    if (_blockedLoading && _blocked.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_blocked.isEmpty) {
      return const EmptyState(
        icon: Icons.visibility_off_outlined,
        title: '没有已屏蔽的程序',
        message: '在「软件」或「游戏」里用菜单的「屏蔽」把用不到的程序收起来，需要时在这里恢复。',
      );
    }
    return RefreshIndicator(
      onRefresh: _loadBlocked,
      color: AppColors.primary,
      child: ListView.builder(
        padding: const EdgeInsets.fromLTRB(
          AppMetrics.pagePadding,
          0,
          AppMetrics.pagePadding,
          AppMetrics.tabBarInset,
        ),
        itemCount: _blocked.length,
        itemBuilder: (context, index) {
          final item = _blocked[index];
          return RepaintBoundary(
            child: LibraryRowShell(
              first: index == 0,
              last: index == _blocked.length - 1,
              child: LibraryBlockedTile(
                item: item,
                iconRevision: widget.iconRevision(item.id),
                onRestore: () => _restore(item),
              ),
            ),
          );
        },
      ),
    );
  }
}
