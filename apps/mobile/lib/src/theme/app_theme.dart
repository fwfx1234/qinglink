import 'package:flutter/material.dart';

/// 品牌与语义色 — 深色/浅色两套主题共用。
///
/// 这些颜色在原型 `styles.css` 的 `:root` 与 `body.light` 中保持一致，
/// 因此无需随主题切换。
abstract final class AppColors {
  /// 品牌主色
  static const Color primary = Color(0xFF54D6A6);

  /// 主色深色变体
  static const Color primaryDark = Color(0xFF2DA981);

  /// 主色按钮上的前景色
  static const Color onPrimary = Color(0xFF0B2419);

  /// 成功/在线
  static const Color success = Color(0xFF22C55E);

  /// 警告/进行中
  static const Color warning = Color(0xFFF59E0B);

  /// 错误/危险
  static const Color error = Color(0xFFEF4444);

  /// 信息/青色（鼠标、音量等）
  static const Color info = Color(0xFF06B6D4);

  /// 辅助紫罗兰 #aa8cf4（对齐设计图 --violet）
  static const Color violet = Color(0xFFAA8CF4);

  /// 中性蓝灰 #a8b6bd（对齐设计图 --slate）
  static const Color slate = Color(0xFFA8B6BD);

  /// 蓝色（锁屏等）
  static const Color blue = Color(0xFF3B82F6);

  /// 亮青色 #58d6e8（对齐设计图 --cyan）
  static const Color cyan = Color(0xFF58D6E8);

  /// 琥珀色 #f3bd5b（对齐设计图 --amber）
  static const Color amber = Color(0xFFF3BD5B);

  /// 红色 #f37777（对齐设计图 --red）
  static const Color red = Color(0xFFF37777);

  /// 状态卡片的 kicker 文案色（原型浅色主题未覆盖，两套一致）
  static const Color statusKicker = Color(0xFF9DD7BF);

  /// 进度文案色（原型浅色主题未覆盖，两套一致）
  static const Color progressText = Color(0xFFA9CDBD);
}

/// 随主题切换的基础色板，对应原型 `styles.css` 的 CSS 变量。
///
/// 布局中表达“背景/表面/文字/分隔线”等含义的颜色统一从这里取。
@immutable
class AppPalette extends ThemeExtension<AppPalette> {
  const AppPalette({
    required this.background,
    required this.surface,
    required this.surface2,
    required this.surface3,
    required this.line,
    required this.textPrimary,
    required this.textMuted,
    required this.textSubtle,
    required this.topbarBackground,
    required this.navBackground,
    required this.dialogBackground,
    required this.statusGradientStart,
    required this.statusGradientEnd,
    required this.statusBorder,
    required this.statusTitle,
    required this.statusDescription,
    required this.touchpadStart,
    required this.touchpadEnd,
    required this.touchpadBorder,
    required this.shadow,
  });

  /// 页面背景 --bg
  final Color background;

  /// 卡片/表面 --surface
  final Color surface;

  /// 次级表面 --surface-2
  final Color surface2;

  /// 三级表面 --surface-3
  final Color surface3;

  /// 边框线 --line
  final Color line;

  /// 主文字 --text
  final Color textPrimary;

  /// 次要文字 --muted
  final Color textMuted;

  /// 辅助文字 --subtle
  final Color textSubtle;

  /// 顶栏背景（带透明度，配合模糊）
  final Color topbarBackground;

  /// 底部导航背景（带透明度，配合模糊）
  final Color navBackground;

  /// 对话框/底部弹层背景
  final Color dialogBackground;

  /// 状态卡片渐变起始色
  final Color statusGradientStart;

  /// 状态卡片渐变结束色
  final Color statusGradientEnd;

  /// 状态卡片边框色
  final Color statusBorder;

  /// 状态卡片标题色
  final Color statusTitle;

  /// 状态卡片描述色
  final Color statusDescription;

  /// 触控板渐变起始色
  final Color touchpadStart;

  /// 触控板渐变结束色
  final Color touchpadEnd;

  /// 触控板边框色
  final Color touchpadBorder;

  /// 投影色
  final Color shadow;

  /// 原型默认深色主题（`:root`）。
  static const AppPalette dark = AppPalette(
    background: Color(0xFF101311),
    surface: Color(0xFF171C19),
    surface2: Color(0xFF1E2521),
    surface3: Color(0xFF26302A),
    line: Color(0xFF303B34),
    textPrimary: Color(0xFFF0F4EF),
    textMuted: Color(0xFF9AA69D),
    textSubtle: Color(0xFF6E7B72),
    topbarBackground: Color(0xDB101311),
    navBackground: Color(0xEB101311),
    dialogBackground: Color(0xFF171C19),
    statusGradientStart: Color(0xFF1C3029),
    statusGradientEnd: Color(0xFF17221D),
    statusBorder: Color(0xFF2D5A48),
    statusTitle: Color(0xFFF0F4EF),
    statusDescription: Color(0xFFAEBEB4),
    touchpadStart: Color(0xFF263D32),
    touchpadEnd: Color(0xFF18221C),
    touchpadBorder: Color(0xFF2E5444),
    shadow: Color(0x38000000),
  );

