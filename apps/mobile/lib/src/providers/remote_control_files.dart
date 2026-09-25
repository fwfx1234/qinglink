part of 'remote_control_provider.dart';

/// 远程文件浏览与游戏目录扫描相关动作。
mixin _FileSystemActions on _RemoteControlBase {
  // ========== Remote File System ==========
  Future<RemoteDirectoryListing?> browseFiles([String? path]) async {
    try {
      final resp = await _wsRequest({'type': 'browse_files', 'path': path});
      if (resp['type'] == 'directory_listing') {
        _setState(state.copyWith(clearError: true));
        return RemoteDirectoryListing.fromJson(
          resp['data'] as Map<String, dynamic>,
        );
      }
    } catch (e) {
      _setState(state.copyWith(errorMessage: e.toString()));
    }
    return null;
  }

  Future<GameScanCandidatesResponse?> scanGameDirectory(String path) async {
    final resp = await _wsCommand(
      {'type': 'scan_game_directory', 'path': path},
      timeout: const Duration(seconds: 60),
    );
    if (resp['type'] == 'game_scan_candidates') {
      return GameScanCandidatesResponse.fromJson(
        resp['data'] as Map<String, dynamic>,
      );
    }
    return null;
  }

  Future<bool> addScannedGames(String path, Iterable<String> exePaths) async {
    try {
      final resp = await _wsCommand(
        {
          'type': 'add_scanned_games',
          'path': path,
          'exe_paths': exePaths.toList(),
        },
        timeout: const Duration(seconds: 60),
      );
      return resp['type'] != 'error';
    } catch (_) {
      return false;
    }
  }

  // ========== File System ==========

  Future<FsTreeResponse?> getDirectoryTree({String? path}) async {
    try {
      final payload = <String, dynamic>{'type': 'browse_files'};
      if (path != null) {
        payload['path'] = path;
      }
      final resp = await _wsRequest(payload);
      if (resp['type'] == 'directory_listing') {
        final data = resp['data'] as Map<String, dynamic>;
        final entries = (data['entries'] as List)
            .map((e) => FsEntry.fromJson(e as Map<String, dynamic>))
            .toList();
        _setState(state.copyWith(clearError: true));
        return FsTreeResponse(
          path: data['current_path'] as String? ?? '',
          entries: entries,
        );
      }
    } catch (e) {
      _setState(state.copyWith(errorMessage: e.toString()));
    }
    return null;
  }
}
