import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import 'remote_command_bus.dart';

class LauncherController {
  LauncherController(this._bus);
  final RemoteCommandBus _bus;

  Future<String?> icon(String id) async {
    final response = await _bus.request({'type': 'get_launcher_icon', 'id': id});
    if (response['type'] != 'launcher_icon') return null;
    final data = response['data'];
    return data is Map<String, dynamic> ? data['icon_base64'] as String? : null;
  }

  Future<void> launch(String id) => _bus.request({'type': 'launch_scanner_app', 'id': id}).then((_) {});
  Future<void> launchPath(String path, {List<String> args = const []}) => _bus.request({'type': 'launch_app', 'path': path, 'args': args}).then((_) {});
  Future<bool> update({required String id, required String name, required String exePath, String? iconBase64, List<String>? args}) async {
    final payload = <String, dynamic>{'type': 'update_scanner_app', 'id': id, 'name': name, 'exe_path': exePath};
    if (iconBase64 != null) payload['icon_base64'] = iconBase64;
    if (args != null) payload['args'] = args;
    final response = await _bus.request(payload);
    return response['type'] != 'error';
  }

  Future<bool> delete(String id) async {
    final response = await _bus.request({'type': 'delete_scanner_app', 'id': id});
    return response['type'] != 'error';
  }

  Future<void> rename(String id, String originalName, String name) => _bus
      .request({'type': 'rename_scanner_app', 'id': id, 'original_name': originalName, 'name': name})
      .then((_) {});

  /// 重新扫描图标。`iconPath` 为空表示走自动来源链（快捷方式图标 → 程序内嵌
  /// 资源 → 同目录 PNG/ICO → 系统默认图标）。
  Future<IconUpdateResult> refreshIcon(String id, {String? iconPath}) async {
    final response = await _bus.request({
      'type': 'refresh_scanner_app_icon',
      'id': id,
      if (iconPath != null && iconPath.trim().isNotEmpty) 'icon_path': iconPath.trim(),
    }, timeout: const Duration(seconds: 30));
    final data = response['data'];
    if (data is Map) {
      return IconUpdateResult.fromJson(Map<String, dynamic>.from(data));
    }
    return const IconUpdateResult(id: '', source: 'none');
  }

  /// 列出 PC 上该条目可用的图标文件，供用户直接挑选。
  Future<List<IconCandidate>> iconCandidates(String id) async {
    final response = await _bus.request({
      'type': 'list_scanner_app_icons',
      'id': id,
    }, timeout: const Duration(seconds: 20));
    if (response['type'] != 'icon_candidates') return const [];
    final data = response['data'];
    if (data is! Map) return const [];
    final candidates = data['candidates'];
    if (candidates is! List) return const [];
    return candidates
        .whereType<Map>()
        .map((item) => IconCandidate.fromJson(Map<String, dynamic>.from(item)))
        .where((item) => item.path.isNotEmpty)
        .toList(growable: false);
  }

  /// 屏蔽 / 恢复一个条目。屏蔽后在「库」里不再出现，但数据仍在 PC 上。
  Future<bool> setHidden(String id, bool hidden) async {
    final response = await _bus.request({
      'type': 'set_scanner_app_hidden',
      'id': id,
      'hidden': hidden,
    });
    return response['type'] != 'error';
  }

  /// 加入 / 移出「常用（我的库）」。数据存在 PC 上，换手机也是同一份。
  Future<bool> setFavorite(String id, bool favorite) async {
    final response = await _bus.request({
      'type': 'set_scanner_app_favorite',
      'id': id,
      'favorite': favorite,
    });
    return response['type'] != 'error';
  }

  /// 置顶 / 取消置顶到手机首页。置顶会自动加入常用。
  Future<bool> setPinned(String id, bool pinned) async {
    final response = await _bus.request({
      'type': 'set_scanner_app_pinned',
      'id': id,
      'pinned': pinned,
    });
    return response['type'] != 'error';
  }

  /// 已被屏蔽的条目，用于恢复。
  Future<List<LauncherItem>> blockedItems() async {
    final response = await _bus.request({'type': 'list_blocked_scanner_apps'});
    if (response['type'] != 'blocked_launcher_items') return const [];
    final data = response['data'];
    final items = data is Map ? data['items'] : null;
    if (items is! List) return const [];
    return items
        .whereType<Map>()
        .map((item) => LauncherItem.fromJson(Map<String, dynamic>.from(item)))
        .toList(growable: false);
  }

  Future<void> refreshCatalog(String kind) =>
      _bus.request({'type': 'refresh_launcher_catalog', 'kind': kind}).then((_) {});

  Future<String?> add({required String name, required String exePath}) async {
    final response = await _bus.request({
      'type': 'add_scanner_app',
      'name': name,
      'exe_path': exePath,
    });
    final data = response['data'];
    return data is Map ? data['id'] as String? : response['id'] as String?;
  }
}

final launcherControllerProvider = Provider<LauncherController>((ref) => LauncherController(ref.watch(remoteCommandBusProvider)));
