import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 主题模式（默认跟随原型的深色主题，可切换浅色）。
class ThemeModeNotifier extends StateNotifier<ThemeMode> {
  ThemeModeNotifier() : super(ThemeMode.dark) {
    _restore();
  }

  static const String _key = 'theme_mode';

  Future<void> _restore() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final value = prefs.getString(_key);
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
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_key, isDark ? 'dark' : 'light');
    } catch (_) {
      // 忽略持久化失败。
    }
  }
}

final themeModeProvider = StateNotifierProvider<ThemeModeNotifier, ThemeMode>(
  (ref) => ThemeModeNotifier(),
);
