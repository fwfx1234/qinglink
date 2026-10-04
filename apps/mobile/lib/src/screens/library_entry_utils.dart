/// 目录条目的来源标签、用户添加判定与启动参数编解码。
library;

/// Human readable source for a library entry.  Folder scans and manual
/// additions are the same kind of user-created entry.
String librarySourceLabel(String source) {
  switch (source.toLowerCase()) {
    case 'steam':
      return 'Steam';
    case 'startmenu':
      return '开始菜单';
    case 'custom':
    case 'user':
      return '手动添加';
    default:
      return source;
  }
}

/// Only entries the user created may be deleted from the PC catalog.
bool isUserAddedLibraryItem(String source) {
  final normalized = source.toLowerCase();
  return normalized == 'user' || normalized == 'custom';
}

/// 把「空格分隔、可以带引号」的启动参数串解析成参数列表。
///
/// 引号没配平时返回 `null`，由调用方提示用户，而不是静默丢掉半截参数。
List<String>? parseLaunchArgs(String raw) {
  final tokens = <String>[];
  final buffer = StringBuffer();
  var inQuotes = false;
  var escaped = false;
  for (var i = 0; i < raw.length; i++) {
    final ch = raw[i];
    if (escaped) {
      buffer.write(ch);
      escaped = false;
    } else if (ch == '\\' && i + 1 < raw.length && raw[i + 1] == '"') {
      escaped = true;
    } else if (ch == '"') {
      inQuotes = !inQuotes;
    } else if (!inQuotes && ch.trim().isEmpty) {
      if (buffer.isNotEmpty) {
        tokens.add(buffer.toString());
        buffer.clear();
      }
    } else {
      buffer.write(ch);
    }
  }
  if (inQuotes || escaped) return null;
  if (buffer.isNotEmpty) tokens.add(buffer.toString());
  return tokens;
}

/// 把参数列表回写成可编辑的带引号形式（[parseLaunchArgs] 的逆向操作）。
String formatLaunchArgs(List<String> args) =>
    args
        .map(
          (arg) => arg.contains(RegExp(r'[\s"]'))
              ? '"${arg.replaceAll('"', '\\"')}"'
              : arg,
        )
        .join(' ');
