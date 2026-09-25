import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/models/auth.dart';
import 'package:qinglink_mobile/src/providers/auth_provider.dart';
import 'package:qinglink_mobile/src/providers/settings_provider.dart';
import 'package:qinglink_mobile/src/services/secure_token_store.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  test('ConnectionConfig.copyWith can clear a saved token', () {
    const config = ConnectionConfig(
      pcIpAddress: '192.168.1.8',
      savedToken: 'revoked-token',
      isPaired: true,
    );

    final cleared = config.copyWith(savedToken: null, isPaired: false);

    expect(cleared.savedToken, isNull);
    expect(cleared.isPaired, isFalse);
    expect(cleared.baseUrl, config.baseUrl);
  });

  test('restores authentication after a saved-token reconnect', () {
    const current = AuthState(
      connection: AppConnectionState.disconnected,
      status: AuthStatus.authenticating,
      token: 'saved-token',
      errorMessage: 'PC 暂时无法连接',
    );

    final restored = restoreAuthenticatedStateAfterReconnect(
      current,
      'saved-token',
    );

    expect(restored.connection, AppConnectionState.connected);
    expect(restored.status, AuthStatus.authenticated);
    expect(restored.token, 'saved-token');
    expect(restored.errorMessage, isNull);
  });

  test('does not invent authentication without a token', () {
    const current = AuthState(
      connection: AppConnectionState.connecting,
      status: AuthStatus.unauthenticated,
    );

    final restored = restoreAuthenticatedStateAfterReconnect(current, null);

    expect(restored.connection, AppConnectionState.connected);
    expect(restored.status, AuthStatus.unauthenticated);
  });

  test(
    'loads a clamped global mouse sensitivity and saves later changes',
    () async {
      SharedPreferences.setMockInitialValues({'mouse_sensitivity': 12.0});
      final settings = SettingsNotifier(tokenStore: _MemoryTokenStore());
      addTearDown(settings.dispose);

      await settings.loaded;
      expect(settings.state.mouseSensitivity, 8);

      await settings.saveMouseSensitivity(0.25);
      expect(settings.state.mouseSensitivity, 1);

      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getDouble('mouse_sensitivity'), 1);
    },
  );

  test('uses the default mouse sensitivity for existing settings', () async {
    SharedPreferences.setMockInitialValues({});
    final settings = SettingsNotifier(tokenStore: _MemoryTokenStore());
    addTearDown(settings.dispose);

    await settings.loaded;

    expect(
      settings.state.mouseSensitivity,
      ConnectionConfig.defaultMouseSensitivity,
    );
  });
}

class _MemoryTokenStore implements SecureTokenStore {
  String? token;

  @override
  Future<void> clear() async => token = null;

  @override
  Future<String?> read() async => token;

  @override
  Future<void> write(String value) async => token = value;
}
