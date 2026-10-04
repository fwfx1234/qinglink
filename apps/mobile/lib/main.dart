import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/providers/theme_provider.dart';
import 'src/app/app_router.dart';
import 'src/theme/app_theme.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const ProviderScope(child: QingLinkApp()));
}

class QingLinkApp extends ConsumerWidget {
  const QingLinkApp({super.key});

  static const _dialogShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radius.circular(AppRadius.lg)),
  );

  /// 组装应用主题：配色方案与各组件主题分别由下面的子函数构建，
  /// 本函数只负责按原有顺序把它们挂到 [ThemeData] 上。
  ThemeData _theme(AppPalette palette, Brightness brightness) {
    return ThemeData(
      colorScheme: _colorScheme(palette, brightness),
      useMaterial3: true,
      brightness: brightness,
      fontFamily: 'PingFang SC',
      scaffoldBackgroundColor: palette.background,
      canvasColor: palette.background,
      extensions: [palette],
      splashFactory: InkRipple.splashFactory,
      cardTheme: _cardTheme(palette),
      inputDecorationTheme: _inputDecorationTheme(palette),
      dialogTheme: _dialogTheme(palette),
      bottomSheetTheme: _bottomSheetTheme(palette),
      dividerTheme: DividerThemeData(color: palette.line, thickness: 1),
      navigationBarTheme: _navigationBarTheme(palette),
      listTileTheme: _listTileTheme(palette),
      iconTheme: IconThemeData(color: palette.textMuted, size: 20),
      filledButtonTheme: _filledButtonTheme(),
      outlinedButtonTheme: _outlinedButtonTheme(palette),
      textButtonTheme: _textButtonTheme(),
      splashColor: AppColors.primary.withValues(alpha: 0.08),
    );
  }

  /// 构建随明暗模式切换的 Material 配色方案。
  ColorScheme _colorScheme(AppPalette palette, Brightness brightness) {
    return ColorScheme(
      brightness: brightness,
      primary: AppColors.primary,
      onPrimary: AppColors.onPrimary,
      secondary: AppColors.cyan,
      onSecondary: AppColors.onPrimary,
      error: AppColors.error,
      onError: Colors.white,
      surface: palette.surface,
      onSurface: palette.textPrimary,
      surfaceContainerHighest: palette.surface2,
      onSurfaceVariant: palette.textMuted,
      outline: palette.line,
      outlineVariant: palette.line.withValues(alpha: 0.5),
      shadow: palette.shadow,
      scrim: Colors.black.withValues(alpha: 0.63),
      inverseSurface: palette.textPrimary,
      inversePrimary: AppColors.primaryDark,
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

  /// 输入框主题：填充式，聚焦描边使用主色深色。
  InputDecorationTheme _inputDecorationTheme(AppPalette palette) {
    return InputDecorationTheme(
      filled: true,
      fillColor: palette.surface,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
        borderSide: BorderSide(color: palette.line),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
        borderSide: BorderSide(color: palette.line),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
        borderSide: const BorderSide(color: AppColors.primaryDark),
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
        fontSize: 18,
        fontWeight: FontWeight.w700,
      ),
      contentTextStyle: TextStyle(
        color: palette.textMuted,
        fontSize: 12,
        height: 1.5,
      ),
    );
  }

  /// 底部弹层主题：背景取对话框色，仅保留顶部圆角。
  BottomSheetThemeData _bottomSheetTheme(AppPalette palette) {
    return BottomSheetThemeData(
      backgroundColor: palette.dialogBackground,
      surfaceTintColor: Colors.transparent,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadius.xl)),
      ),
    );
  }

  /// 底部导航栏主题：透明背景，选中项使用主色高亮。
  NavigationBarThemeData _navigationBarTheme(AppPalette palette) {
    return NavigationBarThemeData(
      backgroundColor: Colors.transparent,
      surfaceTintColor: Colors.transparent,
      elevation: 0,
      height: 62,
      indicatorColor: AppColors.primary.withValues(alpha: 0.16),
      labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
      iconTheme: WidgetStateProperty.resolveWith(
        (states) => IconThemeData(
          size: 22,
          color: states.contains(WidgetState.selected)
              ? AppColors.primary
              : palette.textSubtle,
        ),
      ),
      labelTextStyle: WidgetStateProperty.resolveWith(
        (states) => TextStyle(
          fontSize: 11,
          fontWeight: states.contains(WidgetState.selected)
              ? FontWeight.w700
              : FontWeight.w500,
          color: states.contains(WidgetState.selected)
              ? AppColors.primary
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
      subtitleTextStyle: TextStyle(color: palette.textSubtle, fontSize: 12),
      titleTextStyle: TextStyle(color: palette.textPrimary, fontSize: 14),
    );
  }

  /// 主按钮主题：主色填充、最小高度 44、中号圆角。
  FilledButtonThemeData _filledButtonTheme() {
    return FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: AppColors.primary,
        foregroundColor: AppColors.onPrimary,
        minimumSize: const Size(0, 44),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
        ),
        textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w700),
      ),
    );
  }

  /// 次按钮主题：描边式，文字取主文本色。
  OutlinedButtonThemeData _outlinedButtonTheme(AppPalette palette) {
    return OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: palette.textPrimary,
        side: BorderSide(color: palette.line),
        minimumSize: const Size(0, 44),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(9),
        ),
      ),
    );
  }

  /// 文字按钮主题：主色文字的小号按钮。
  TextButtonThemeData _textButtonTheme() {
    return TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: AppColors.primary,
        textStyle: const TextStyle(fontSize: 12),
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
      routerConfig: ref.watch(appRouterProvider),
    );
  }
}
