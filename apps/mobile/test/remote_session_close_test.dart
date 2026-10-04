import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/services/remote_session.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('pause keeps the token and schedules a reconnect', () async {
    final server = await _closingServer('SERVICE_PAUSED');
    final session = RemoteSession();
    final failures = <RemoteSessionAuthFailure>[];
    final subscription = session.failureStream.listen(failures.add);
    addTearDown(() async {
      await subscription.cancel();
      session.dispose();
      await server.close(force: true);
    });

    session.configure('http://127.0.0.1:${server.port}', 'token-1');
    expect(await session.connect(), isTrue);

    await _waitUntil(() => failures.isNotEmpty);
    await _waitUntil(() => session.isReconnectScheduled);

    expect(failures.first.code, 'SERVICE_PAUSED');
    expect(session.authenticatedToken, isNull);
    expect(
      session.state,
      anyOf(
        RemoteSessionState.disconnected,
        RemoteSessionState.connecting,
        RemoteSessionState.authenticating,
      ),
    );
  });

  test('a replaced session stops reconnecting', () async {
    final server = await _closingServer('REPLACED');
    final session = RemoteSession();
    final failures = <RemoteSessionAuthFailure>[];
    final subscription = session.failureStream.listen(failures.add);
    addTearDown(() async {
      await subscription.cancel();
      session.dispose();
      await server.close(force: true);
    });

    session.configure('http://127.0.0.1:${server.port}', 'token-1');
    expect(await session.connect(), isTrue);

    await _waitUntil(() => failures.isNotEmpty);
    await Future<void>.delayed(const Duration(milliseconds: 50));

    expect(failures.first.code, 'REPLACED');
    expect(session.isReconnectScheduled, isFalse);
    expect(session.state, RemoteSessionState.disconnected);
  });
}

Future<HttpServer> _closingServer(String code) async {
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  server.listen((request) {
    unawaited(() async {
      if (!WebSocketTransformer.isUpgradeRequest(request)) {
        request.response.statusCode = HttpStatus.notFound;
        await request.response.close();
        return;
      }
      final socket = await WebSocketTransformer.upgrade(request);
      socket.listen((message) {
        final data = jsonDecode(message as String) as Map<String, dynamic>;
        if (data['type'] != 'authenticate') return;
        socket.add(
          jsonEncode({
            'type': 'authenticated',
            'data': {'token': data['token'], 'session_id': 'session-1'},
            'request_id': data['request_id'],
          }),
        );
        unawaited(
          Future<void>.delayed(const Duration(milliseconds: 20), () async {
            socket.add(jsonEncode({'type': 'session_closed', 'data': {'code': code}}));
            await socket.close();
          }),
        );
      });
    }());
  });
  return server;
}

Future<void> _waitUntil(
  bool Function() predicate, {
  Duration timeout = const Duration(seconds: 2),
}) async {
  final deadline = DateTime.now().add(timeout);
  while (!predicate() && DateTime.now().isBefore(deadline)) {
    await Future<void>.delayed(const Duration(milliseconds: 10));
  }
  expect(predicate(), isTrue);
}
