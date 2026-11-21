/// QingLink 界面调色板：暗色 / 亮色两套语义色 token。
///
/// 取值来源为 `docs/design/DESIGN-SPEC.md` 第 2 节，页面与组件一律从这里取色，
/// 不要在业务代码里写死色值。品牌色与状态色也放在本类中，因为它们同样随主题变化
/// （亮色需要压深才能满足 4.5:1 对比度）。
library;

import 'package:flutter/material.dart';

/// 随主题切换的语义色板。
///
/// 字段名与设计 token 的对应关系写在每个字段的注释里，便于和设计稿对表。
@immutable
class AppPalette extends ThemeExtension<AppPalette> {
  const AppPalette({
    required this.background,
    required this.surface,
    required this.surface2,
    required this.surface3,
    required this.line,
    required this.lineStrong,
    required this.iconBtn,
    required this.textPrimary,
    required this.textMuted,
    required this.textSubtle,
    required this.brand,
    required this.brandDeep,
    required this.brandTint,
    required this.onBrand,
    required this.info,
    required this.violet,
    required this.warn,
    required this.danger,
    required this.dangerTint,
    required this.scrim,
    required this.topbarBackground,
    required this.navBackground,
    required this.dialogBackground,
    required this.statusGradientStart,
    required this.statusGradientEnd,
    required this.statusBorder,
    required this.touchpadStart,
    required this.touchpadEnd,
    required this.touchpadBorder,
    required this.shadow,
  });

  /// `bg.base` 页面背景。
  final Color background;

  /// `bg.surface` 卡片。
  final Color surface;

  /// `bg.raised` 卡片内的次级块、按钮底。
  final Color surface2;

  /// 中性强调块（开关轨道、分段底）。
  final Color surface3;

  /// `line` 常规分隔线、卡片描边。
  final Color line;

  /// `line.strong` 强调描边、输入框边框。
  final Color lineStrong;

  /// `bg.icon-btn` 图标按钮底座。
  final Color iconBtn;

  /// `text.high` 标题、正文。
  final Color textPrimary;

  /// `text.mid` 次要说明。
  final Color textMuted;

  /// `text.low` 路径、时间戳、占位。
  final Color textSubtle;

  /// `brand.500` 品牌主色。
  final Color brand;

  /// 品牌主色的深一级变体（按下态、聚焦边框）。
  final Color brandDeep;

  /// `brand.tint` 品牌浅底（选中态、强调块）。
  final Color brandTint;

  /// `on-brand` 品牌色底上的文字。
  final Color onBrand;

  /// `accent.info` 信息、速度、音量。
  final Color info;

  /// `accent.violet` 快捷键、脚本类。
  final Color violet;

  /// `state.warn` 进行中、警示。
  final Color warn;

  /// `state.danger` 危险操作、离线。
  final Color danger;

  /// `danger.tint` 危险按钮底座。
  final Color dangerTint;

  /// `scrim` 弹层遮罩。
  final Color scrim;

  /// 顶栏背景（带透明度，配合模糊）。
  final Color topbarBackground;

  /// 底部导航背景（带透明度，配合模糊）。
  final Color navBackground;

  /// 对话框 / 底部弹层背景。
  final Color dialogBackground;

  /// 设备状态卡渐变起始色。
  final Color statusGradientStart;

  /// 设备状态卡渐变结束色。
  final Color statusGradientEnd;

  /// 设备状态卡描边。
  final Color statusBorder;

  /// 触控板渐变起始色（不属于 token 体系，设计单独给定）。
  final Color touchpadStart;

  /// 触控板渐变结束色。
  final Color touchpadEnd;

  /// 触控板描边。
  final Color touchpadBorder;

  /// 投影色。
  final Color shadow;

  /// 暗色主题（设计稿默认）。
  static const AppPalette dark = AppPalette(
    background: Color(0xFF0A0D0D),
    surface: Color(0xFF121414),
    surface2: Color(0xFF1A1C1C),
    surface3: Color(0xFF212626),
    line: Color(0xFF212626),
    lineStrong: Color(0xFF303836),
    iconBtn: Color(0xFF141717),
    textPrimary: Color(0xFFEDF2F0),
    textMuted: Color(0xFF8F9996),
    textSubtle: Color(0xFF5E6B66),
    brand: Color(0xFF36D9A1),
    brandDeep: Color(0xFF2BB788),
    brandTint: Color(0xFF0F2921),
    onBrand: Color(0xFF08120E),
    info: Color(0xFF59C7E8),
    violet: Color(0xFFA88CF5),
    warn: Color(0xFFF2B557),
    danger: Color(0xFFFF6B6E),
    dangerTint: Color(0xFF2E1113),
    scrim: Color(0x9E000000),
    topbarBackground: Color(0xDB0A0D0D),
    navBackground: Color(0xEB0A0D0D),
    dialogBackground: Color(0xFF121414),
    statusGradientStart: Color(0xFF12241D),
    statusGradientEnd: Color(0xFF0C1412),
    statusBorder: Color(0xFF1D3B2F),
    touchpadStart: Color(0xFF1E3A2E),
    touchpadEnd: Color(0xFF101A16),
    touchpadBorder: Color(0xFF2A4F3E),
    shadow: Color(0x66000000),
  );

