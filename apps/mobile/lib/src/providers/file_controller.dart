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

  Future<GameScanCandidatesResponse?> scanGameDirectory(String path) async {
    final response = await _bus.request({'type': 'scan_game_directory', 'path': path}, timeout: const Duration(seconds: 60));
    if (response['type'] != 'game_scan_candidates') return null;
    return GameScanCandidatesResponse.fromJson(response['data'] as Map<String, dynamic>);
  }

  Future<List<LauncherItem>> addScannedGames(String path, Iterable<String> paths, {Iterable<String>? names}) async {
    final response = await _bus.request({'type': 'add_scanned_games', 'path': path, 'exe_paths': paths.toList(growable: false), if (names != null) 'names': names.toList(growable: false)}, timeout: const Duration(seconds: 60));
    if (response['type'] != 'scanner_apps' || response['data'] is! List) return const [];
    return (response['data'] as List).whereType<Map>().map((e) => LauncherItem.fromJson(Map<String, dynamic>.from(e))).toList(growable: false);
  }
}

final fileControllerProvider = Provider<FileController>((ref) => FileController(ref.watch(remoteCommandBusProvider)));
