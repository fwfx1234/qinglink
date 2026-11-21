import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/providers/theme_provider.dart';
import 'src/app/app_router.dart';
import 'src/theme/app_theme.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  // 单页构建异常不应该把整块界面变成红屏/灰屏：换成一个可读的兜底页，
  // 用户还能系统返回上一页；异常本身仍然打印到日志（debug 下附带摘要）。
  ErrorWidget.builder = _buildBrokenPage;
  runApp(const ProviderScope(child: QingLinkApp()));
}

/// 某个页面的 build 抛异常时代替默认错误块显示的兜底页。
///
/// 这里不能依赖 App 主题或 Provider（出错的可能正是它们），因此配色与
/// 文字样式全部写死，并显式提供 [Directionality]，保证在最坏情况下也能渲染。
Widget _buildBrokenPage(FlutterErrorDetails details) {
  debugPrint('页面渲染失败：${details.exceptionAsString()}');
  return Directionality(
    textDirection: TextDirection.ltr,
    child: ColoredBox(
      color: const Color(0xFFF4F6F5),
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.error_outline_rounded,
                size: 28,
                color: Color(0xFFB3261E),
              ),
              const SizedBox(height: 12),
              const Text(
                '这个页面没能加载出来',
                textAlign: TextAlign.center,
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  color: Color(0xFF1B1F1D),
                ),
              ),
              const SizedBox(height: 6),
              const Text(
                '返回上一页后重试；如果反复出现，请重启应用。',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 12, color: Color(0xFF5A625E)),
              ),
              if (kDebugMode) ...[
                const SizedBox(height: 12),
                Text(
                  details.exceptionAsString(),
                  textAlign: TextAlign.center,
                  style: const TextStyle(
                    fontSize: 11,
                    color: Color(0xFF8A918D),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    ),
  );
}

class QingLinkApp extends ConsumerWidget {
  const QingLinkApp({super.key});

