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
    const apkName = 'QingLink-mobile-1.0.0+9-android.apk';

    test('识别统一的 SHA256SUMS 清单', () async {
      final client = MockClient((request) async => http.Response(
          _releasesJson([_asset(apkName), _asset('SHA256SUMS')]), 200));
      final repo = UpdateRepository(client: client);
      final info = await repo.check('1.0.0+2');
      expect(info, isNotNull);
      expect(info!.hasApk, isTrue);
      expect(info.checksumUrl, 'https://example.test/SHA256SUMS');
    });

    test('识别与 APK 同名的 .sha256 清单（旧客户端兼容命名）', () async {
      final client = MockClient((request) async => http.Response(
          _releasesJson([_asset(apkName), _asset('$apkName.sha256')]), 200));
      final repo = UpdateRepository(client: client);
      final info = await repo.check('1.0.0+2');
      expect(info, isNotNull);
      expect(info!.hasApk, isTrue);
      expect(info.checksumUrl, 'https://example.test/$apkName.sha256');
    });

    test('两种清单同时存在时优先 SHA256SUMS', () async {
      final client = MockClient((request) async => http.Response(
          _releasesJson([
            _asset(apkName),
            _asset('$apkName.sha256'),
            _asset('SHA256SUMS'),
          ]),
          200));
      final repo = UpdateRepository(client: client);
      final info = await repo.check('1.0.0+2');
      expect(info!.checksumUrl, 'https://example.test/SHA256SUMS');
    });

    test('没有清单时仍报出新版本，但标记为不可应用内安装', () async {
      final client = MockClient((request) async =>
          http.Response(_releasesJson([_asset(apkName)]), 200));
      final repo = UpdateRepository(client: client);
      final info = await repo.check('1.0.0+2');
      expect(info, isNotNull);
      expect(info!.version, '1.0.0+9');
      expect(info.hasApk, isFalse);
      expect(info.checksumUrl, isEmpty);
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
