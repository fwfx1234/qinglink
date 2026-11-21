/// 「库」顶部工具栏：搜索框 + 分区筛选标签。
///
/// 搜索只在「软件 / 游戏」这类服务端分区生效（[LibraryFilter.serverKind] 非空），
/// 其余分区提示先切换；加载中的细进度条也放在这里，提示"还在拉数据"。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import 'library_filter.dart';

/// 「库」的搜索条与筛选标签条。
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
    final palette = context.palette;
    final searchable = filter.serverKind != null;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(
            AppMetrics.pagePadding,
            AppSpacing.md,
            AppMetrics.pagePadding,
            0,
          ),
          child: Row(
            children: [
              Expanded(
                child: SearchField(
                  controller: controller,
                  hint: searchable ? '搜索应用、游戏或路径' : '切到「软件 / 游戏」后可搜索',
                  onChanged: onSearchChanged,
                  onClear: onSearchCleared,
                ),
              ),
              const SizedBox(width: AppSpacing.sm),
              IconBtn(
                icon: Icons.add_rounded,
                tooltip: '添加 / 扫描',
                iconSize: 20,
                background: palette.brand,
                color: palette.onBrand,
                onPressed: scanning ? null : onAddPressed,
              ),
            ],
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: AppMetrics.pagePadding,
          ),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                for (final value in LibraryFilter.values) ...[
                  if (value != LibraryFilter.values.first)
                    const SizedBox(width: AppSpacing.sm),
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
        if (scanning || loading)
          Padding(
            padding: const EdgeInsets.only(top: AppSpacing.md),
            child: LinearProgressIndicator(
              minHeight: 2,
              color: AppColors.primary,
              backgroundColor: Colors.transparent,
            ),
          ),
      ],
    );
  }
}
