import 'package:flutter_secure_storage/flutter_secure_storage.dart';

abstract interface class SecureTokenStore {
  Future<String?> read();
  Future<void> write(String token);
  Future<void> clear();
}

/// Secure storage for the Windows lock-screen credential. Kept separate from
/// the QingLink auth token so the password can never enter connection JSON.
abstract interface class LockPasswordStore {
  Future<String?> read();
  Future<void> write(String password);
  Future<void> clear();
}

class PlatformSecureTokenStore implements SecureTokenStore {
  const PlatformSecureTokenStore();

  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );
  static const _key = 'qinglink_auth_token';

  @override
  Future<String?> read() async {
    try {
      return await _storage.read(key: _key);
    } catch (_) {
      return null;
    }
  }

  @override
  Future<void> write(String token) => _storage.write(key: _key, value: token);

  @override
  Future<void> clear() => _storage.delete(key: _key);
}

class PlatformLockPasswordStore implements LockPasswordStore {
  const PlatformLockPasswordStore();

  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );
  static const _key = 'qinglink_windows_lock_password';

  @override
  Future<String?> read() async {
    try {
      return await _storage.read(key: _key);
    } catch (_) {
      return null;
    }
  }

  @override
  Future<void> write(String password) async {
    await _storage.write(key: _key, value: password);
  }

  @override
  Future<void> clear() async {
    await _storage.delete(key: _key);
  }
}
