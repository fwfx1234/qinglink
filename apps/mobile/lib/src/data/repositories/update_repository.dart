/// 更新数据源：GitHub Releases 的读取与 APK 下载。
///
/// 通道约定：移动端只认 `mobile-v` 前缀的 tag；从 asset 里找 `.apk`，再找它的
/// 校验清单——优先 `SHA256SUMS`，没有则退回与 APK 同名的 `<apk>.sha256`
/// （旧版发布沿用的命名，已安装的旧客户端只能读这个）。
/// 下载走 `http` 包的流式响应，逐块写入文件并回报进度，避免整包进内存。
library;

import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart' as http;

import '../../models/update_info.dart';

/// 默认 GitHub Releases API 地址。
const _defaultApiUrl =
    'https://api.github.com/repos/fwfx1234/qinglink/releases?per_page=30';

/// 更新仓库。URL 集中在此，便于测试时替换。
class UpdateRepository {
  /// [apiUrl] 可注入以便测试；默认指向本项目 GitHub Releases。
  UpdateRepository({http.Client? client, this.apiUrl = _defaultApiUrl})
      : _client = client ?? http.Client();

  static const _tagPrefix = 'mobile-v';
  static const _manifestAsset = 'SHA256SUMS';

  final http.Client _client;

  /// GitHub Releases API 地址。
  final String apiUrl;

  /// 检查是否有比 [currentVersion] 更新的移动端 release。
  Future<ReleaseInfo?> check(String currentVersion) async {
    final response = await _client
        .get(Uri.parse(apiUrl), headers: const {'User-Agent': 'QingLink-Mobile'})
        .timeout(const Duration(seconds: 20));
    if (response.statusCode != 200) {
      throw _httpFailure('检查更新失败', response.statusCode);
    }
    final releases = _decodeList(response.body);
    final current = ParsedVersion.parse(currentVersion);
    ReleaseInfo? newest;

    for (final release in releases) {
      final map = release as Map<String, dynamic>?;
      if (map == null || map['draft'] == true || map['prerelease'] == true) {
        continue;
      }
      final tag = map['tag_name'] as String? ?? '';
      if (!tag.startsWith(_tagPrefix)) continue;
      final version = tag.substring(_tagPrefix.length);
      if (version.isEmpty) continue;
      final parsed = ParsedVersion.parse(version);
      if (!parsed.isNewerThan(current)) continue;

      final info = _toReleaseInfo(map, tag, version);
      if (info == null) continue;
      if (newest == null ||
          ParsedVersion.parse(info.version).isNewerThan(
            ParsedVersion.parse(newest.version),
          )) {
        newest = info;
      }
    }
    return newest;
  }

  /// 下载 APK 到 [destination]，逐块回报进度（已下载字节 / 总字节，总数未知为 0）。
  Future<void> downloadApk(
    String url,
    File destination,
    void Function(int downloaded, int total)? onProgress,
  ) async {
    final request = http.Request('GET', Uri.parse(url));
    request.headers['User-Agent'] = 'QingLink-Mobile';
    final response = await _client.send(request).timeout(const Duration(minutes: 10));
    if (response.statusCode != 200) {
      throw _httpFailure('下载更新失败', response.statusCode);
    }
    final total = response.contentLength ?? 0;
    var downloaded = 0;
    final sink = destination.openWrite();
    try {
      await for (final chunk in response.stream) {
        sink.add(chunk);
        downloaded += chunk.length;
        onProgress?.call(downloaded, total);
      }
    } finally {
      await sink.close();
    }
  }

  /// 校验本地 APK 的 SHA256 是否与期望值一致。
  Future<void> verifyChecksum(File file, String expected) async {
    final digest = await sha256.bind(file.openRead()).first;
    if (digest.toString().toLowerCase() != expected.toLowerCase()) {
      throw '更新包校验失败：哈希与发布页不一致';
    }
  }

