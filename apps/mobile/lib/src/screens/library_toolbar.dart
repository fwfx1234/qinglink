/// 「库」顶部工具栏：分区筛选标签 + 目录分区的搜索框与「添加 / 扫描」按钮。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import 'library_filter.dart';

/// 「库」的筛选标签条与目录分区搜索条。
///
/// 只有「软件 / 游戏」这类服务端分区才需要搜索与扫描（[LibraryFilter.serverKind]
/// 非空时）；加载中的细进度条也放在这里，提示"还在拉数据"。
class LibraryToolbar extends StatelessWidget {
  const LibraryToolbar({
    super.key,
    required this.filter,
    required this.controller,
    required this.scanning,
    required this.loading,
    required this.onFilterChanged,
    required this.onSearchChanged,
    required this.onSearchCleared,
    required this.onAddPressed,
  });

  /// 当前分区。
  final LibraryFilter filter;

  /// 搜索框控制器，由宿主持有（切分区时清空）。
  final TextEditingController controller;

  /// 是否正在扫描：扫描期间「＋」置灰。
  final bool scanning;

  /// 当前分区是否正在加载目录。
  final bool loading;

  /// 切换分区。
  final ValueChanged<LibraryFilter> onFilterChanged;

  /// 搜索词变化（防抖由宿主实现）。
  final ValueChanged<String> onSearchChanged;

  /// 清空搜索框。
  final VoidCallback onSearchCleared;

  /// 点击「＋」打开添加 / 扫描菜单。
  final VoidCallback onAddPressed;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 14, 18, 0),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                for (final value in LibraryFilter.values) ...[
                  if (value != LibraryFilter.values.first)
                    const SizedBox(width: 7),
                  FilterChipX(
                    label: value.label,
                    active: filter == value,
                    onTap: () => onFilterChanged(value),
                  ),
                ],
              ],
            ),
          ),
        ),
        if (filter.serverKind != null) ...[
          const SizedBox(height: 12),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 18),
            child: Row(
              children: [
                Expanded(
                  child: SearchField(
                    controller: controller,
                    hint: '搜索程序',
                    onChanged: onSearchChanged,
                    onClear: onSearchCleared,
                  ),
                ),
                const SizedBox(width: 8),
                IconBtn(
                  icon: Icons.add_rounded,
                  tooltip: '添加 / 扫描',
                  onPressed: scanning ? null : onAddPressed,
                ),
              ],
            ),
          ),
          if (scanning || loading)
            const Padding(
              padding: EdgeInsets.only(top: 12),
              child: LinearProgressIndicator(
                minHeight: 2,
                color: AppColors.primary,
              ),
            ),
        ],
      ],
    );
  }
}
