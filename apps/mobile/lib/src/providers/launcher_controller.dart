import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../models/display_info.dart';
import 'remote_command_bus.dart';

class LauncherController {
  LauncherController(this._bus);
  final RemoteCommandBus _bus;

  /// 图标按需拉取；失败时返回 `null` 让调用方保留回退图标。
  ///
  /// 列表里每行都会调用它，异常逃逸会变成"整页刷不出来 + 控制台报错"。
  Future<String?> icon(String id) async {
    try {
      final response = await _bus.request({'type': 'get_launcher_icon', 'id': id});
      if (response['type'] != 'launcher_icon') return null;
      final data = response['data'];
      return data is Map<String, dynamic> ? data['icon_base64'] as String? : null;
    } catch (error) {
      debugPrint('launcher icon failed: $error');
      return null;
    }
  }

  Future<void> launch(String id) => _bus.request({'type': 'launch_scanner_app', 'id': id}).then((_) {});
  Future<void> launchPath(String path, {List<String> args = const []}) => _bus.request({'type': 'launch_app', 'path': path, 'args': args}).then((_) {});
  Future<bool> update({required String id, required String name, required String exePath, String? iconBase64, List<String>? args, DisplayModeInfo? launchMode, bool clearLaunchMode = false}) {
    final payload = <String, dynamic>{'type': 'update_scanner_app', 'id': id, 'name': name, 'exe_path': exePath};
    if (iconBase64 != null) payload['icon_base64'] = iconBase64;
    if (args != null) payload['args'] = args;
    if (launchMode != null) payload['launch_mode'] = launchMode.toJson();
    if (clearLaunchMode) payload['clear_launch_mode'] = true;
    return _requestOk(payload);
  }

  Future<bool> delete(String id) =>
      _requestOk({'type': 'delete_scanner_app', 'id': id});

  /// 重命名条目。服务端字段是 `new_name`（历史实现里发错过 `name`）。
  Future<bool> rename(String id, String originalName, String name) =>
      _requestOk({
        'type': 'rename_scanner_app',
        'id': id,
        'original_name': originalName,
        'new_name': name,
      });

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
  Future<bool> setHidden(String id, bool hidden) => _requestOk({
    'type': 'set_scanner_app_hidden',
    'id': id,
    'hidden': hidden,
  });

  /// 加入 / 移出「常用（我的库）」。数据存在 PC 上，换手机也是同一份。
  Future<bool> setFavorite(String id, bool favorite) => _requestOk({
    'type': 'set_scanner_app_favorite',
    'id': id,
    'favorite': favorite,
  });

  /// 置顶 / 取消置顶到手机首页。置顶会自动加入常用。
  Future<bool> setPinned(String id, bool pinned) => _requestOk({
    'type': 'set_scanner_app_pinned',
    'id': id,
    'pinned': pinned,
  });

  /// 预览 PC 上的一个图标文件（只解码，不落库）。
  ///
  /// 文件不可读或不是图标管线能解析的类型时返回 `null`，由调用方保留占位图。
  Future<String?> previewIcon(String path) async {
    try {
      final response = await _bus.request({
        'type': 'get_icon_preview',
        'path': path,
      });
      if (response['type'] != 'icon_preview') return null;
      final data = response['data'];
      return data is Map ? data['icon_base64'] as String? : null;
    } catch (error) {
      debugPrint('icon preview failed: $error');
      return null;
    }
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

  /// 把用户选的程序加入目录。`category` 是服务端必填字段：忽略大小写等于 `game`
  /// 归入游戏，其余归入软件，缺字段会被判为无效请求。
  Future<String?> add({
    required String name,
    required String exePath,
    String category = 'Application',
  }) async {
    try {
      final response = await _bus.request({
        'type': 'add_scanner_app',
        'name': name,
        'exe_path': exePath,
        'category': category,
      });
      final data = response['data'];
      return data is Map ? data['id'] as String? : response['id'] as String?;
    } catch (error) {
      debugPrint('launcher add failed: $error');
      return null;
    }
  }

  /// 写命令统一收口：服务端拒绝（`INVALID_REQUEST`、`NOT_FOUND`…）或连接断开时返回
  /// `false`，失败原因留在日志里。
  ///
  /// 这些方法的调用方都在弹层/弹窗关闭后的异步回调中（重命名、删除、屏蔽、置顶…），
  /// 抛异常只会在控制台变成未捕获错误，用户看到的是"点一下就报错"。
  Future<bool> _requestOk(Map<String, dynamic> payload) async {
    try {
      final response = await _bus.request(payload);
      return response['type'] != 'error';
    } catch (error) {
      debugPrint('launcher command failed: ${payload['type']} $error');
      return false;
    }
  }
}

final launcherControllerProvider = Provider<LauncherController>((ref) => LauncherController(ref.watch(remoteCommandBusProvider)));
