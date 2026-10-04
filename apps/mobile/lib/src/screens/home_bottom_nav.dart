/// 主壳底部导航栏：毛玻璃底 + 首页 / 库 / 更多 三个 Tab 目标。
///
/// 由主壳 `home_screen.dart` 装配，只负责导航外观与点击回传；
/// Tab 内容与页面切换分别由 `home_tab.dart` 与主壳负责。
library;

import 'dart:ui';

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// 主壳底部导航栏，选中项由 [currentIndex] 决定，点击通过 [onTap] 回传。
class HomeBottomNav extends StatelessWidget {
  final int currentIndex;
  final ValueChanged<int> onTap;

  const HomeBottomNav({
    super.key,
    required this.currentIndex,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 16, sigmaY: 16),
        child: Container(
          decoration: BoxDecoration(
            color: palette.navBackground,
            border: Border(
              top: BorderSide(color: palette.line.withValues(alpha: 0.6)),
            ),
          ),
          child: SafeArea(
            top: false,
            child: NavigationBar(
              selectedIndex: currentIndex,
              onDestinationSelected: onTap,
              destinations: const [
                NavigationDestination(
                  icon: Icon(Icons.home_outlined),
                  selectedIcon: Icon(Icons.home_rounded),
                  label: '首页',
                ),
                NavigationDestination(
                  icon: Icon(Icons.grid_view_outlined),
                  selectedIcon: Icon(Icons.grid_view_rounded),
                  label: '库',
                ),
                NavigationDestination(
                  icon: Icon(Icons.more_horiz_rounded),
                  label: '更多',
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
