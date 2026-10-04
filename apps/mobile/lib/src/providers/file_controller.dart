import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import 'remote_command_bus.dart';

class FileController {
  FileController(this._bus);
  final RemoteCommandBus _bus;

  Future<RemoteDirectoryListing?> browse([String? path]) async {
    final payload = <String, dynamic>{'type': 'browse_files'};
    if (path != null) payload['path'] = path;
    final response = await _bus.request(payload);
    if (response['type'] != 'directory_listing') return null;
    return RemoteDirectoryListing.fromJson(response['data'] as Map<String, dynamic>);
  }

  /// 扫描一个目录里的可执行文件。`kind` 为 `game` 或 `app`，决定候选条目
  /// 加入库后的分类（历史实现一律当成游戏，这是"游戏列表爆炸"的根因）。
  Future<GameScanCandidatesResponse?> scanDirectory(
    String path, {
    String kind = 'game',
  }) async {
    final response = await _bus.request({
      'type': 'scan_game_directory',
      'path': path,
      'kind': kind,
    }, timeout: const Duration(seconds: 60));
    if (response['type'] != 'game_scan_candidates') return null;
    return GameScanCandidatesResponse.fromJson(response['data'] as Map<String, dynamic>);
  }

  /// 批量加入目录条目。服务端不再重复扫描目录，所选路径直接落库。
  Future<List<LauncherItem>> addScannedItems(
    String folder,
    Iterable<String> paths, {
    Iterable<String>? names,
    String kind = 'game',
  }) async {
    final response = await _bus.request({
      'type': 'add_scanned_games',
      'path': folder,
      'exe_paths': paths.toList(growable: false),
      if (names != null) 'names': names.toList(growable: false),
      'kind': kind,
    }, timeout: const Duration(seconds: 60));
    if (response['type'] != 'scanner_apps' || response['data'] is! List) return const [];
    return (response['data'] as List).whereType<Map>().map((e) => LauncherItem.fromJson(Map<String, dynamic>.from(e))).toList(growable: false);
  }
}

final fileControllerProvider = Provider<FileController>((ref) => FileController(ref.watch(remoteCommandBusProvider)));