  /// 亮色主题。
  static const AppPalette light = AppPalette(
    background: Color(0xFFF5F7F7),
    surface: Color(0xFFFFFFFF),
    surface2: Color(0xFFFAFCFC),
    surface3: Color(0xFFE7ECEA),
    line: Color(0xFFE7ECEA),
    lineStrong: Color(0xFFD9DEDB),
    iconBtn: Color(0xFFEFF3F1),
    textPrimary: Color(0xFF0D1212),
    textMuted: Color(0xFF5C6963),
    textSubtle: Color(0xFF8C9694),
    brand: Color(0xFF12A87A),
    brandDeep: Color(0xFF0D8A63),
    brandTint: Color(0xFFE5F5F0),
    onBrand: Color(0xFFFFFFFF),
    info: Color(0xFF0D94B8),
    violet: Color(0xFF7D5CE0),
    warn: Color(0xFFB87503),
    danger: Color(0xFFD93B3D),
    dangerTint: Color(0xFFFDECEC),
    scrim: Color(0x520D1712),
    topbarBackground: Color(0xE6F5F7F7),
    navBackground: Color(0xF2FFFFFF),
    dialogBackground: Color(0xFFFFFFFF),
    statusGradientStart: Color(0xFFE5F5F0),
    statusGradientEnd: Color(0xFFF7FCFA),
    statusBorder: Color(0xFFC9E9DD),
    touchpadStart: Color(0xFFD3EFE2),
    touchpadEnd: Color(0xFFEAF8F1),
    touchpadBorder: Color(0xFFB1D9C8),
    shadow: Color(0x14000000),
  );

  /// 按亮度取对应色板。
  static AppPalette of(Brightness brightness) =>
      brightness == Brightness.dark ? dark : light;

  @override
  AppPalette copyWith({
    Color? background,
    Color? surface,
    Color? surface2,
    Color? surface3,
    Color? line,
    Color? lineStrong,
    Color? iconBtn,
    Color? textPrimary,
    Color? textMuted,
    Color? textSubtle,
    Color? brand,
    Color? brandDeep,
    Color? brandTint,
    Color? onBrand,
    Color? info,
    Color? violet,
    Color? warn,
    Color? danger,
    Color? dangerTint,
    Color? scrim,
    Color? topbarBackground,
    Color? navBackground,
    Color? dialogBackground,
    Color? statusGradientStart,
    Color? statusGradientEnd,
    Color? statusBorder,
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
      lineStrong: lineStrong ?? this.lineStrong,
      iconBtn: iconBtn ?? this.iconBtn,
      textPrimary: textPrimary ?? this.textPrimary,
      textMuted: textMuted ?? this.textMuted,
      textSubtle: textSubtle ?? this.textSubtle,
      brand: brand ?? this.brand,
      brandDeep: brandDeep ?? this.brandDeep,
      brandTint: brandTint ?? this.brandTint,
      onBrand: onBrand ?? this.onBrand,
      info: info ?? this.info,
      violet: violet ?? this.violet,
      warn: warn ?? this.warn,
      danger: danger ?? this.danger,
      dangerTint: dangerTint ?? this.dangerTint,
      scrim: scrim ?? this.scrim,
      topbarBackground: topbarBackground ?? this.topbarBackground,
      navBackground: navBackground ?? this.navBackground,
      dialogBackground: dialogBackground ?? this.dialogBackground,
      statusGradientStart: statusGradientStart ?? this.statusGradientStart,
      statusGradientEnd: statusGradientEnd ?? this.statusGradientEnd,
      statusBorder: statusBorder ?? this.statusBorder,
      touchpadStart: touchpadStart ?? this.touchpadStart,
      touchpadEnd: touchpadEnd ?? this.touchpadEnd,
      touchpadBorder: touchpadBorder ?? this.touchpadBorder,
      shadow: shadow ?? this.shadow,
    );
  }

  @override
  AppPalette lerp(ThemeExtension<AppPalette>? other, double t) {
    if (other is! AppPalette) return this;
    Color mix(Color a, Color b) => Color.lerp(a, b, t)!;
    return AppPalette(
      background: mix(background, other.background),
      surface: mix(surface, other.surface),
      surface2: mix(surface2, other.surface2),
      surface3: mix(surface3, other.surface3),
      line: mix(line, other.line),
      lineStrong: mix(lineStrong, other.lineStrong),
      iconBtn: mix(iconBtn, other.iconBtn),
      textPrimary: mix(textPrimary, other.textPrimary),
      textMuted: mix(textMuted, other.textMuted),
      textSubtle: mix(textSubtle, other.textSubtle),
      brand: mix(brand, other.brand),
      brandDeep: mix(brandDeep, other.brandDeep),
      brandTint: mix(brandTint, other.brandTint),
      onBrand: mix(onBrand, other.onBrand),
      info: mix(info, other.info),
      violet: mix(violet, other.violet),
      warn: mix(warn, other.warn),
      danger: mix(danger, other.danger),
      dangerTint: mix(dangerTint, other.dangerTint),
      scrim: mix(scrim, other.scrim),
      topbarBackground: mix(topbarBackground, other.topbarBackground),
      navBackground: mix(navBackground, other.navBackground),
      dialogBackground: mix(dialogBackground, other.dialogBackground),
      statusGradientStart: mix(statusGradientStart, other.statusGradientStart),
      statusGradientEnd: mix(statusGradientEnd, other.statusGradientEnd),
      statusBorder: mix(statusBorder, other.statusBorder),
      touchpadStart: mix(touchpadStart, other.touchpadStart),
      touchpadEnd: mix(touchpadEnd, other.touchpadEnd),
      touchpadBorder: mix(touchpadBorder, other.touchpadBorder),
      shadow: mix(shadow, other.shadow),
    );
  }
}

/// 便捷访问当前调色板。
extension AppPaletteContext on BuildContext {
  AppPalette get palette =>
      Theme.of(this).extension<AppPalette>() ?? AppPalette.dark;
}
