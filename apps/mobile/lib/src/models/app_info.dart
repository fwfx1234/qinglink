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

  /// 图标来源（manual/steam/shortcut/executable/image/shell）。
  /// 为 `shell` 表示只拿到系统默认图标，通常意味着图标不准。
  final String? iconSource;

  /// 目录扫描提示：可执行文件大小（字节）。
  final int? sizeBytes;

  /// 目录扫描提示：这个 exe 像是所在目录的主程序。
  final bool recommended;

  /// 在「常用（我的库）」中。由服务端拥有，因此换手机也一致。
  final bool favorite;

  /// 置顶到手机首页。
  final bool pinned;

  const LauncherItem({
    required this.id,
    required this.name,
    required this.originalName,
    required this.executablePath,
    required this.category,
    required this.source,
    this.iconBase64,
    this.args = const [],
    this.iconSource,
    this.sizeBytes,
    this.recommended = false,
    this.favorite = false,
    this.pinned = false,
  });

  /// 只拿到系统默认图标时，图标几乎必然与程序不符。
  bool get iconLooksWrong =>
      iconSource == null || iconSource == 'shell' || iconSource == 'none';

  factory LauncherItem.fromJson(Map<String, dynamic> json) => LauncherItem(
    id: json['id'] as String,
    name: json['name'] as String,
    originalName: json['original_name'] as String? ?? json['name'] as String,
    executablePath: json['exe_path'] as String? ?? '',
    category: json['category'] as String? ?? 'Application',
    source: json['source'] as String? ?? 'Unknown',
    iconBase64: json['icon_base64'] as String?,
    iconSource: json['icon_source'] as String?,
    sizeBytes: (json['size_bytes'] as num?)?.toInt(),
    recommended: json['recommended'] as bool? ?? false,
    favorite: json['favorite'] as bool? ?? false,
    pinned: json['pinned'] as bool? ?? false,
    args:
        (json['args'] as List<dynamic>?)
            ?.map((value) => value.toString())
            .toList() ??
        const [],
  );

  /// 服务端改了标记后本地先更新，避免整页重拉带来的闪烁。
  LauncherItem copyWith({bool? favorite, bool? pinned, String? name, List<String>? args}) =>
      LauncherItem(
        id: id,
        name: name ?? this.name,
        originalName: originalName,
        executablePath: executablePath,
        category: category,
        source: source,
        iconBase64: iconBase64,
        args: args ?? this.args,
        iconSource: iconSource,
        sizeBytes: sizeBytes,
        recommended: recommended,
        favorite: favorite ?? this.favorite,
        pinned: pinned ?? this.pinned,
      );
}

/// 单个条目可选的图标文件（来自 PC 上的实际文件）。
class IconCandidate {
  final String path;

  /// executable | shortcut | shortcut_icon | image
  final String kind;
  final int sizeBytes;

  const IconCandidate({
    required this.path,
    required this.kind,
    required this.sizeBytes,
  });

  factory IconCandidate.fromJson(Map<String, dynamic> json) => IconCandidate(
    path: json['path'] as String? ?? '',
    kind: json['kind'] as String? ?? 'image',
    sizeBytes: (json['size_bytes'] as num?)?.toInt() ?? 0,
  );

  String get kindLabel => switch (kind) {
    'executable' => '程序本体',
    'shortcut' => '快捷方式',
    'shortcut_icon' => '快捷方式图标',
    'image' => '图片文件',
    _ => kind,
  };
}

/// 重新解析图标的结果。
class IconUpdateResult {
  final String id;
  final String? iconBase64;
  final String source;
  final String? iconPath;

  const IconUpdateResult({
    required this.id,
    this.iconBase64,
    required this.source,
    this.iconPath,
  });

  factory IconUpdateResult.fromJson(Map<String, dynamic> json) =>
      IconUpdateResult(
        id: json['id'] as String? ?? '',
        iconBase64: json['icon_base64'] as String?,
        source: json['source'] as String? ?? 'none',
        iconPath: json['icon_path'] as String?,
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
