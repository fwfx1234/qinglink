import 'dart:convert';
import 'dart:io';

import '../protocol/qinglink_protocol.dart';

class QingLinkPairing {
  Map<String, dynamic>? lastAutoConfirm;

  Future<void> cancelAt(String baseUrl, String requestId) async {
    final client = HttpClient();
    try {
      final request = await client.deleteUrl(
        Uri.parse('$baseUrl/api/v2/pairing/requests/$requestId'),
      );
      final response = await request.close();
      await response.drain<void>();
      if (response.statusCode != HttpStatus.ok &&
          response.statusCode != HttpStatus.notFound) {
        throw const QingLinkPairingException('PAIRING_CANCEL_FAILED', '无法取消配对');
      }
    } finally {
      client.close(force: true);
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
    final client = HttpClient();
    try {
      final request = await client.postUrl(Uri.parse(url));
      request.headers.contentType = ContentType.json;
      request.write(jsonEncode(body));
      final response = await request.close();
      final decoded =
          jsonDecode(await response.transform(utf8.decoder).join())
              as Map<String, dynamic>;
      if (response.statusCode < 200 ||
          response.statusCode >= 300 ||
          decoded['ok'] == false) {
        final error = decoded['error'] as Map<String, dynamic>?;
        throw QingLinkPairingException(
          error?['code'] as String? ?? 'PAIRING_FAILED',
          error?['message'] as String? ?? '配对失败',
        );
      }
      return (decoded['data'] as Map<String, dynamic>?) ?? decoded;
    } finally {
      client.close(force: true);
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
