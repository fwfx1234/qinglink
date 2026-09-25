part of 'remote_control_provider.dart';

/// 应用/游戏启动器、扫描器与 Steam 相关动作。
mixin _LauncherActions on _RemoteControlBase {
  /// Loads the unified mobile library for one category. The server accepts
  /// `steam`, `software` and `other` as kinds; `all` is kept for legacy calls.
  @override
  Future<bool> loadUnifiedLibrary({
    String? kind,
    String? query,
    bool force = false,
    int page = 0,
    bool append = false,
  }) async {
    if (state.isLoadingLibrary && !force && !append) return false;
    // Fall back to the category already on screen so background refreshes do
    // not clobber the active tab with the legacy `all` list.
    final effectiveKind = kind ?? state.libraryKind;
    // A category switch replaces the list, so the previous query no longer
    // applies unless the caller supplied a new one.
    final queryText = query ?? (append ? state.libraryQuery : '');
    _setState(
      append
          ? state.copyWith(isLoadingMoreLibrary: true)
          : state.copyWith(
              isLoadingLibrary: true,
              libraryQuery: queryText,
              libraryKind: effectiveKind,
            ),
    );
    try {
      final response = await _wsRequest({
        'type': 'list_launcher_items',
        'kind': effectiveKind,
        if (queryText.trim().isNotEmpty) 'query': queryText.trim(),
        'page': page,
        'page_size': 50,
      });
      if (response['type'] != 'launcher_items') {
        throw StateError('PC 返回了无效的启动器列表响应');
      }
      final result = LauncherItemPage.fromJson(
        response['data'] as Map<String, dynamic>,
      );
      _setState(
        state.copyWith(
          launcherItems: append
              ? [...state.launcherItems, ...result.items]
              : result.items,
          isLoadingLibrary: false,
          isLoadingMoreLibrary: false,
          libraryHasMore: result.hasMore,
          libraryPage: result.page,
          libraryKind: effectiveKind,
        ),
      );
      return true;
    } catch (error) {
      _setState(
        state.copyWith(
          isLoadingLibrary: false,
          isLoadingMoreLibrary: false,
          errorMessage: error.toString(),
        ),
      );
      return false;
    }
  }

  @override
  Future<bool> loadRecentUnifiedLibrary() async {
    try {
      final response = await _wsRequest({
        'type': 'get_recent_launcher_items',
        'kind': 'all',
      });
      if (response['type'] != 'recent_launcher_items') return false;
      final raw = response['data'];
      if (raw is! List) return false;
      final items = raw
          .whereType<Map>()
          .map((item) => LauncherItem.fromJson(Map<String, dynamic>.from(item)))
          .toList(growable: false);
      _setState(state.copyWith(recentLauncherItems: items));
      return true;
    } catch (_) {
      return false;
    }
  }

  // ========== Mobile launch directory ==========

  @override
  Future<bool> loadLauncherItems(
    String kind, {
    String? query,
    int page = 0,
    bool append = false,
    bool force = false,
  }) async {
    final loading = kind == 'game' ? state.isLoadingGames : state.isLoadingApps;
    if (loading && !force) return false;

    final loadId = (_launcherLoadGeneration[kind] ?? 0) + 1;
    _launcherLoadGeneration[kind] = loadId;
    _setLauncherLoading(kind, true);
    _setState(state.copyWith(isLoadingLauncher: true));
    try {
      final resp = await _wsRequest({
        'type': 'list_launcher_items',
        'kind': kind,
        if (query != null && query.isNotEmpty) 'query': query,
        'page': page,
        'page_size': 30,
      });
      if (_launcherLoadGeneration[kind] != loadId) return false;
      if (resp['type'] == 'launcher_items') {
        final result = LauncherItemPage.fromJson(
          resp['data'] as Map<String, dynamic>,
        );
        if (kind == 'game') {
          _setState(
            state.copyWith(
              gameItems: append
                  ? [...state.gameItems, ...result.items]
                  : result.items,
              gameHasMore: result.hasMore,
              isLoadingGames: false,
              isLoadingLauncher: state.isLoadingApps,
            ),
          );
          return true;
        } else {
          _setState(
            state.copyWith(
              appItems: append
                  ? [...state.appItems, ...result.items]
                  : result.items,
              appHasMore: result.hasMore,
              isLoadingApps: false,
              isLoadingLauncher: state.isLoadingGames,
            ),
          );
          return true;
        }
      } else {
        _setLauncherLoading(kind, false);
        return false;
      }
    } catch (error) {
      if (_launcherLoadGeneration[kind] != loadId) return false;
      _setState(
        state.copyWith(
          isLoadingApps: kind == 'app' ? false : null,
          isLoadingGames: kind == 'game' ? false : null,
          isLoadingLauncher: false,
          errorMessage: error.toString(),
        ),
      );
      return false;
    }
  }

  void _setLauncherLoading(String kind, bool loading) {
    final otherLoading = kind == 'game'
        ? state.isLoadingApps
        : state.isLoadingGames;
    _setState(
      state.copyWith(
        isLoadingApps: kind == 'app' ? loading : null,
        isLoadingGames: kind == 'game' ? loading : null,
        isLoadingLauncher: loading || otherLoading,
      ),
    );
  }

  Future<void> refreshLauncherCatalog(String kind) async {
    _setState(state.copyWith(isRefreshingApps: true));
    try {
      await _wsCommand({'type': 'refresh_launcher_catalog', 'kind': kind});
    } catch (_) {
      _setState(state.copyWith(isRefreshingApps: false));
      rethrow;
    }
  }

  Future<String?> getLauncherIcon(String id) async {
    if (_iconCache.containsKey(id)) return _iconCache[id];
    try {
      final resp = await _wsRequest({'type': 'get_launcher_icon', 'id': id});
      final icon = resp['type'] == 'launcher_icon'
          ? (resp['data'] as Map<String, dynamic>)['icon_base64'] as String?
          : null;
      _iconCache[id] = icon;
      if (_iconCache.length > 50) _iconCache.remove(_iconCache.keys.first);
      return icon;
    } catch (_) {
      return null;
    }
  }

  /// Re-extract the icon on the PC and persist it for one scanner entry.
  Future<String?> refreshScannerAppIcon(String id) async {
    _iconCache.remove(id);
    final resp = await _wsRequest({
      'type': 'refresh_scanner_app_icon',
      'id': id,
    });
    final icon = resp['type'] == 'launcher_icon'
        ? (resp['data'] as Map<String, dynamic>)['icon_base64'] as String?
        : null;
    _iconCache[id] = icon;
    return icon;
  }

  Future<void> launchLauncherItem(LauncherItem item) async {
    await launchScannerApp(item.id);
  }

  // ========== Apps ==========

  Future<void> launchApp(String path, {List<String>? args}) async {
    await _wsCommand({'type': 'launch_app', 'path': path, 'args': args ?? []});
  }

  // ========== Scanner Apps ==========

  Future<void> refreshScannerApps() async {
    _setState(state.copyWith(isRefreshingApps: true));
    try {
      await Future.wait([
        loadLauncherItems('app', force: true),
        loadLauncherItems('game', force: true),
      ]);
      _setState(state.copyWith(isRefreshingApps: false));
    } catch (_) {
      _setState(state.copyWith(isRefreshingApps: false));
    }
  }

  Future<void> forceRefreshScannerApps() async {
    _setState(state.copyWith(isRefreshingApps: true));
    try {
      await refreshLauncherCatalog('app');
    } finally {
      _setState(state.copyWith(isRefreshingApps: false));
    }
  }

  Future<void> launchScannerApp(String id) async {
    await _wsCommand({'type': 'launch_scanner_app', 'id': id});
  }

  Future<void> renameScannerApp(
    String id,
    String originalName,
    String newName,
  ) async {
    await _wsCommand({
      'type': 'rename_scanner_app',
      'id': id,
      'original_name': originalName,
      'new_name': newName,
    });
  }

  // ========== Steam ==========

  Future<SteamGamesResponse?> getSteamGames() async {
    try {
      final resp = await _wsRequest({'type': 'get_steam_games'});
      if (resp['type'] == 'steam_games') {
        _setState(state.copyWith(clearError: true));
        return SteamGamesResponse.fromJson(
          resp['data'] as Map<String, dynamic>,
        );
      }
    } catch (e) {
      _setState(state.copyWith(errorMessage: e.toString()));
    }
    return null;
  }

  Future<void> refreshSteamGames() async {
    try {
      await _wsRequest({'type': 'refresh_steam_games'});
      await getSteamGames();
    } catch (_) {}
  }

  Future<void> installSteamGame(int appId) async {
    await _wsCommand({'type': 'install_steam_game', 'app_id': appId});
  }

  // === Scanner App Management ===

  Future<String?> addScannerApp({
    required String name,
    required String exePath,
    String category = 'Application',
    String? iconBase64,
    String? source,
    List<String>? args,
  }) async {
    try {
      final payload = <String, dynamic>{
        'type': 'add_scanner_app',
        'name': name,
        'exe_path': exePath,
        'category': category,
      };
      if (iconBase64 != null) {
        payload['icon_base64'] = iconBase64;
      }
      if (source != null) {
        payload['source'] = source;
      }
      if (args != null && args.isNotEmpty) {
        payload['args'] = args;
      }
      final resp = await _wsCommand(payload);
      if (resp['type'] == 'scanner_app_updated') {
        final data = resp['data'];
        if (data is Map && data['id'] is String) {
          return data['id'] as String;
        }
      }
    } catch (_) {}
    return null;
  }

  Future<bool> updateScannerApp({
    required String id,
    required String name,
    required String exePath,
    String? iconBase64,
    List<String>? args,
  }) async {
    try {
      final payload = <String, dynamic>{
        'type': 'update_scanner_app',
        'id': id,
        'name': name,
        'exe_path': exePath,
      };
      if (iconBase64 != null) {
        payload['icon_base64'] = iconBase64;
      }
      if (args != null) {
        payload['args'] = args;
      }
      final resp = await _wsCommand(payload);
      if (resp['type'] != 'error') {
        return true;
      }
    } catch (_) {}
    return false;
  }

  Future<bool> deleteScannerApp(String id) async {
    try {
      final resp = await _wsCommand({'type': 'delete_scanner_app', 'id': id});
      if (resp['type'] != 'error') {
        return true;
      }
    } catch (_) {}
    return false;
  }

  Future<bool> setScannerAppHidden(String id, bool hidden) async {
    try {
      final resp = await _wsCommand({
        'type': 'set_scanner_app_hidden',
        'id': id,
        'hidden': hidden,
      });
      if (resp['type'] != 'error') {
        return true;
      }
    } catch (_) {}
    return false;
  }
}
