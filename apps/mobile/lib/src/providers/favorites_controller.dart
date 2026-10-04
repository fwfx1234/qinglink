import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import 'launcher_controller.dart';
import 'remote_command_bus.dart';
import 'settings_provider.dart';

/// 「常用（我的库）」的服务端镜像。
///
/// 这份数据以前存在手机的 SharedPreferences 里：换手机就丢、管理端看不到，
/// 而且和"数据只有一个所有者"的架构声明矛盾。现在它只属于 PC 上的服务端，
/// 手机端只是它的一个视图（`kind = favorite` 的目录查询结果）。
class FavoritesState {
  const FavoritesState({
    this.items = const [],
    this.loading = false,
    this.error,
  });

  final List<LauncherItem> items;
  final bool loading;
  final String? error;

  /// 首页只显示置顶项。
  List<LauncherItem> get pinned =>
      items.where((item) => item.pinned).toList(growable: false);

  FavoritesState copyWith({
    List<LauncherItem>? items,
    bool? loading,
    Object? error = _unset,
  }) => FavoritesState(
    items: items ?? this.items,
    loading: loading ?? this.loading,
    error: identical(error, _unset) ? this.error : error as String?,
  );

  static const _unset = Object();
}

class FavoritesController extends StateNotifier<FavoritesState> {
  FavoritesController(this._bus, this._ref) : super(const FavoritesState());

  final RemoteCommandBus _bus;
  final Ref _ref;

  /// 常用是人工挑出来的短列表，一页足够；真超过 50 条说明该用屏蔽而不是常用。
  static const int _pageSize = 50;

  bool contains(String id) => state.items.any((item) => item.id == id);

  LauncherItem? find(String id) {
    for (final item in state.items) {
      if (item.id == id) return item;
    }
    return null;
  }

  /// 从服务端拉取常用列表，必要时先做一次本地迁移。
  Future<void> refresh() async {
    state = state.copyWith(loading: true, error: null);
    try {
      await _migrateLocalLibrary();
      final response = await _bus.request({
        'type': 'list_launcher_items',
        'kind': 'favorite',
        'page': 0,
        'page_size': _pageSize,
      });
      if (response['type'] != 'launcher_items') {
        state = state.copyWith(loading: false, error: 'PC 未返回常用列表');
        return;
      }
      final page = LauncherItemPage.fromJson(
        response['data'] as Map<String, dynamic>,
      );
      state = FavoritesState(items: page.items);
    } catch (error) {
      state = state.copyWith(loading: false, error: '$error');
    }
  }

  /// 加入常用。
  Future<bool> add(String id) async {
    final ok = await _setFlag(id, favorite: true);
    if (ok) {
      await refresh();
    }
    return ok;
  }

  /// 移出常用。置顶状态由服务端一并清除。
  Future<bool> remove(String id) async {
    final ok = await _setFlag(id, favorite: false);
    if (ok) {
      await refresh();
    }
    return ok;
  }

  /// 置顶 / 取消置顶到首页。置顶会自动加入常用。
  Future<bool> setPinned(String id, bool pinned) async {
    final ok = await _setFlag(id, pinned: pinned);
    if (ok) {
      // 先本地回显，避免整页重拉造成的闪烁；随后以服务端结果为准。
      state = state.copyWith(
        items: state.items
            .map(
              (item) => item.id == id
                  ? item.copyWith(pinned: pinned, favorite: true)
                  : item,
            )
            .toList(growable: false),
      );
      await refresh();
    }
    return ok;
  }

  Future<bool> _setFlag(
    String id, {
    bool? favorite,
    bool? pinned,
  }) async {
    try {
      if (favorite != null) {
        await _bus.request({
          'type': 'set_scanner_app_favorite',
          'id': id,
          'favorite': favorite,
        });
      }
      if (pinned != null) {
        await _bus.request({
          'type': 'set_scanner_app_pinned',
          'id': id,
          'pinned': pinned,
        });
      }
      return true;
    } catch (error) {
      state = state.copyWith(error: '$error');
      return false;
    }
  }

  /// 把旧版本存在本机的常用列表搬到服务端，只执行一次。
  ///
  /// 失败（例如当时没连上 PC）不会打上标记，下次进「库」时会重试。
  Future<void> _migrateLocalLibrary() async {
    // 设置是异步读盘加载的：必须等它完成，否则会把"还没读到本地列表"误判成
    // "本地没有列表"并打上迁移标记，用户旧数据就永远不会被搬过去。
    await _ref.read(settingsProvider.notifier).loaded;
    final settings = _ref.read(settingsProvider);
    if (settings.libraryMigrated) return;
    final local = settings.quickApps;
    if (local.isEmpty) {
      await _ref.read(settingsProvider.notifier).markLibraryMigrated();
      return;
    }
    final launcher = _ref.read(launcherControllerProvider);
    var moved = 0;
    for (final app in local) {
      var id = app.scannerId;
      if (id == null || id.isEmpty) {
        // 旧条目只有路径：先在 PC 上登记，再标记常用。
        id = await launcher.add(name: app.name, exePath: app.path);
      }
      if (id == null || id.isEmpty) continue;
      await launcher.setFavorite(id, true);
      if (app.pinnedToHome) {
        await launcher.setPinned(id, true);
      }
      moved++;
    }
    if (moved != local.length) {
      debugPrint('[QingLink][favorites] 迁移 $moved/${local.length} 条，下次重试');
      return;
    }
    await _ref.read(settingsProvider.notifier).markLibraryMigrated();
    await _ref.read(settingsProvider.notifier).clearLocalLibrary();
    debugPrint('[QingLink][favorites] 本地常用已迁移到服务端：$moved 条');
  }
}

final favoritesProvider =
    StateNotifierProvider<FavoritesController, FavoritesState>((ref) {
      return FavoritesController(ref.watch(remoteCommandBusProvider), ref);
    });
