import '../core/errors/failure.dart';
import '../core/network/http_transport.dart';
import '../protocol/qinglink_protocol.dart';

class QingLinkPairing {
  QingLinkPairing({HttpTransport? transport})
    : _transport = transport ?? DartHttpTransport();

  final HttpTransport _transport;
  Map<String, dynamic>? lastAutoConfirm;

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
    if (data.containsKey('token')) {
      lastAutoConfirm = data;
      return '';
    }
    lastAutoConfirm = null;
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
      if (decoded['ok'] == false) {
        final error = decoded['error'];
        final map = error is Map ? Map<String, dynamic>.from(error) : const {};
        throw QingLinkPairingException(
          map['code'] as String? ?? 'PAIRING_FAILED',
          map['message'] as String? ?? '配对失败',
        );
      }
      return (decoded['data'] as Map<String, dynamic>?) ?? decoded;
    } on QingLinkPairingException {
      rethrow;
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
  String toString() => '$code: $message';
}
