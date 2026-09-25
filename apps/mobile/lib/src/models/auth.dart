class DiscoveredPc {
  final String id;
  final String name;
  final String host;
  final int port;
  final int version;

  const DiscoveredPc({
    required this.id,
    required this.name,
    required this.host,
    required this.port,
    required this.version,
  });

  String get baseUrl {
    final formattedHost = host.contains(':') ? '[$host]' : host;
    return 'http://$formattedHost:$port';
  }
}

/// WS 连接状态
enum AppConnectionState {
  /// 未连接
  disconnected,

  /// 正在连接
  connecting,

  /// 连接已建立
  connected,
}

/// 认证状态
enum AuthStatus {
  /// 无 token
  unauthenticated,

  /// 认证中（验证 token 或配对）
  authenticating,

  /// 已认证
  authenticated,
}

/// 认证状态数据
class AuthState {
  final AppConnectionState connection;
  final AuthStatus status;
  final String? token;
  final String? errorMessage;
  final bool pairingRequested;
  final List<DiscoveredPc> discoveredDevices;

  const AuthState({
    this.connection = AppConnectionState.disconnected,
    this.status = AuthStatus.unauthenticated,
    this.token,
    this.errorMessage,
    this.pairingRequested = false,
    this.discoveredDevices = const [],
  });

  AuthState copyWith({
    AppConnectionState? connection,
    AuthStatus? status,
    String? token,
    String? errorMessage,
    bool? pairingRequested,
    bool clearError = false,
    bool clearToken = false,
    List<DiscoveredPc>? discoveredDevices,
  }) {
    return AuthState(
      connection: connection ?? this.connection,
      status: status ?? this.status,
      token: clearToken ? null : (token ?? this.token),
      errorMessage: clearError ? null : (errorMessage ?? this.errorMessage),
      pairingRequested: pairingRequested ?? this.pairingRequested,
      discoveredDevices: discoveredDevices ?? this.discoveredDevices,
    );
  }

  bool get isAuthenticated => status == AuthStatus.authenticated;
  bool get isAuthenticating => status == AuthStatus.authenticating;
  bool get isConnected => connection == AppConnectionState.connected;
  bool get isConnecting => connection == AppConnectionState.connecting;
}