  static const _dialogShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radius.circular(AppRadius.lg)),
  );

  /// 组装应用主题：配色方案与各组件主题分别由下面的子函数构建，
  /// 本函数只负责按原有顺序把它们挂到 [ThemeData] 上。
  ///
  /// 注意：这里不能用 `AppColors.xxx`，因为 `theme` 与 `darkTheme` 会被同时
  /// 构建，全局门面只反映最后构建的那一套。统一从传入的 [palette] 取色。
  ThemeData _theme(AppPalette palette, Brightness brightness) {
    return ThemeData(
      colorScheme: _colorScheme(palette, brightness),
      useMaterial3: true,
      brightness: brightness,
      // 设计稿要求中文 Noto Sans SC、拉丁 Inter；两者都不随包发布，
      // 交给系统按回退链挑选，避免把 iOS 专属字体名写死在主题里。
      fontFamilyFallback: const ['PingFang SC', 'Noto Sans SC', 'Inter'],
      scaffoldBackgroundColor: palette.background,
      canvasColor: palette.background,
      extensions: [palette],
      splashFactory: InkRipple.splashFactory,
      cardTheme: _cardTheme(palette),
      inputDecorationTheme: _inputDecorationTheme(palette),
      dialogTheme: _dialogTheme(palette),
      bottomSheetTheme: _bottomSheetTheme(palette),
      appBarTheme: _appBarTheme(palette),
      dividerTheme: DividerThemeData(
        color: palette.line,
        thickness: 1,
        space: 1,
      ),
      navigationBarTheme: _navigationBarTheme(palette),
      listTileTheme: _listTileTheme(palette),
      iconTheme: IconThemeData(color: palette.textMuted, size: 20),
      filledButtonTheme: _filledButtonTheme(palette),
      outlinedButtonTheme: _outlinedButtonTheme(palette),
      textButtonTheme: _textButtonTheme(palette),
      splashColor: palette.brand.withValues(alpha: 0.08),
    );
  }

  /// 构建随明暗模式切换的 Material 配色方案。
  ColorScheme _colorScheme(AppPalette palette, Brightness brightness) {
    return ColorScheme(
      brightness: brightness,
      primary: palette.brand,
      onPrimary: palette.onBrand,
      secondary: palette.info,
      onSecondary: palette.onBrand,
      error: palette.danger,
      onError: Colors.white,
      surface: palette.surface,
      onSurface: palette.textPrimary,
      surfaceContainerHighest: palette.surface2,
      onSurfaceVariant: palette.textMuted,
      outline: palette.line,
      outlineVariant: palette.lineStrong,
      shadow: palette.shadow,
      scrim: palette.scrim,
      inverseSurface: palette.textPrimary,
      inversePrimary: palette.brandDeep,
    );
  }

  /// 顶栏主题：背景与页面同色，标题 18 半粗，返回键取次级文字色。
  ///
  /// 次级页（进程、设置、遥控等）统一走这套，避免每页各写一遍 AppBar。
  AppBarThemeData _appBarTheme(AppPalette palette) {
    return AppBarThemeData(
      backgroundColor: palette.background,
      surfaceTintColor: Colors.transparent,
      foregroundColor: palette.textPrimary,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: true,
      titleTextStyle: TextStyle(
        color: palette.textPrimary,
        fontSize: 17,
        fontWeight: FontWeight.w600,
      ),
      iconTheme: IconThemeData(color: palette.textMuted, size: 22),
    );
  }

  /// 卡片主题：无海拔平面卡片，使用分隔线色描边。
  CardThemeData _cardTheme(AppPalette palette) {
    return CardThemeData(
      color: palette.surface,
      surfaceTintColor: Colors.transparent,
      elevation: 0,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
        side: BorderSide(color: palette.line),
      ),
    );
  }

  /// 输入框主题：次级块填充 + `line.strong` 描边，聚焦换品牌色。
  InputDecorationTheme _inputDecorationTheme(AppPalette palette) {
    return InputDecorationTheme(
      filled: true,
      fillColor: palette.surface2,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
        borderSide: BorderSide(color: palette.lineStrong),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
        borderSide: BorderSide(color: palette.lineStrong),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
        borderSide: BorderSide(color: palette.brand),
      ),
    );
  }

  /// 对话框主题：统一圆角与标题、正文文字样式。
  DialogThemeData _dialogTheme(AppPalette palette) {
    return DialogThemeData(
      backgroundColor: palette.dialogBackground,
      shape: _dialogShape,
      titleTextStyle: TextStyle(
        color: palette.textPrimary,
        fontSize: 17,
        fontWeight: FontWeight.w600,
      ),
      contentTextStyle: TextStyle(
        color: palette.textMuted,
        fontSize: 13,
        height: 1.5,
      ),
    );
  }

  /// 底部弹层主题：背景取对话框色，顶部圆角 22，把手 36×4。
  BottomSheetThemeData _bottomSheetTheme(AppPalette palette) {
    return BottomSheetThemeData(
      backgroundColor: palette.dialogBackground,
      surfaceTintColor: Colors.transparent,
      showDragHandle: false,
      dragHandleColor: palette.lineStrong,
      dragHandleSize: const Size(36, 4),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadius.xl)),
      ),
    );
  }

  /// 底部导航栏主题：透明背景，选中项使用品牌色。
  NavigationBarThemeData _navigationBarTheme(AppPalette palette) {
    return NavigationBarThemeData(
      backgroundColor: Colors.transparent,
      surfaceTintColor: Colors.transparent,
      elevation: 0,
      height: 62,
      indicatorColor: palette.brand,
      labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
      iconTheme: WidgetStateProperty.resolveWith(
        (states) => IconThemeData(
          size: 22,
          color: states.contains(WidgetState.selected)
              ? palette.onBrand
              : palette.textSubtle,
        ),
      ),
      labelTextStyle: WidgetStateProperty.resolveWith(
        (states) => TextStyle(
          fontSize: 12,
          fontWeight: states.contains(WidgetState.selected)
              ? FontWeight.w600
              : FontWeight.w400,
          color: states.contains(WidgetState.selected)
              ? palette.onBrand
              : palette.textSubtle,
        ),
      ),
    );
  }

  /// 列表项主题：图标取次级文字色，标题取主文本色。
  ListTileThemeData _listTileTheme(AppPalette palette) {
    return ListTileThemeData(
      iconColor: palette.textMuted,
      textColor: palette.textPrimary,
      subtitleTextStyle: TextStyle(color: palette.textMuted, fontSize: 11),
      titleTextStyle: TextStyle(
        color: palette.textPrimary,
        fontSize: 13,
        fontWeight: FontWeight.w500,
      ),
    );
  }

  /// 主按钮主题：品牌色填充、高 52、圆角 14。
  FilledButtonThemeData _filledButtonTheme(AppPalette palette) {
    return FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: palette.brand,
        foregroundColor: palette.onBrand,
        minimumSize: const Size(0, AppMetrics.primaryButton),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
        ),
        textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
      ),
    );
  }

  /// 次按钮主题：描边式，文字取主文本色。
  OutlinedButtonThemeData _outlinedButtonTheme(AppPalette palette) {
    return OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: palette.textPrimary,
        side: BorderSide(color: palette.lineStrong),
        minimumSize: const Size(0, AppMetrics.minTap),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
        ),
      ),
    );
  }

  /// 文字按钮主题：品牌色文字的小号按钮。
  TextButtonThemeData _textButtonTheme(AppPalette palette) {
    return TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: palette.brand,
        textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
      ),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    return MaterialApp.router(
      localizationsDelegates: const [
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: const [Locale('zh', 'CN'), Locale('en')],
      title: '轻联 QingLink',
      debugShowCheckedModeBanner: false,
      theme: _theme(AppPalette.light, Brightness.light),
      darkTheme: _theme(AppPalette.dark, Brightness.dark),
      themeMode: themeMode,
      // 主题实际生效后才能确定亮度，这里把当前色板回填给 `AppColors` 门面，
      // 让既有页面里的 `AppColors.primary` 等写法跟随明暗切换。
      builder: (context, child) {
        AppColors.useTheme = AppPalette.of(Theme.of(context).brightness);
        return child ?? const SizedBox.shrink();
      },
      routerConfig: ref.watch(appRouterProvider),
    );
  }
}