  /// 原型浅色主题（`body.light`）。
  static const AppPalette light = AppPalette(
    background: Color(0xFFF4F7F4),
    surface: Color(0xFFFFFFFF),
    surface2: Color(0xFFEDF2EE),
    surface3: Color(0xFFE2EAE4),
    line: Color(0xFFDBE3DC),
    textPrimary: Color(0xFF17211B),
    textMuted: Color(0xFF647168),
    textSubtle: Color(0xFF8B978E),
    topbarBackground: Color(0xE6F4F7F4),
    navBackground: Color(0xE6F4F7F4),
    dialogBackground: Color(0xFFFFFFFF),
    statusGradientStart: Color(0xFFDFF5E9),
    statusGradientEnd: Color(0xFFEEF8F1),
    statusBorder: Color(0xFFB8E3CC),
    statusTitle: Color(0xFF173A29),
    statusDescription: Color(0xFF4A6353),
    touchpadStart: Color(0xFFD5EEE0),
    touchpadEnd: Color(0xFFEDF7F0),
    touchpadBorder: Color(0xFFB6DBC5),
    shadow: Color(0x1C193E2A),
  );

  @override
  AppPalette copyWith({
    Color? background,
    Color? surface,
    Color? surface2,
    Color? surface3,
    Color? line,
    Color? textPrimary,
    Color? textMuted,
    Color? textSubtle,
    Color? topbarBackground,
    Color? navBackground,
    Color? dialogBackground,
    Color? statusGradientStart,
    Color? statusGradientEnd,
    Color? statusBorder,
    Color? statusTitle,
    Color? statusDescription,
    Color? touchpadStart,
    Color? touchpadEnd,
    Color? touchpadBorder,
    Color? shadow,
  }) {
    return AppPalette(
      background: background ?? this.background,
      surface: surface ?? this.surface,
      surface2: surface2 ?? this.surface2,
      surface3: surface3 ?? this.surface3,
      line: line ?? this.line,
      textPrimary: textPrimary ?? this.textPrimary,
      textMuted: textMuted ?? this.textMuted,
      textSubtle: textSubtle ?? this.textSubtle,
      topbarBackground: topbarBackground ?? this.topbarBackground,
      navBackground: navBackground ?? this.navBackground,
      dialogBackground: dialogBackground ?? this.dialogBackground,
      statusGradientStart: statusGradientStart ?? this.statusGradientStart,
      statusGradientEnd: statusGradientEnd ?? this.statusGradientEnd,
      statusBorder: statusBorder ?? this.statusBorder,
      statusTitle: statusTitle ?? this.statusTitle,
      statusDescription: statusDescription ?? this.statusDescription,
      touchpadStart: touchpadStart ?? this.touchpadStart,
      touchpadEnd: touchpadEnd ?? this.touchpadEnd,
      touchpadBorder: touchpadBorder ?? this.touchpadBorder,
      shadow: shadow ?? this.shadow,
    );
  }

  @override
  AppPalette lerp(ThemeExtension<AppPalette>? other, double t) {
    if (other is! AppPalette) return this;
    return AppPalette(
      background: Color.lerp(background, other.background, t)!,
      surface: Color.lerp(surface, other.surface, t)!,
      surface2: Color.lerp(surface2, other.surface2, t)!,
      surface3: Color.lerp(surface3, other.surface3, t)!,
      line: Color.lerp(line, other.line, t)!,
      textPrimary: Color.lerp(textPrimary, other.textPrimary, t)!,
      textMuted: Color.lerp(textMuted, other.textMuted, t)!,
      textSubtle: Color.lerp(textSubtle, other.textSubtle, t)!,
      topbarBackground: Color.lerp(
        topbarBackground,
        other.topbarBackground,
        t,
      )!,
      navBackground: Color.lerp(navBackground, other.navBackground, t)!,
      dialogBackground: Color.lerp(
        dialogBackground,
        other.dialogBackground,
        t,
      )!,
      statusGradientStart: Color.lerp(
        statusGradientStart,
        other.statusGradientStart,
        t,
      )!,
      statusGradientEnd: Color.lerp(
        statusGradientEnd,
        other.statusGradientEnd,
        t,
      )!,
      statusBorder: Color.lerp(statusBorder, other.statusBorder, t)!,
      statusTitle: Color.lerp(statusTitle, other.statusTitle, t)!,
      statusDescription: Color.lerp(
        statusDescription,
        other.statusDescription,
        t,
      )!,
      touchpadStart: Color.lerp(touchpadStart, other.touchpadStart, t)!,
      touchpadEnd: Color.lerp(touchpadEnd, other.touchpadEnd, t)!,
      touchpadBorder: Color.lerp(touchpadBorder, other.touchpadBorder, t)!,
      shadow: Color.lerp(shadow, other.shadow, t)!,
    );
  }
}

/// 便捷访问当前调色板。
extension AppPaletteContext on BuildContext {
  AppPalette get palette =>
      Theme.of(this).extension<AppPalette>() ?? AppPalette.dark;
}

/// 全局圆角 token。
abstract final class AppRadius {
  static const double sm = 10;
  static const double md = 14;
  static const double lg = 18;
  static const double xl = 22;
  static const double pill = 999;
}

/// 全局间距 token。
abstract final class AppSpacing {
  static const double xs = 4;
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
  static const double xl = 20;
  static const double xxl = 24;
}
