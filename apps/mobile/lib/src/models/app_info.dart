/// A compact launch-directory entry. Icons are fetched on demand, while the
/// executable path is available for the details view.
class LauncherItem {
  final String id;
  final String name;
  final String originalName;
  final String executablePath;
  final String category;
  final String source;
  final String? iconBase64;
  final List<String> args;

  const LauncherItem({
    required this.id,
    required this.name,
    required this.originalName,
    required this.executablePath,
    required this.category,
    required this.source,
    this.iconBase64,
    this.args = const [],
  });

  factory LauncherItem.fromJson(Map<String, dynamic> json) => LauncherItem(
    id: json['id'] as String,
    name: json['name'] as String,
    originalName: json['original_name'] as String? ?? json['name'] as String,
    executablePath: json['exe_path'] as String? ?? '',
    category: json['category'] as String? ?? 'Application',
    source: json['source'] as String? ?? 'Unknown',
    iconBase64: json['icon_base64'] as String?,
    args:
        (json['args'] as List<dynamic>?)
            ?.map((value) => value.toString())
            .toList() ??
        const [],
  );
}

class LauncherItemPage {
  final List<LauncherItem> items;
  final int total;
  final int page;
  final int pageSize;
  final bool hasMore;

  const LauncherItemPage({
    required this.items,
    required this.total,
    required this.page,
    required this.pageSize,
    required this.hasMore,
  });

  factory LauncherItemPage.fromJson(Map<String, dynamic> json) =>
      LauncherItemPage(
        items: (json['items'] as List)
            .map((item) => LauncherItem.fromJson(item as Map<String, dynamic>))
            .toList(),
        total: json['total'] as int,
        page: json['page'] as int,
        pageSize: json['page_size'] as int,
        hasMore: json['has_more'] as bool? ?? false,
      );
}

class GameScanCandidatesResponse {
  final String path;
  final List<LauncherItem> candidates;

  const GameScanCandidatesResponse({
    required this.path,
    required this.candidates,
  });

  factory GameScanCandidatesResponse.fromJson(Map<String, dynamic> json) =>
      GameScanCandidatesResponse(
        path: json['path'] as String,
        candidates: (json['candidates'] as List)
            .map((item) => LauncherItem.fromJson(item as Map<String, dynamic>))
            .toList(),
      );
}

class RemoteFileEntry {
  final String name;
  final String path;
  final bool isDir;

  const RemoteFileEntry({
    required this.name,
    required this.path,
    required this.isDir,
  });

  factory RemoteFileEntry.fromJson(Map<String, dynamic> json) =>
      RemoteFileEntry(
        name: json['name'] as String,
        path: json['path'] as String,
        isDir: json['is_dir'] as bool,
      );
}

class RemoteDirectoryListing {
  final String currentPath;
  final String? parentPath;
  final List<RemoteFileEntry> entries;

  const RemoteDirectoryListing({
    required this.currentPath,
    required this.parentPath,
    required this.entries,
  });

  factory RemoteDirectoryListing.fromJson(Map<String, dynamic> json) =>
      RemoteDirectoryListing(
        currentPath: json['current_path'] as String,
        parentPath: json['parent_path'] as String?,
        entries: (json['entries'] as List)
            .map(
              (item) => RemoteFileEntry.fromJson(item as Map<String, dynamic>),
            )
            .toList(),
      );
}

// ============================================================
// 任务管理器
// ============================================================

// ============================================================
// Steam 游戏
// ============================================================

class SteamGame {
  final int appId;
  final String name;
  final String installDir;
  final String installPath;
  final String libraryPath;
  final String? iconPath;
  final int? sizeBytes;
  final int? lastPlayed;

  const SteamGame({
    required this.appId,
    required this.name,
    required this.installDir,
    required this.installPath,
    required this.libraryPath,
    this.iconPath,
    this.sizeBytes,
    this.lastPlayed,
  });

  factory SteamGame.fromJson(Map<String, dynamic> json) {
    return SteamGame(
      appId: json['app_id'] as int,
      name: json['name'] as String,
      installDir: json['install_dir'] as String,
      installPath: json['install_path'] as String,
      libraryPath: json['library_path'] as String,
      iconPath: json['icon_path'] as String?,
      sizeBytes: json['size_bytes'] as int?,
      lastPlayed: json['last_played'] as int?,
    );
  }
}

class SteamLibrary {
  final String path;
  final int gameCount;

  const SteamLibrary({required this.path, required this.gameCount});

  factory SteamLibrary.fromJson(Map<String, dynamic> json) {
    return SteamLibrary(
      path: json['path'] as String,
      gameCount: json['game_count'] as int,
    );
  }
}

class SteamGamesResponse {
  final bool steamInstalled;
  final String? steamPath;
  final List<SteamLibrary> libraries;
  final List<SteamGame> games;
  final int total;

  const SteamGamesResponse({
    required this.steamInstalled,
    this.steamPath,
    required this.libraries,
    required this.games,
    required this.total,
  });

  factory SteamGamesResponse.fromJson(Map<String, dynamic> json) {
    return SteamGamesResponse(
      steamInstalled: json['steam_installed'] as bool,
      steamPath: json['steam_path'] as String?,
      libraries: (json['libraries'] as List)
          .map((e) => SteamLibrary.fromJson(e as Map<String, dynamic>))
          .toList(),
      games: (json['games'] as List)
          .map((e) => SteamGame.fromJson(e as Map<String, dynamic>))
          .toList(),
      total: json['total'] as int,
    );
  }
}
