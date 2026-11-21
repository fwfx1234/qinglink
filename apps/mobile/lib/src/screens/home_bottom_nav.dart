/// 主壳底部导航栏：悬浮胶囊条 + 首页 / 库 / 更多 三个 Tab 目标。
///
/// 由主壳 `home_screen.dart` 装配，只负责导航外观与点击回传；
/// Tab 内容与页面切换分别由 `home_tab.dart` 与主壳负责。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// 单个导航目标。
class _NavItem {
  const _NavItem(this.icon, this.activeIcon, this.label);
  final IconData icon;
  final IconData activeIcon;
  final String label;
}

/// 主壳底部导航栏，选中项由 [currentIndex] 决定，点击通过 [onTap] 回传。
class HomeBottomNav extends StatelessWidget {
  const HomeBottomNav({
    super.key,
    required this.currentIndex,
    required this.onTap,
  });

  final int currentIndex;
  final ValueChanged<int> onTap;

  static const _items = [
    _NavItem(Icons.home_outlined, Icons.home_rounded, '首页'),
    _NavItem(Icons.grid_view_outlined, Icons.grid_view_rounded, '库'),
    _NavItem(Icons.more_horiz_rounded, Icons.more_horiz_rounded, '更多'),
  ];

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SafeArea(
      top: false,
      minimum: const EdgeInsets.only(bottom: AppSpacing.md),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: AppMetrics.pagePadding),
        child: Container(
          height: 62,
          padding: const EdgeInsets.all(6),
          decoration: BoxDecoration(
            color: palette.surface,
            borderRadius: BorderRadius.circular(AppRadius.pill),
            border: Border.all(color: palette.line),
            boxShadow: [
              BoxShadow(
                color: palette.shadow,
                blurRadius: 24,
                offset: const Offset(0, 10),
                spreadRadius: -12,
              ),
            ],
          ),
          child: Row(
            children: [
              for (var index = 0; index < _items.length; index++)
                Expanded(
                  child: _NavButton(
                    item: _items[index],
                    selected: index == currentIndex,
                    onTap: () => onTap(index),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 单个导航按钮：选中态是品牌色填充胶囊。
class _NavButton extends StatelessWidget {
  const _NavButton({
    required this.item,
    required this.selected,
    required this.onTap,
  });

  final _NavItem item;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final tone = selected ? palette.onBrand : palette.textSubtle;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
        decoration: BoxDecoration(
          color: selected ? palette.brand : Colors.transparent,
          borderRadius: BorderRadius.circular(AppRadius.pill),
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              selected ? item.activeIcon : item.icon,
              size: 22,
              color: tone,
            ),
            const SizedBox(height: 2),
            Text(
              item.label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 12,
                height: 1.2,
                fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                color: tone,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
