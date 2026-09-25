import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/providers/theme_provider.dart';
import 'src/screens/home_screen.dart';
import 'src/theme/app_theme.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  // Keep the phone UI in fixed portrait. A rotation-capable app makes some OEM
  // ROMs (e.g. ColorOS) flip the system "auto-rotate" quick toggle on; pinning
  // to a single orientation (PORTRAIT, not SENSOR_PORTRAIT) prevents that.
  SystemChrome.setPreferredOrientations(const [DeviceOrientation.portraitUp]);
  runApp(const ProviderScope(child: QingLinkApp()));
}

class QingLinkApp extends ConsumerWidget {
  const QingLinkApp({super.key});

  static const _dialogShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radius.circular(AppRadius.lg)),
  );

  ThemeData _theme(AppPalette palette, Brightness brightness) {
    final colorScheme = ColorScheme(
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

    return ThemeData(
      colorScheme: colorScheme,
      useMaterial3: true,
      brightness: brightness,
      fontFamily: 'PingFang SC',
      scaffoldBackgroundColor: palette.background,
      canvasColor: palette.background,
      extensions: [palette],
      splashFactory: InkRipple.splashFactory,
      cardTheme: CardThemeData(
        color: palette.surface,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
          side: BorderSide(color: palette.line),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
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
      ),
      dialogTheme: DialogThemeData(
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
      ),
      bottomSheetTheme: BottomSheetThemeData(
        backgroundColor: palette.dialogBackground,
        surfaceTintColor: Colors.transparent,
        shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadius.xl)),
        ),
      ),
      dividerTheme: DividerThemeData(color: palette.line, thickness: 1),
      navigationBarTheme: NavigationBarThemeData(
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
      ),
      listTileTheme: ListTileThemeData(
        iconColor: palette.textMuted,
        textColor: palette.textPrimary,
        subtitleTextStyle: TextStyle(color: palette.textSubtle, fontSize: 12),
        titleTextStyle: TextStyle(color: palette.textPrimary, fontSize: 14),
      ),
      iconTheme: IconThemeData(color: palette.textMuted, size: 20),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: AppColors.primary,
          foregroundColor: AppColors.onPrimary,
          minimumSize: const Size(0, 44),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.md),
          ),
          textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w700),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: palette.textPrimary,
          side: BorderSide(color: palette.line),
          minimumSize: const Size(0, 44),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(9),
          ),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          foregroundColor: AppColors.primary,
          textStyle: const TextStyle(fontSize: 12),
        ),
      ),
      splashColor: AppColors.primary.withValues(alpha: 0.08),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    return MaterialApp(
      title: '轻联 QingLink',
      debugShowCheckedModeBanner: false,
      theme: _theme(AppPalette.light, Brightness.light),
      darkTheme: _theme(AppPalette.dark, Brightness.dark),
      themeMode: themeMode,
      home: const HomeScreen(),
    );
  }
}
