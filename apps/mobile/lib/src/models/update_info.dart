/// 移动端更新模型：GitHub Release 的版本解析与比较。
///
/// 版本号形如 `1.0.0+2`（来自 pubspec.yaml）。语义版本三段与 `+` 后的构建号都参与
/// 比较——本项目的构建号是随源码改动递增的序号，忽略它会把 `1.0.0+2` → `1.0.0+3`
/// 误判成同版本。与桌面端 `qinglink_version::is_newer` 保持同一套语义。
library;

/// 解析后的版本号四元组，可直接按字典序比较。
class ParsedVersion implements Comparable<ParsedVersion> {
  const ParsedVersion(this.major, this.minor, this.patch, this.build);

  final int major;
  final int minor;
  final int patch;
  final int build;

  /// 从 `1.0.0+2` 这样的字符串解析；任一段无效按 0 处理。
  factory ParsedVersion.parse(String value) {
    final plus = value.indexOf('+');
    final core = plus >= 0 ? value.substring(0, plus) : value;
    final build = plus >= 0 ? value.substring(plus + 1) : '0';
    final parts = core.split('.');
    int at(int index) =>
        index < parts.length ? int.tryParse(parts[index]) ?? 0 : 0;
    return ParsedVersion(at(0), at(1), at(2), int.tryParse(build) ?? 0);
  }

  @override
  int compareTo(ParsedVersion other) {
    if (major != other.major) return major.compareTo(other.major);
    if (minor != other.minor) return minor.compareTo(other.minor);
    if (patch != other.patch) return patch.compareTo(other.patch);
    return build.compareTo(other.build);
  }

  /// 本版本是否比 [other] 更新。
  bool isNewerThan(ParsedVersion other) => compareTo(other) > 0;

  @override
  String toString() => '$major.$minor.$patch+$build';
}

/// GitHub Release 中检查更新所需的字段。
class ReleaseInfo {
  const ReleaseInfo({
    required this.tag,
    required this.version,
    required this.htmlUrl,
    required this.notes,
    required this.apkUrl,
    required this.apkName,
  });

  /// 完整 tag，例如 `mobile-v1.0.0+3`。
  final String tag;

  /// 去掉前缀的版本号，例如 `1.0.0+3`。
  final String version;

  /// release 页面地址（iOS 与手动下载用）。
  final String htmlUrl;

  /// 更新说明。
  final String notes;

  /// APK 下载地址；没有 APK 资产时为空。
  final String apkUrl;

  /// APK 文件名。
  final String apkName;

  bool get hasApk => apkUrl.isNotEmpty;
}
