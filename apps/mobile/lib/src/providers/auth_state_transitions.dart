import '../models/auth.dart';

/// Restores the authenticated state after a transport reconnects and the
/// server has accepted the saved token.
AuthState restoreAuthenticatedStateAfterReconnect(
  AuthState current,
  String? savedToken,
) {
  final token = current.token ?? savedToken;
  if (token == null || token.isEmpty) {
    return current.copyWith(connection: AppConnectionState.connected);
  }
  return AuthState(
    connection: AppConnectionState.connected,
    status: AuthStatus.authenticated,
    token: token,
    errorMessage: null,
  );
}
