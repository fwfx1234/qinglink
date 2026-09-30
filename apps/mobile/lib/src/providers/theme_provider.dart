import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../core/storage/preferences_repository.dart';
import 'dependencies.dart';

/// 主题模式（默认跟随原型的深色主题，可切换浅色）。
class ThemeModeNotifier extends StateNotifier<ThemeMode> {
  ThemeModeNotifier({required this._preferences}) : super(ThemeMode.dark) {
    _restore();
  }

  static const String _key = 'theme_mode';
  final PreferencesRepository _preferences;

  Future<void> _restore() async {
    try {
      final value = await _preferences.readString(_key);
      if (value == 'light') {
        state = ThemeMode.light;
      } else if (value == 'dark') {
        state = ThemeMode.dark;
      }
    } catch (_) {
      // 忽略持久化不可用的情况，保持默认深色。
    }
  }

  bool get isDark => state == ThemeMode.dark;

  Future<void> toggle() async {
    state = isDark ? ThemeMode.light : ThemeMode.dark;
    try {
      await _preferences.writeString(
        _key,
        isDark ? 'dark' : 'light',
      );
    } catch (_) {
      // 忽略持久化失败。
    }
  }
}

final themeModeProvider = StateNotifierProvider<ThemeModeNotifier, ThemeMode>(
  (ref) => ThemeModeNotifier(
    preferences: ref.read(preferencesRepositoryProvider),
  ),
);
