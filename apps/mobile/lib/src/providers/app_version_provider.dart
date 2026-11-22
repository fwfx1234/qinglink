/// 应用版本信息：供设置页展示与检查更新共用。
///
/// 版本号来自 `pubspec.yaml`，由 `package_info_plus` 在运行时读取，避免像旧版那样
/// 在界面里写死 `1.0.0`——写死会让界面版本与真实产物版本不一致，检查更新也失去意义。
library;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';

/// 应用版本信息。
class AppVersionInfo {
  const AppVersionInfo({required this.version, required this.buildNumber});

  /// 语义版本，如 `1.0.0`。
  final String version;

  /// 构建号，如 `2`。
  final String buildNumber;

  /// 完整版本串，与 GitHub Release 的 tag 版本部分同形，如 `1.0.0+2`。
  String get displayVersion => '$version+$buildNumber';
}

/// 读取本机应用版本。检查更新与设置页共用一个来源。
final appVersionProvider = FutureProvider<AppVersionInfo>((ref) async {
  final info = await PackageInfo.fromPlatform();
  return AppVersionInfo(version: info.version, buildNumber: info.buildNumber);
});
