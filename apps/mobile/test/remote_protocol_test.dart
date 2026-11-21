import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/protocol/remote_protocol.dart';

void main() {
  test('typed authentication requests preserve the v2 wire shape', () {
    expect(
      AuthenticateTokenRequest('secret').toJson(),
      {'type': 'authenticate', 'method': 'token', 'token': 'secret'},
    );
    expect(
      AuthenticatePairingRequest('123456', deviceName: '手机').toJson(),
      {
        'type': 'authenticate',
        'method': 'pair',
        'pin': '123456',
        'device_name': '手机',
      },
    );
  });

  test('typed response exposes structured errors', () {
    final response = RemoteResponse.fromJson({
      'type': 'error',
      'request_id': 'r1',
      'data': {'code': 'NOT_READY', 'message': '尚未准备好'},
    });

    expect(response.isError, isTrue);
    expect(response.requestId, 'r1');
    expect(response.error?.code, 'NOT_READY');
  });
}
