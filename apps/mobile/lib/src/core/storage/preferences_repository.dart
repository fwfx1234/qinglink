import 'package:shared_preferences/shared_preferences.dart';

/// Small scalar persistence boundary. Feature controllers should depend on
/// this interface instead of the plugin API.
abstract interface class PreferencesRepository {
  Future<String?> readString(String key);
  Future<void> writeString(String key, String value);
  Future<void> remove(String key);
}

final class SharedPreferencesRepository implements PreferencesRepository {
  SharedPreferencesRepository({this._preferences});

  SharedPreferences? _preferences;

  Future<SharedPreferences> _instance() async =>
      _preferences ??= await SharedPreferences.getInstance();

  @override
  Future<String?> readString(String key) async =>
      (await _instance()).getString(key);

  @override
  Future<void> writeString(String key, String value) async {
    await (await _instance()).setString(key, value);
  }

  @override
  Future<void> remove(String key) async {
    await (await _instance()).remove(key);
  }
}
