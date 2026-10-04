import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/core/errors/failure.dart';
import 'package:qinglink_mobile/src/core/network/http_transport.dart';
import 'package:qinglink_mobile/src/services/qinglink_pairing.dart';

void main() {
  // No `TestWidgetsFlutterBinding` here: it installs HTTP overrides that turn
  // every real request into a 400, which would mask the envelope handling.
  test('keeps the server error code from a non-2xx envelope', () async {
    final server = await _respond(
      HttpStatus.locked,
      '{"ok":false,"error":{"code":"PAIRING_REQUIRES_DESKTOP",'
      '"message":"首次配对需要在已登录桌面确认"}}',
    );
    addTearDown(() => server.close(force: true));

    await expectLater(
      QingLinkPairing().requestAt(
        'http://127.0.0.1:${server.port}',
        deviceId: 'mobile-1',
        deviceName: '轻联手机',
      ),
      throwsA(
        isA<QingLinkPairingException>()
            .having((error) => error.code, 'code', 'PAIRING_REQUIRES_DESKTOP')
            .having((error) => error.message, 'message', contains('桌面')),
      ),
    );
  });

  test('confirm keeps the code and never renders it to the user', () async {
    final server = await _respond(
      HttpStatus.badRequest,
      '{"ok":false,"error":{"code":"INVALID_PIN","message":"pairing code is invalid"}}',
    );
    addTearDown(() => server.close(force: true));

    try {
      await QingLinkPairing().confirmAt(
        'http://127.0.0.1:${server.port}',
        requestId: '123e4567-e89b-12d3-a456-426614174000',
        pin: '000000',
        deviceId: 'mobile-1',
        deviceName: '轻联手机',
      );
      fail('expected a pairing exception');
    } on QingLinkPairingException catch (error) {
      expect(error.code, 'INVALID_PIN');
      expect(error.toString(), isNot(contains('INVALID_PIN')));
    }
  });

  test('falls back to a network failure when the body is not an envelope',
      () async {
    final server = await _respond(
      HttpStatus.badRequest,
      'Failed to deserialize the JSON body',
      contentType: 'text/plain',
    );
    addTearDown(() => server.close(force: true));

    final transport = DartHttpTransport();
    addTearDown(transport.close);

    await expectLater(
      transport.postJson(
        Uri.parse('http://127.0.0.1:${server.port}/api/v2/pairing/requests'),
        const {},
      ),
      throwsA(isA<NetworkFailure>()),
    );
  });

  test('treats a 2xx envelope as a normal payload', () async {
    final server = await _respond(
      HttpStatus.ok,
      '{"ok":true,"data":{"request_id":"abc","expires_at_unix":1}}',
    );
    addTearDown(() => server.close(force: true));

    final response = await QingLinkPairing().requestAt(
      'http://127.0.0.1:${server.port}',
      deviceId: 'mobile-1',
      deviceName: '轻联手机',
    );

    expect(response, 'abc');
  });
}

Future<HttpServer> _respond(
  int status,
  String body, {
  String contentType = 'application/json',
}) async {
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  server.listen((request) {
    unawaited(() async {
      await request.drain<void>();
      request.response
        ..statusCode = status
        ..headers.contentType = ContentType.parse(contentType)
        ..add(utf8.encode(body));
      await request.response.close();
    }());
  });
  return server;
}
