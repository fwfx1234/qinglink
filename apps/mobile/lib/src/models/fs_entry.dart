class FsEntry {
  final String name;
  final String path;
  final bool isDir;

  const FsEntry({
    required this.name,
    required this.path,
    required this.isDir,
  });

  factory FsEntry.fromJson(Map<String, dynamic> json) {
    return FsEntry(
      name: json['name'] as String,
      path: json['path'] as String,
      isDir: json['is_dir'] as bool? ?? true,
    );
  }

  Map<String, dynamic> toJson() => {
        'name': name,
        'path': path,
        'is_dir': isDir,
      };
}

class FsTreeResponse {
  final String path;
  final List<FsEntry> entries;

  const FsTreeResponse({
    required this.path,
    required this.entries,
  });

  factory FsTreeResponse.fromJson(Map<String, dynamic> json) {
    return FsTreeResponse(
      path: json['path'] as String,
      entries: (json['entries'] as List)
          .map((e) => FsEntry.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }

  Map<String, dynamic> toJson() => {
        'path': path,
        'entries': entries.map((e) => e.toJson()).toList(),
      };
}
