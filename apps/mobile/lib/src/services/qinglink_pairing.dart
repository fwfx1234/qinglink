import '../core/errors/failure.dart';
import '../core/network/http_transport.dart';
import '../protocol/qinglink_protocol.dart';

class QingLinkPairing {
  QingLinkPairing({HttpTransport? transport})
    : _transport = transport ?? DartHttpTransport();

  final HttpTransport _transport;

  Future<void> cancelAt(String baseUrl, String requestId) async {
    try {
      await _transport.delete(
        Uri.parse('$baseUrl/api/v2/pairing/requests/$requestId'),
      );
    } on NetworkFailure catch (error) {
      throw QingLinkPairingException('PAIRING_CANCEL_FAILED', error.message);
    }
  }

  Future<String> requestAt(
    String baseUrl, {
    required String deviceId,
    required String deviceName,
  }) async {
    final data = await _post(
      '$baseUrl/api/v2/pairing/requests',
      PairingRequestDto(deviceId: deviceId, deviceName: deviceName).toJson(),
    );
    // A request never authorizes a device.  The token is returned only by a
    // separate PIN confirmation and is intentionally not accepted here.
    final response = PairingRequestResponseDto.fromJson(data);
    if (response.requestId.isEmpty) {
      throw const QingLinkPairingException('PAIRING_FAILED', 'PC 未返回配对请求标识');
    }
    return response.requestId;
  }

  Future<Map<String, dynamic>> confirmAt(
    String baseUrl, {
    required String requestId,
    required String pin,
    required String deviceId,
    required String deviceName,
  }) {
    return _post(
      '$baseUrl/api/v2/pairing/confirm',
      PairingConfirmDto(
        requestId: requestId,
        pin: pin,
        deviceId: deviceId,
        deviceName: deviceName,
      ).toJson(),
    );
  }

  Future<Map<String, dynamic>> _post(
    String url,
    Map<String, dynamic> body,
  ) async {
    try {
      final decoded = await _transport.postJson(Uri.parse(url), body);
      // HTTP 契约用 {"ok":false,"error":{code,message}}，而传输层已对非 2xx
      // 抛 ServerFailure / ProtocolFailure，因此这里拿到的恒为成功信封。
      // 注意：WebSocket 契约不是这个形状（错误帧是 {"type":"error","data":{...}}），
      // 不要把两边的字段名互相套用。
      return (decoded['data'] as Map<String, dynamic>?) ?? decoded;
    } on QingLinkPairingException {
      rethrow;
    } on ServerFailure catch (error) {
      // The transport already decoded the server error envelope, so keep the
      // stable protocol code instead of degrading it to a network failure.
      throw QingLinkPairingException(error.code, error.message);
    } on QingLinkFailure catch (error) {
      throw QingLinkPairingException('PAIRING_NETWORK_ERROR', error.message);
    }
  }
}

class QingLinkPairingException implements Exception {
  const QingLinkPairingException(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => message;
}
