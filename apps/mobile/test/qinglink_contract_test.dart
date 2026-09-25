import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/protocol/qinglink_protocol.dart';

void main() {
  test('pairing fixtures match the Flutter wire shape', () async {
    final request = await _fixture('pairing-request.json');
    expect(request, {'device_id': 'mobile-fixture', 'device_name': '轻联手机'});

    final confirm = await _fixture('pairing-confirm.json');
    expect(confirm['request_id'], '123e4567-e89b-12d3-a456-426614174000');
    expect(confirm['pin'], '123456');
    expect(confirm['device_id'], request['device_id']);

    final token = await _fixture('pairing-token.json');
    expect(token['pc_id'], 'pc-fixture');
    expect(token['mac_address'], 'AA:BB:CC:DD:EE:FF');
  });

  test('Dart DTOs round-trip the shared pairing contract', () async {
    final request = PairingRequestDto(
      deviceId: 'mobile-fixture',
      deviceName: '轻联手机',
    );
    expect(request.toJson(), await _fixture('pairing-request.json'));

    final confirm = PairingConfirmDto(
      requestId: '123e4567-e89b-12d3-a456-426614174000',
      pin: '123456',
      deviceId: 'mobile-fixture',
      deviceName: '轻联手机',
    );
    expect(confirm.toJson(), await _fixture('pairing-confirm.json'));

    final token = PairingTokenResponseDto.fromJson(
      await _fixture('pairing-token.json'),
    );
    expect(token.pcId, 'pc-fixture');
    expect(token.macAddress, 'AA:BB:CC:DD:EE:FF');
  });
}

Future<Map<String, dynamic>> _fixture(String name) async {
  final file = File('../../protocol/fixtures/$name');
  return jsonDecode(await file.readAsString()) as Map<String, dynamic>;
}
