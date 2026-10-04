/// 「库」分区入口：分区状态、动作分发与装配。
///
/// 其余职责分别落在同目录的独立库里：[LibraryFilter] 与标签在
/// `library_filter.dart`，列表行/卡片在 `library_tiles.dart`，各分区视图在
/// `library_views.dart`，已屏蔽分区在 `library_blocked_section.dart`，工具栏在
/// `library_toolbar.dart`，菜单在 `library_sheets.dart`，对话框在
/// `library_dialogs.dart`，条目元信息与启动参数工具在 `library_entry_utils.dart`，
/// 条目管理动作与添加/扫描动作在 `library_item_actions.dart` 与
/// `library_scan_actions.dart`。本文件只保留 `LibraryTab` 与它必须持有的状态。
library;

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/auth_provider.dart';
import '../providers/favorites_controller.dart';
import '../providers/launcher_controller.dart';
import '../providers/library_controller.dart';
import '../providers/settings_provider.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/icon_picker_sheet.dart';
import '../widgets/unconnected_state.dart';
import 'library_blocked_section.dart';
import 'library_filter.dart';
import 'library_item_actions.dart';
import 'library_scan_actions.dart';
import 'library_sheets.dart';
import 'library_toolbar.dart';
import 'library_views.dart';

class LibraryTab extends ConsumerStatefulWidget {
  const LibraryTab({super.key});

  @override
  ConsumerState<LibraryTab> createState() => _LibraryTabState();
}

