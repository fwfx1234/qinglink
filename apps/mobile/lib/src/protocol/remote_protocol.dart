import '../core/errors/failure.dart';

/// Typed envelope for a request sent over the QingLink WebSocket protocol.
sealed class RemoteRequest {
  const RemoteRequest(this.type, {this.requestId});

  final String type;
  final String? requestId;

  Map<String, dynamic> toJson() => {
    'type': type,
    if (requestId != null) 'request_id': requestId,
  };
}

final class RemoteCommandRequest extends RemoteRequest {
  const RemoteCommandRequest(
    super.type, {
    this.data = const <String, dynamic>{},
    super.requestId,
  });

  final Map<String, dynamic> data;

  @override
  Map<String, dynamic> toJson() => {...super.toJson(), ...data};
}

final class AuthenticateTokenRequest extends RemoteRequest {
  const AuthenticateTokenRequest(this.token, {super.requestId}) : super('authenticate');

  final String token;

  @override
  Map<String, dynamic> toJson() => {
    ...super.toJson(),
    'method': 'token',
    'token': token,
  };
}

final class AuthenticatePairingRequest extends RemoteRequest {
  const AuthenticatePairingRequest(this.pin, {this.deviceName, super.requestId})
    : super('authenticate');

  final String pin;
  final String? deviceName;

  @override
  Map<String, dynamic> toJson() => {
    ...super.toJson(),
    'method': 'pair',
    'pin': pin,
    if (deviceName != null && deviceName!.isNotEmpty) 'device_name': deviceName,
  };
}

/// Typed WebSocket response. Raw JSON is confined to this protocol boundary.
final class RemoteResponse {
  const RemoteResponse({
    required this.type,
    required this.requestId,
    this.data,
    this.error,
  });

  final String type;
  final String? requestId;
  final Map<String, dynamic>? data;
  final RemoteError? error;

  bool get isError => type == 'error' || error != null;

  factory RemoteResponse.fromJson(Map<String, dynamic> json) {
    final type = json['type'] as String? ?? 'unknown';
    // WS 错误帧的线格式是 {"type":"error","data":{code,message}}：信封把
    // code/message 放在 `data` 里，**没有**顶层 `error` 键
    // （服务端 responses_ws.rs 的 #[serde(tag = "type", content = "data")] 决定）。
    // 只有 `type == 'error'` 时 `data` 才是错误载荷；成功帧的 `data` 是正常结果，
    // 不能无条件当错误读——`RemoteError.fromJson` 对任何输入都返回非空对象，
    // 那样会让每一次成功调用都抛异常。
    final rawData = json['data'];
    final isErrorFrame = type == 'error';
    final rawError = isErrorFrame ? rawData : json['error'];
    return RemoteResponse(
      type: type,
      requestId: json['request_id'] as String?,
      data: isErrorFrame || rawData is! Map
          ? null
          : Map<String, dynamic>.from(rawData),
      error: rawError is Map
          ? RemoteError.fromJson(Map<String, dynamic>.from(rawError))
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'type': type,
    if (requestId != null) 'request_id': requestId,
    if (data != null) 'data': data,
    if (error != null) 'error': error!.toJson(),
  };
}

final class RemoteError {
  const RemoteError({required this.code, required this.message});

  final String code;
  final String message;

  factory RemoteError.fromJson(Map<String, dynamic> json) => RemoteError(
    code: json['code'] as String? ?? 'REMOTE_COMMAND_FAILED',
    message: json['message'] as String? ?? 'PC 未能完成该操作',
  );

  Map<String, dynamic> toJson() => {'code': code, 'message': message};
}

/// Converts a typed response into the domain error used by controllers.
QingLinkFailure failureFromResponse(RemoteResponse response) {
  final error = response.error;
  if (error == null) return const ProtocolFailure('服务端返回了错误响应');
  return ProtocolFailure('${error.code}: ${error.message}');
}