  /// 读取并解析 SHA256SUMS 清单，返回 `小写文件名 → 小写哈希` 映射。
  Future<Map<String, String>> fetchChecksums(String url) async {
    final response = await _client
        .get(Uri.parse(url), headers: const {'User-Agent': 'QingLink-Mobile'})
        .timeout(const Duration(seconds: 30));
    if (response.statusCode != 200) {
      throw _httpFailure('获取校验清单失败', response.statusCode);
    }
    final result = <String, String>{};
    for (final rawLine in response.body.split(RegExp(r'\r?\n'))) {
      final line = rawLine.trim();
      if (line.isEmpty) continue;
      if (line.startsWith('\\')) throw '校验清单含不支持的转义文件名';
      final parts = line.split(RegExp(r'\s+'));
      if (parts.length < 2) throw '校验清单格式无效';
      final hash = parts[0];
      if (!RegExp(r'^[0-9a-fA-F]{64}$').hasMatch(hash)) {
        throw '校验清单存在无效哈希';
      }
      // GNU sha256sum 二进制模式输出 `hash *file`，文本模式输出 `hash  file`；
      // 剥掉可能粘在文件名前的星号分隔符。
      var name = parts.sublist(1).join(' ').toLowerCase();
      if (name.startsWith('*')) name = name.substring(1);
      if (result.containsKey(name)) throw '校验清单存在重复文件名';
      result[name] = hash.toLowerCase();
    }
    if (result.isEmpty) throw '校验清单为空';
    return result;
  }

  /// 把 HTTP 状态码翻成能指导下一步的说法：404 基本都是发布页少了文件。
  String _httpFailure(String action, int status) => switch (status) {
        404 => '$action：发布页缺少对应文件（HTTP 404）',
        403 => '$action：GitHub 访问受限，请稍后再试（HTTP 403）',
        >= 500 => '$action：GitHub 服务异常（HTTP $status）',
        _ => '$action（HTTP $status）',
      };

  List<dynamic> _decodeList(String body) {
    final decoded = jsonDecode(body);
    if (decoded is! List) throw 'GitHub 响应格式无效';
    return decoded;
  }

  ReleaseInfo? _toReleaseInfo(
    Map<String, dynamic> map,
    String tag,
    String version,
  ) {
    final assets = map['assets'] as List<dynamic>? ?? [];
    Map<String, dynamic>? apk;
    for (final item in assets) {
      final asset = item as Map<String, dynamic>?;
      if (asset == null) continue;
      final name = asset['name'] as String? ?? '';
      if (name.endsWith('.apk')) {
        apk = asset;
        break;
      }
    }
    if (apk == null) return null;

    final apkName = apk['name'] as String? ?? 'app-release.apk';
    // 清单有两种合法命名：统一的 SHA256SUMS，或跟 APK 同名的 `.sha256`。
    // 后者是旧版发布沿用的写法，旧客户端会按「APK 地址 + .sha256」直接取，
    // 所以新发布仍产出这个文件名；这里两种都认，SHA256SUMS 优先。
    Map<String, dynamic>? manifest;
    for (final item in assets) {
      final asset = item as Map<String, dynamic>?;
      if (asset == null) continue;
      final name = asset['name'] as String? ?? '';
      if (name == _manifestAsset) {
        manifest = asset;
        break;
      }
      if (name == '$apkName.sha256') manifest = asset;
    }

    // 没有清单也要把版本报出来：下载入口会禁用，但「前往发布页」仍可用，
    // 比静默当成「已是最新」更诚实。
    return ReleaseInfo(
      tag: tag,
      version: version,
      htmlUrl: map['html_url'] as String? ?? '',
      notes: map['body'] as String? ?? '',
      apkUrl: apk['browser_download_url'] as String? ?? '',
      apkName: apkName,
      checksumUrl: manifest?['browser_download_url'] as String? ?? '',
    );
  }
}