class _LibraryTabState extends ConsumerState<LibraryTab>
    with AutomaticKeepAliveClientMixin {
  final _searchController = TextEditingController();
  final _scrollController = ScrollController();
  Timer? _searchDebounce;
  LibraryFilter _filter = LibraryFilter.common;
  final Set<String> _launching = <String>{};
  final Map<String, int> _iconRevisions = <String, int>{};
  bool _scanning = false;

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  void _onScroll() {
    final kind = _filter.serverKind;
    if (kind == null || !_scrollController.hasClients) return;
    if (_scrollController.position.extentAfter > 300) return;
    final state = ref.read(libraryControllerProvider);
    if (state.loadingMore || state.loading) return;
    if (!state.hasMore) return;
    ref.read(libraryControllerProvider.notifier).refresh(kind: kind, append: true);
  }

  @override
  void dispose() {
    _searchDebounce?.cancel();
    _searchController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _switchFilter(LibraryFilter filter) {
    if (_filter == filter) return;
    setState(() {
      _filter = filter;
      _searchController.clear();
    });
    final kind = filter.serverKind;
    if (kind != null) {
      ref.read(libraryControllerProvider.notifier).refresh(kind: kind);
    }
    if (filter == LibraryFilter.recent) {
      ref.read(libraryControllerProvider.notifier).refreshRecent();
    }
  }

  void _clearSearch() {
    _searchController.clear();
    _search('');
  }

  Future<void> _load() async {
    // 常用（我的库）现在由服务端拥有，所以每次进「库」都先同步它。
    await ref.read(favoritesProvider.notifier).refresh();
    final kind = _filter.serverKind;
    await Future.wait([
      if (kind != null) ref.read(libraryControllerProvider.notifier).refresh(kind: kind),
      ref.read(libraryControllerProvider.notifier).refreshRecent(),
    ]);
  }

  void _search(String value) {
    final kind = _filter.serverKind;
    if (kind == null) return;
    _searchDebounce?.cancel();
    _searchDebounce = Timer(const Duration(milliseconds: 300), () {
      ref
          .read(libraryControllerProvider.notifier)
          .refresh(
            kind: kind,
            query: value.trim().isEmpty ? null : value.trim(),
          );
    });
  }

  // ========== 启动 ==========

  Future<void> _launchCatalogItem(LauncherItem item) async {
    if (_launching.contains(item.id)) return;
    setState(() => _launching.add(item.id));
    try {
      await ref.read(launcherControllerProvider).launch(item.id);
      await ref.read(settingsProvider.notifier).setLastUsedLauncher(item.id);
      if (mounted) context.showSuccessSnackBar('正在启动 ${item.name}');
    } catch (error) {
      if (mounted) context.showErrorSnackBar('启动失败: $error');
    } finally {
      if (mounted) setState(() => _launching.remove(item.id));
    }
  }

  // ========== 常用管理（数据存在 PC 上） ==========

  /// 条目管理动作：只依赖 `ref` 与 `_load`，实现见 [LibraryItemActions]。
  LibraryItemActions get _itemActions =>
      LibraryItemActions(ref: ref, reload: _load);

  /// 添加 / 扫描动作：扫描标志仍由本状态持有，用来禁用「＋」并显示进度条。
  LibraryScanActions get _scanActions => LibraryScanActions(
    ref: ref,
    reload: _load,
    scanning: () => _scanning,
    onScanningChanged: (value) {
      if (mounted) setState(() => _scanning = value);
    },
    serverKind: () => _filter.serverKind,
  );

  /// 目录条目的管理菜单。启动本身是整行点击，所以这里只放"改这个条目"的动作。
  Future<void> _showCatalogMenu(LauncherItem item) async {
    final added = ref.read(favoritesProvider).items.any((e) => e.id == item.id);
    final action = await showLibraryCatalogMenu(
      context,
      item: item,
      added: added,
    );
    if (action == null) return;
    if (!mounted) return;
    final actions = _itemActions;
    switch (action) {
      case LibraryCatalogAction.add:
        return actions.addToFavorites(context, item);
      case LibraryCatalogAction.unadd:
        return actions.removeFromFavorites(context, item);
      case LibraryCatalogAction.icon:
        return _openIconPicker(item);
      case LibraryCatalogAction.rename:
        return actions.renameItem(context, item);
      case LibraryCatalogAction.args:
        return actions.editLaunchArgs(context, item);
      case LibraryCatalogAction.block:
        return actions.blockItem(
          context,
          item,
          serverKind: () => _filter.serverKind,
        );
      case LibraryCatalogAction.delete:
        return actions.deleteItem(context, item);
    }
  }

  /// 常用条目的管理菜单。条目本身就是目录条目，所以动作直接落到 PC。
  Future<void> _showFavoriteMenu(LauncherItem item) async {
    final action = await showLibraryFavoriteMenu(context, item: item);
    if (action == null) return;
    if (!mounted) return;
    final actions = _itemActions;
    switch (action) {
      case LibraryFavoriteAction.home:
        return actions.toggleHome(context, item);
      case LibraryFavoriteAction.rename:
        return actions.renameItem(context, item);
      case LibraryFavoriteAction.args:
        return actions.editLaunchArgs(context, item);
      case LibraryFavoriteAction.remove:
        return actions.removeFromFavorites(context, item, confirm: true);
      case LibraryFavoriteAction.icon:
        return _openIconPicker(item);
    }
  }

  /// 打开图标修复面板：自动重扫 + 候选文件 + 手动路径。
  Future<void> _openIconPicker(LauncherItem item) async {
    final changed = await showIconPickerSheet(context, item: item);
    if (!mounted || !changed) return;
    setState(() {
      _iconRevisions[item.id] = (_iconRevisions[item.id] ?? 0) + 1;
    });
  }

  // ========== 添加 / 扫描 ==========

  Future<void> _showAddMenu() async {
    final action = await showLibraryAddMenu(context);
    if (action == null) return;
    if (!mounted) return;
    final scans = _scanActions;
    switch (action) {
      case LibraryAddAction.rescan:
        return scans.scanAll(context);
      case LibraryAddAction.folderGame:
        return scans.scanFolder(context, kind: 'game');
      case LibraryAddAction.folderApp:
        return scans.scanFolder(context, kind: 'app');
      case LibraryAddAction.file:
        return scans.addFromFile(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    if (!ref.watch(connectionProvider.select((state) => state.isAuthenticated))) {
      return const UnconnectedState();
    }
    // Keep the library isolated from high-frequency connection/foreground
    // updates. Watching the whole remote state rebuilds every visible row on
    // each heartbeat even when the catalog has not changed.
    final libraryState = ref.watch(libraryControllerProvider);
    final favorites = ref.watch(favoritesProvider);

    return Column(
      children: [
        LibraryToolbar(
          filter: _filter,
          controller: _searchController,
          scanning: _scanning,
          loading: libraryState.loading,
          onFilterChanged: _switchFilter,
          onSearchChanged: _search,
          onSearchCleared: _clearSearch,
          onAddPressed: _showAddMenu,
        ),
        const SizedBox(height: 12),
        Expanded(child: _buildContent(libraryState, favorites)),
      ],
    );
  }

  /// 列表视图需要的宿主动作；每次 build 装配一次，回调本身很轻。
  LibraryViewActions get _viewActions {
    final actions = _itemActions;
    return LibraryViewActions(
      isLaunching: _launching.contains,
      iconRevision: (id) => _iconRevisions[id] ?? 0,
      onLaunch: _launchCatalogItem,
      onCatalogMenu: _showCatalogMenu,
      onFavoriteMenu: _showFavoriteMenu,
      onAdd: (item) => actions.addToFavorites(context, item),
      onRemove: (item) => actions.removeFromFavorites(context, item),
    );
  }

  Widget _buildContent(LibraryState libraryState, FavoritesState favorites) {
    switch (_filter) {
      case LibraryFilter.common:
        return LibraryCommonView(
          favorites: favorites,
          actions: _viewActions,
          onRefresh: _load,
        );
      case LibraryFilter.recent:
        return LibraryRecentView(
          items: libraryState.recent,
          favorites: favorites,
          actions: _viewActions,
          onRefresh: _load,
        );
      case LibraryFilter.blocked:
        return LibraryBlockedSection(
          iconRevision: (id) => _iconRevisions[id] ?? 0,
        );
      case LibraryFilter.software:
      case LibraryFilter.games:
        return LibraryCatalogView(
          items: libraryState.items,
          favorites: favorites,
          filter: _filter,
          hasMore: libraryState.hasMore,
          loading: libraryState.loading,
          controller: _scrollController,
          actions: _viewActions,
          onRefresh: _load,
        );
    }
  }
}
