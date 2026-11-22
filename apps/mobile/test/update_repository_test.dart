import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:qinglink_mobile/src/data/repositories/update_repository.dart';

/// 用固定 release 列表构造 GitHub API 响应体。
String _releasesJson(List<Map<String, dynamic>> assets) => jsonEncode([
      {
        'tag_name': 'mobile-v1.0.0+9',
        'html_url': 'https://example.test/release',
        'body': 'notes',
        'draft': false,
        'prerelease': false,
        'assets': assets,
      }
    ]);

Map<String, dynamic> _asset(String name) => {
      'name': name,
      'browser_download_url': 'https://example.test/$name',
    };

void main() {
  group('UpdateRepository.check', () {
    test('需要 APK 与 SHA256SUMS 同时存在', () async {
      final client = MockClient((request) async => http.Response(
          _releasesJson([
            _asset('QingLink-mobile-1.0.0+9-android.apk'),
            _asset('SHA256SUMS')
          ]),
          200));
      final repo = UpdateRepository(client: client);
      final info = await repo.check('1.0.0+2');
      expect(info, isNotNull);
      expect(info!.hasApk, isTrue);
      expect(info.checksumUrl, 'https://example.test/SHA256SUMS');
    });

    test('缺少 SHA256SUMS 时跳过 release', () async {
      final client = MockClient((request) async => http.Response(
          _releasesJson([_asset('QingLink-mobile-1.0.0+9-android.apk')]),
          200));
      final repo = UpdateRepository(client: client);
      expect(await repo.check('1.0.0+2'), isNull);
    });
  });

  group('UpdateRepository.fetchChecksums', () {
    test('解析标准清单并小写化文件名', () async {
      final hash = 'a' * 64;
      final client = MockClient((request) async =>
          http.Response('$hash  QingLink-mobile-1.0.0+9-android.apk\n', 200));
      final repo = UpdateRepository(client: client);
      final map = await repo.fetchChecksums('https://example.test/SHA256SUMS');
      expect(map['qinglink-mobile-1.0.0+9-android.apk'], hash);
    });

    test('容忍二进制模式的星号分隔符', () async {
      final hash = 'c' * 64;
      final client = MockClient((request) async =>
          http.Response('$hash *QingLink-mobile-1.0.0+9-android.apk', 200));
      final repo = UpdateRepository(client: client);
      final map = await repo.fetchChecksums('https://example.test/SHA256SUMS');
      expect(map['qinglink-mobile-1.0.0+9-android.apk'], hash);
    });

    test('拒绝无效哈希', () async {
      final client = MockClient(
          (request) async => http.Response('short  QingLink.exe', 200));
      final repo = UpdateRepository(client: client);
      expect(repo.fetchChecksums('https://example.test/SHA256SUMS'),
          throwsA(anything));
    });

    test('拒绝重复文件名', () async {
      final hash = 'b' * 64;
      final client = MockClient((request) async =>
          http.Response('$hash  a.apk\n$hash  A.APK', 200));
      final repo = UpdateRepository(client: client);
      expect(repo.fetchChecksums('https://example.test/SHA256SUMS'),
          throwsA(anything));
    });
  });
}
