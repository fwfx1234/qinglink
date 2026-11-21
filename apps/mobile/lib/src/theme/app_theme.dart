/// 主题入口：语义色门面、圆角 / 间距 / 尺寸 token。
///
/// 色板的实体定义在 `app_palette.dart`（随主题切换），本文件额外提供
/// [AppColors] 这层“当前主题色”门面，让既有代码里的 `AppColors.primary`
/// 这类写法不必逐一改成 `context.palette.primary` 就能跟随明暗切换。
library;

import 'package:flutter/material.dart';

import 'app_palette.dart';

export 'app_palette.dart';

/// 当前主题的语义色门面。
///
/// 值来自 [AppPalette]：应用根节点在构建主题时调用 [useTheme] 注入当前生效的
/// 色板，之后所有 `AppColors.xxx` 读取的都是该主题的取值。需要 `const` 的地方
/// 不要用它，改用 `context.palette` 或直接写字面色。
abstract final class AppColors {
  static AppPalette _active = AppPalette.dark;

  /// 当前生效的色板。
  static AppPalette get active => _active;

  /// 切换当前生效的色板；只允许应用根节点在主题构建时调用。
  static set useTheme(AppPalette palette) => _active = palette;

  /// 品牌主色（暗 `#36D9A1` / 亮 `#12A87A`）。
  static Color get primary => _active.brand;

  /// 品牌主色的深一级变体，用于聚焦边框与按下态。
  static Color get primaryDark => _active.brandDeep;

  /// 品牌色浅底。
  static Color get brandTint => _active.brandTint;

  /// 品牌色底上的前景色。
  static Color get onPrimary => _active.onBrand;

  /// 在线 / 成功。
  static Color get success => _active.brand;

  /// 警告 / 进行中。
  static Color get warning => _active.warn;

  /// 错误 / 危险。
  static Color get error => _active.danger;

  /// 危险操作的浅底。
  static Color get dangerTint => _active.dangerTint;

  /// 信息 / 青色（鼠标、音量等）。
  static Color get info => _active.info;

  /// 辅助紫罗兰。
  static Color get violet => _active.violet;

  /// 中性蓝灰。
  static Color get slate => _active.textMuted;

  /// 蓝色（锁屏等）。
  static Color get blue => _active.info;

  /// 亮青色。
  static Color get cyan => _active.info;

  /// 琥珀色。
  static Color get amber => _active.warn;

  /// 红色。
  static Color get red => _active.danger;

  /// 状态卡 kicker 文案色。
  static Color get statusKicker => _active.brand;

  /// 进度文案色。
  static Color get progressText => _active.textMuted;
}

/// 全局圆角 token。
abstract final class AppRadius {
  /// 元素（图标底衬、输入框）。
  static const double sm = 10;

  /// 卡片。
  static const double md = 14;

  /// 大卡。
  static const double lg = 18;

  /// 弹层与面板。
  static const double xl = 22;

  /// 胶囊。
  static const double pill = 999;
}

/// 全局间距 token（8pt 栅格）。
abstract final class AppSpacing {
  static const double xs = 4;
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
  static const double xl = 20;
  static const double xxl = 24;
  static const double xxxl = 32;
}

/// 基准尺寸 token，取值见设计稿第 1 节。
abstract final class AppMetrics {
  /// 页面左右内边距。
  static const double pagePadding = 16;

  /// 顶栏高度（不含状态栏）。
  static const double navHeight = 52;

  /// 底部 TabBar 高度（含手势条）。
  static const double tabBarHeight = 88;

  /// 页面底部为 TabBar 预留的高度。
  static const double tabBarInset = 88;

  /// 窗口控制 Dock 条高度。
  static const double dockHeight = 56;

  /// 最小点按区。
  static const double minTap = 44;

  /// 主按钮高度。
  static const double primaryButton = 48;

  /// 常规列表行高（设计稿 57，取 56 落到 4pt 栅格）。
  static const double listRow = 56;

  /// 密集列表行高（如按键表）。
  static const double denseRow = 48;

  /// 卡片内边距。
  static const double cardPadding = 16;
}
