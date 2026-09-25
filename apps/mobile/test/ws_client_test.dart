import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/services/remote_session.dart';
import 'package:qinglink_mobile/src/services/ws_client.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('reconnects once when a socket closes', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    var connectionCount = 0;
    server.listen((request) {
      unawaited(() async {
        if (!WebSocketTransformer.isUpgradeRequest(request)) {
          request.response
            ..statusCode = HttpStatus.notFound
            ..close();
          return;
        }
        final socket = await WebSocketTransformer.upgrade(request);
        connectionCount++;
        if (connectionCount == 1) {
          await Future<void>.delayed(const Duration(milliseconds: 30));
          await socket.close();
        } else {
          socket.listen((message) {
            final data = jsonDecode(message as String) as Map<String, dynamic>;
            if (data['type'] == 'ping') {
              socket.add(
                jsonEncode({'type': 'pong', 'request_id': data['request_id']}),
              );
            }
          });
        }
      }());
    });

    final client = WsClient(
      baseUrl: 'http://127.0.0.1:${server.port}',
      token: '',
    );
    final states = <WsConnectionState>[];
    final subscription = client.stateStream.listen(states.add);
    addTearDown(() async {
      await subscription.cancel();
      client.dispose();
      await server.close(force: true);
    });

    unawaited(client.connect());
    await _waitUntil(() => connectionCount >= 2);
    await Future<void>.delayed(const Duration(milliseconds: 250));

    expect(connectionCount, 2);
    expect(
      states.where((state) => state == WsConnectionState.connected),
      hasLength(2),
    );
  });

  test('marks a silent socket unhealthy', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    server.listen((request) {
      unawaited(() async {
        if (WebSocketTransformer.isUpgradeRequest(request)) {
          final socket = await WebSocketTransformer.upgrade(request);
          socket.listen((_) {});
        }
      }());
    });

    final client = WsClient(
      baseUrl: 'http://127.0.0.1:${server.port}',
      token: '',
      healthInterval: const Duration(milliseconds: 30),
      healthTimeout: const Duration(milliseconds: 30),
    );
    final states = <WsConnectionState>[];
    final subscription = client.stateStream.listen(states.add);
    addTearDown(() async {
      await subscription.cancel();
      client.dispose();
      await server.close(force: true);
    });

    unawaited(client.connect());
    await _waitUntil(
      () => states.contains(WsConnectionState.reconnecting),
      timeout: const Duration(seconds: 2),
    );

    expect(states, contains(WsConnectionState.connected));
    expect(states, contains(WsConnectionState.disconnected));
  });

  test('sends binary frames over a separate input channel', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final inputBinaryFrames = <List<int>>[];
    server.listen((request) {
      unawaited(() async {
        final socket = await WebSocketTransformer.upgrade(request);
        final isInput = request.uri.path == '/api/v2/input';
        socket.listen((message) {
          if (isInput) {
            if (message is List<int>) {
              inputBinaryFrames.add(message);
            } else {
              final data =
                  jsonDecode(message as String) as Map<String, dynamic>;
              if (data['type'] == 'authenticate') {
                socket.add(
                  jsonEncode({
                    'type': 'authenticated',
                    'data': {'token': data['token'], 'session_id': 'input-1'},
                    'request_id': data['request_id'],
                  }),
                );
              }
            }
          } else {
            final data = jsonDecode(message as String) as Map<String, dynamic>;
            final requestId = data['request_id'];
            if (data['type'] == 'authenticate') {
              socket.add(
                jsonEncode({
                  'type': 'authenticated',
                  'data': {'token': data['token'], 'session_id': 'session-1'},
                  'request_id': requestId,
                }),
              );
            } else {
              socket.add(
                jsonEncode({'type': 'success', 'request_id': requestId}),
              );
            }
          }
        });
      }());
    });

    final session = RemoteSession();
    addTearDown(() async {
      session.dispose();
      await server.close(force: true);
    });

    session.configure('http://127.0.0.1:${server.port}', 'token-1');
    expect(await session.connect(), isTrue);
    expect(session.state, RemoteSessionState.ready);

    final frame = <int>[1, 1, 1, 0, 5, 0, 253, 255, 0, 0, 0, 0];
    expect(await session.sendBinaryFrame(frame), isTrue);
    await _waitUntil(() => inputBinaryFrames.isNotEmpty);
    expect(inputBinaryFrames.single, frame);
  });

  test('authenticates and sends business requests on one socket', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    var connectionCount = 0;
    server.listen((request) {
      unawaited(() async {
        final socket = await WebSocketTransformer.upgrade(request);
        connectionCount++;
        socket.listen((message) {
          final data = jsonDecode(message as String) as Map<String, dynamic>;
          final requestId = data['request_id'];
          if (data['type'] == 'authenticate') {
            socket.add(
              jsonEncode({
                'type': 'authenticated',
                'data': {'token': data['token'], 'session_id': 'session-1'},
                'request_id': requestId,
              }),
            );
          } else {
            socket.add(
              jsonEncode({'type': 'success', 'request_id': requestId}),
            );
          }
        });
      }());
    });

    final session = RemoteSession();
    addTearDown(() async {
      session.dispose();
      await server.close(force: true);
    });

    session.configure('http://127.0.0.1:${server.port}', 'token-1');
    final results = await Future.wait([
      session.connect(),
      session.connect(),
      session.connect(),
    ]);
    expect(results, everyElement(isTrue));
    expect(session.state, RemoteSessionState.ready);
    final response = await session.request({'type': 'get_system_status'});

    expect(response['type'], 'success');
    expect(connectionCount, 1);
  });
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
