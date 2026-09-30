import 'dart:async';
import 'dart:convert';
import 'dart:io';

import '../errors/failure.dart';

/// Shared JSON transport used by all HTTP repositories.
abstract interface class HttpTransport {
  Future<Map<String, dynamic>> postJson(
    Uri uri,
    Map<String, dynamic> body, {
    Duration timeout = const Duration(seconds: 10),
  });

  Future<void> delete(
    Uri uri, {
    Duration timeout = const Duration(seconds: 10),
  });
}

/// `dart:io` implementation kept behind [HttpTransport] for testability.
final class DartHttpTransport implements HttpTransport {
  DartHttpTransport({HttpClient? client}) : _client = client ?? HttpClient();

  final HttpClient _client;
  bool _closed = false;

  @override
  Future<Map<String, dynamic>> postJson(
    Uri uri,
    Map<String, dynamic> body, {
    Duration timeout = const Duration(seconds: 10),
  }) async {
    _ensureOpen();
    try {
      final request = await _client.postUrl(uri).timeout(timeout);
      request.headers.contentType = ContentType.json;
      request.write(jsonEncode(body));
      final response = await request.close().timeout(timeout);
      final raw = await response.transform(utf8.decoder).join().timeout(timeout);
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw NetworkFailure('HTTP 请求失败：${response.statusCode} $uri');
      }
      final decoded = jsonDecode(raw);
      if (decoded is! Map) {
        throw const ProtocolFailure('服务端返回了无效 JSON');
      }
      return Map<String, dynamic>.from(decoded);
    } on QingLinkFailure {
      rethrow;
    } on TimeoutException catch (error, stackTrace) {
      throw NetworkFailure('请求超时：$uri', cause: error, stackTrace: stackTrace);
    } on SocketException catch (error, stackTrace) {
      throw NetworkFailure('无法连接到 PC：$uri', cause: error, stackTrace: stackTrace);
    } on FormatException catch (error, stackTrace) {
      throw ProtocolFailure('服务端返回了无法解析的数据', cause: error, stackTrace: stackTrace);
    } catch (error, stackTrace) {
      throw NetworkFailure('HTTP 请求失败：$uri', cause: error, stackTrace: stackTrace);
    }
  }

  @override
  Future<void> delete(
    Uri uri, {
    Duration timeout = const Duration(seconds: 10),
  }) async {
    _ensureOpen();
    try {
      final request = await _client.deleteUrl(uri).timeout(timeout);
      final response = await request.close().timeout(timeout);
      await response.drain<void>().timeout(timeout);
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw NetworkFailure('删除请求失败：${response.statusCode}');
      }
    } on QingLinkFailure {
      rethrow;
    } on TimeoutException catch (error, stackTrace) {
      throw NetworkFailure('请求超时：$uri', cause: error, stackTrace: stackTrace);
    } on SocketException catch (error, stackTrace) {
      throw NetworkFailure('无法连接到 PC：$uri', cause: error, stackTrace: stackTrace);
    } catch (error, stackTrace) {
      throw NetworkFailure('HTTP 请求失败：$uri', cause: error, stackTrace: stackTrace);
    }
  }

  void _ensureOpen() {
    if (_closed) throw StateError('HTTP transport has been closed');
  }

  void close() {
    if (_closed) return;
    _closed = true;
    _client.close(force: true);
  }
}
