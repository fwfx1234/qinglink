import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/auth_provider.dart';
import '../providers/remote_control_provider.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_icon.dart';
import '../widgets/app_ui.dart';
import '../widgets/file_picker_sheet.dart';
import '../widgets/unconnected_state.dart';
import 'scan_folder_screen.dart';

enum LibraryFilter { common, recent, steam, software, other }

extension _LibraryFilterInfo on LibraryFilter {
  /// 服务端统一库对应的分类；常用/最近在本地处理。
  String? get serverKind => switch (this) {
    LibraryFilter.steam => 'steam',
    LibraryFilter.software => 'software',
    LibraryFilter.other => 'other',
    _ => null,
  };

  String get label => switch (this) {
    LibraryFilter.common => '常用',
    LibraryFilter.recent => '最近',
    LibraryFilter.steam => 'Steam',
    LibraryFilter.software => '软件',
    LibraryFilter.other => '其他',
  };
}

/// Human readable source for a library entry.  Folder scans and manual
/// additions are the same kind of user-created entry.
String _sourceLabel(String source) {
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
bool _isUserAdded(String source) {
  final normalized = source.toLowerCase();
  return normalized == 'user' || normalized == 'custom';
}

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
    final state = ref.read(remoteControlProvider);
    if (state.isLoadingMoreLibrary || state.isLoadingLibrary) return;
    if (!state.libraryHasMore) return;
    ref
        .read(remoteControlProvider.notifier)
        .loadUnifiedLibrary(
          kind: kind,
          append: true,
          page: state.libraryPage + 1,
        );
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
      ref
          .read(remoteControlProvider.notifier)
          .loadUnifiedLibrary(kind: kind, force: true);
    }
  }

  Future<void> _load() async {
    final remote = ref.read(remoteControlProvider.notifier);
    final kind = _filter.serverKind;
    await Future.wait([
      if (kind != null) remote.loadUnifiedLibrary(kind: kind, force: true),
      remote.loadRecentUnifiedLibrary(),
    ]);
  }

  void _search(String value) {
    final kind = _filter.serverKind;
    if (kind == null) return;
    _searchDebounce?.cancel();
    _searchDebounce = Timer(const Duration(milliseconds: 300), () {
      ref
          .read(remoteControlProvider.notifier)
          .loadUnifiedLibrary(
            kind: kind,
            query: value.trim().isEmpty ? null : value.trim(),
            force: true,
          );
    });
  }

  // ========== 启动 ==========

  Future<void> _launchCatalogItem(LauncherItem item) async {
    if (_launching.contains(item.id)) return;
    setState(() => _launching.add(item.id));
    try {
      await ref.read(remoteControlProvider.notifier).launchScannerApp(item.id);
      await ref.read(settingsProvider.notifier).setLastUsedLauncher(item.id);
      if (mounted) context.showSuccessSnackBar('正在启动 ${item.name}');
    } catch (error) {
      if (mounted) context.showErrorSnackBar('启动失败: $error');
    } finally {
      if (mounted) setState(() => _launching.remove(item.id));
    }
  }

  Future<void> _launchLibraryApp(QuickApp app) async {
    if (_launching.contains(app.id)) return;
    setState(() => _launching.add(app.id));
    try {
      final remote = ref.read(remoteControlProvider.notifier);
      final scannerId = app.scannerId;
      if (scannerId != null && scannerId.isNotEmpty) {
        await remote.launchScannerApp(scannerId);
      } else {
        await remote.launchApp(app.path, args: app.args);
      }
      await ref.read(settingsProvider.notifier).setLastUsedLauncher(app.id);
      if (mounted) context.showSuccessSnackBar('正在启动 ${app.name}');
    } catch (error) {
      if (mounted) context.showErrorSnackBar('启动失败: $error');
    } finally {
      if (mounted) setState(() => _launching.remove(app.id));
    }
  }

  // ========== 常用管理 ==========

  Future<void> _addToLibrary(LauncherItem item) async {
    await ref
        .read(settingsProvider.notifier)
        .addToLibrary(
          QuickApp(
            name: item.name,
            path: item.executablePath,
            scannerId: item.id,
            isGame: item.category.toLowerCase() == 'game',
            originalName: item.originalName,
            pinnedToHome: false,
            args: item.args,
          ),
        );
    if (mounted) context.showSuccessSnackBar('已添加到常用');
  }

  /// Remove a user-added program from the PC catalog and the local 常用 list.
  Future<void> _deleteCatalogItem(LauncherItem item) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除程序'),
        content: Text('确定删除「${item.name}」吗？删除后将从库中移除。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: AppColors.red),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('删除'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    final ok = await ref
        .read(remoteControlProvider.notifier)
        .deleteScannerApp(item.id);
    await ref.read(settingsProvider.notifier).removeFromLibrary(item.id);
    if (!mounted) return;
    if (ok) {
      context.showSuccessSnackBar('已删除 ${item.name}');
      await _load();
    } else {
      context.showErrorSnackBar('删除失败');
    }
  }

  Future<void> _removeFromLibrary(QuickApp app) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('从库中移除'),
        content: Text('确定将「${app.name}」从常用中移除吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: AppColors.red),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('移除'),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await ref.read(settingsProvider.notifier).removeFromLibrary(app.id);
    }
  }

  /// 目录条目上的「已添加」标签：再次点击即取消添加。
  Future<void> _removeAddedCatalogItem(LauncherItem item) async {
    await ref.read(settingsProvider.notifier).removeFromLibrary(item.id);
    if (mounted) context.showInfoSnackBar('已取消添加 ${item.name}');
  }

  Future<void> _toggleHome(QuickApp app) async {
    await ref
        .read(settingsProvider.notifier)
        .setPinnedToHome(app.id, !app.pinnedToHome);
    if (mounted) {
      context.showSuccessSnackBar(app.pinnedToHome ? '已从首页取消置顶' : '已置顶到首页');
    }
  }

  Future<String?> _promptRename(String current) async {
    final controller = TextEditingController(text: current);
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('重命名'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(labelText: '显示名称'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    controller.dispose();
    final trimmed = name?.trim();
    if (trimmed == null || trimmed.isEmpty || trimmed == current) return null;
    return trimmed;
  }

  Future<void> _renameLibraryApp(QuickApp app) async {
    final newName = await _promptRename(app.name);
    if (newName == null) return;
    final startedAt = DateTime.now();
    debugPrint('[QingLink][rename] 开始 id=${app.id} newName=$newName');
    await ref.read(settingsProvider.notifier).renameLibraryApp(app.id, newName);
    debugPrint(
      '[QingLink][rename] 本地保存完成 +${DateTime.now().difference(startedAt).inMilliseconds}ms',
    );
    final scannerId = app.scannerId;
    if (scannerId != null && scannerId.isNotEmpty) {
      try {
        await ref
            .read(remoteControlProvider.notifier)
            .renameScannerApp(scannerId, app.originalName ?? app.name, newName);
        debugPrint(
          '[QingLink][rename] PC 端完成 +${DateTime.now().difference(startedAt).inMilliseconds}ms',
        );
      } catch (error) {
        debugPrint(
          '[QingLink][rename] PC 端失败 +${DateTime.now().difference(startedAt).inMilliseconds}ms error=$error',
        );
        if (mounted) context.showWarningSnackBar('PC 端重命名失败: $error');
      }
    }
  }

  /// Rename a PC catalog entry (used by 软件 / 其他 tabs). Also updates the
  /// local 常用 entry when present.
  Future<void> _renameCatalogItem(LauncherItem item) async {
    final newName = await _promptRename(item.name);
    if (newName == null) return;
    try {
      await ref
          .read(remoteControlProvider.notifier)
          .renameScannerApp(item.id, item.originalName, newName);
    } catch (error) {
      if (mounted) context.showErrorSnackBar('重命名失败: $error');
      return;
    }
    if (ref.read(settingsProvider).isInLibrary(item.id)) {
      await ref
          .read(settingsProvider.notifier)
          .renameLibraryApp(item.id, newName);
    }
    if (!mounted) return;
    context.showSuccessSnackBar('已重命名为 $newName');
    await _load();
  }

  /// Ask the PC to re-extract and persist an entry's icon, then reload it.
  Future<void> _refreshCatalogIcon(LauncherItem item) async {
    try {
      await ref
          .read(remoteControlProvider.notifier)
          .refreshScannerAppIcon(item.id);
    } catch (error) {
      if (mounted) context.showErrorSnackBar('刷新图标失败: $error');
      return;
    }
    if (!mounted) return;
    setState(() {
      _iconRevisions[item.id] = (_iconRevisions[item.id] ?? 0) + 1;
    });
    context.showSuccessSnackBar('已重新扫描图标');
  }

  static List<String> _parseArgs(String raw) {
    final tokens = <String>[];
    final buffer = StringBuffer();
    var inQuotes = false;
    for (var i = 0; i < raw.length; i++) {
      final ch = raw[i];
      if (ch == '"') {
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
    if (buffer.isNotEmpty) tokens.add(buffer.toString());
    return tokens;
  }

  static String _formatArgs(List<String> args) =>
      args.map((arg) => arg.contains(' ') ? '"$arg"' : arg).join(' ');

  /// Edit the extra launch arguments for a scanner entry, e.g.
  /// `-start steam://open/bigpicture`.
  Future<void> _editCatalogArgs(LauncherItem item) async {
    final controller = TextEditingController(text: _formatArgs(item.args));
    final value = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('启动参数'),
        content: TextField(
          controller: controller,
          autofocus: true,
          minLines: 1,
          maxLines: 3,
          decoration: const InputDecoration(
            labelText: '参数（空格分隔，可用引号）',
            hintText: '例如 -start steam://open/bigpicture',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (value == null || !mounted) return;
    final args = _parseArgs(value);
    final ok = await ref
        .read(remoteControlProvider.notifier)
        .updateScannerApp(
          id: item.id,
          name: item.name,
          exePath: item.executablePath,
          args: args,
        );
    if (!mounted) return;
    if (!ok) {
      context.showErrorSnackBar('保存启动参数失败');
      return;
    }
    if (ref.read(settingsProvider).isInLibrary(item.id)) {
      await ref
          .read(settingsProvider.notifier)
          .setLibraryArgs(item.id, args);
    }
    if (!mounted) return;
    context.showSuccessSnackBar(
      args.isEmpty ? '已清除启动参数' : '已保存启动参数',
    );
    await _load();
  }

  Future<void> _showCatalogMenu(LauncherItem item) async {
    final action = await showModalBottomSheet<String>(
      context: context,
      showDragHandle: true,
      builder: (ctx) => SafeArea(
        child: Wrap(
          children: [
            ListTile(
              leading: const Icon(Icons.drive_file_rename_outline),
              title: const Text('重命名'),
              onTap: () => Navigator.pop(ctx, 'rename'),
            ),
            ListTile(
              leading: const Icon(Icons.image_outlined),
              title: const Text('重新扫描图标'),
              onTap: () => Navigator.pop(ctx, 'icon'),
            ),
            ListTile(
              leading: const Icon(Icons.tune_rounded),
              title: const Text('启动参数'),
              subtitle: Text(
                item.args.isEmpty ? '未设置' : item.args.join(' '),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              onTap: () => Navigator.pop(ctx, 'args'),
            ),
          ],
        ),
      ),
    );
    if (!mounted || action == null) return;
    if (action == 'rename') return _renameCatalogItem(item);
    if (action == 'icon') return _refreshCatalogIcon(item);
    if (action == 'args') return _editCatalogArgs(item);
  }

  Future<void> _showLibraryMenu(QuickApp app) async {
    final action = await showModalBottomSheet<String>(
      context: context,
      showDragHandle: true,
      builder: (ctx) => SafeArea(
        child: Wrap(
          children: [
            ListTile(
              leading: Icon(
                app.pinnedToHome
                    ? Icons.push_pin_rounded
                    : Icons.push_pin_outlined,
              ),
              title: Text(app.pinnedToHome ? '取消置顶到首页' : '置顶到首页'),
              onTap: () => Navigator.pop(ctx, 'home'),
            ),
            ListTile(
              leading: const Icon(Icons.drive_file_rename_outline),
              title: const Text('重命名'),
              onTap: () => Navigator.pop(ctx, 'rename'),
            ),
            ListTile(
              leading: const Icon(Icons.delete_outline, color: AppColors.red),
              title: const Text('从库中移除'),
              onTap: () => Navigator.pop(ctx, 'remove'),
            ),
          ],
        ),
      ),
    );
    if (!mounted || action == null) return;
    if (action == 'home') return _toggleHome(app);
    if (action == 'rename') return _renameLibraryApp(app);
    if (action == 'remove') return _removeFromLibrary(app);
  }

  // ========== 添加 / 扫描 ==========

  Future<void> _addFromFile() async {
    final path = await pickExecutable(context, ref);
    if (path == null || !mounted) return;
    final base = path.split(RegExp(r'[\\/]')).last;
    final suggested = base.toLowerCase().endsWith('.exe')
        ? base.substring(0, base.length - 4)
        : base;

    final controller = TextEditingController(text: suggested);
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('添加到库'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(labelText: '显示名称'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: const Text('添加'),
          ),
        ],
      ),
    );
    controller.dispose();
    final finalName = name?.trim();
    if (finalName == null || finalName.isEmpty || !mounted) return;

    final id = await ref
        .read(remoteControlProvider.notifier)
        .addScannerApp(name: finalName, exePath: path);
    if (!mounted) return;
    if (id == null) {
      context.showErrorSnackBar('添加失败');
      return;
    }
    await ref
        .read(settingsProvider.notifier)
        .addToLibrary(
          QuickApp(
            name: finalName,
            path: path,
            scannerId: id,
            originalName: finalName,
            pinnedToHome: false,
          ),
        );
    await _load();
    if (mounted) context.showSuccessSnackBar('已添加 $finalName');
  }

  Future<void> _scanAll() async {
    if (_scanning) return;
    setState(() => _scanning = true);
    try {
      final remote = ref.read(remoteControlProvider.notifier);
      await remote.refreshLauncherCatalog('app');
      await remote.refreshLauncherCatalog('game');
      if (mounted) {
        context.showSuccessSnackBar('已开始扫描：开始菜单 + Steam');
      }
    } catch (error) {
      if (mounted) context.showErrorSnackBar('扫描失败: $error');
    } finally {
      if (mounted) setState(() => _scanning = false);
    }
  }

  Future<void> _scanFolder() async {
    await Navigator.of(
      context,
    ).push<bool>(MaterialPageRoute(builder: (_) => const ScanFolderScreen()));
    if (!mounted) return;
    await _load();
  }

  Future<void> _showAddMenu() async {
    final action = await showModalBottomSheet<String>(
      context: context,
      showDragHandle: true,
      builder: (ctx) => SafeArea(
        child: Wrap(
          children: [
            ListTile(
              leading: const Icon(Icons.refresh_rounded),
              title: const Text('重新扫描'),
              subtitle: const Text('开始菜单 + Steam'),
              onTap: () => Navigator.pop(ctx, 'rescan'),
            ),
            ListTile(
              leading: const Icon(Icons.create_new_folder_outlined),
              title: const Text('扫描文件夹'),
              subtitle: const Text('选择目录，批量添加其中的程序'),
              onTap: () => Navigator.pop(ctx, 'folder'),
            ),
            ListTile(
              leading: const Icon(Icons.desktop_windows_outlined),
              title: const Text('选择程序文件'),
              subtitle: const Text('从文件树选择单个 .exe'),
              onTap: () => Navigator.pop(ctx, 'file'),
            ),
          ],
        ),
      ),
    );
    if (!mounted || action == null) return;
    if (action == 'rescan') return _scanAll();
    if (action == 'folder') return _scanFolder();
    if (action == 'file') return _addFromFile();
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    if (!ref.watch(authProvider.select((state) => state.isAuthenticated))) {
      return const UnconnectedState();
    }
    final state = ref.watch(remoteControlProvider);
    final settings = ref.watch(settingsProvider);

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 14, 18, 0),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                for (final filter in LibraryFilter.values) ...[
                  if (filter != LibraryFilter.values.first)
                    const SizedBox(width: 7),
                  FilterChipX(
                    label: filter.label,
                    active: _filter == filter,
                    onTap: () => _switchFilter(filter),
                  ),
                ],
              ],
            ),
          ),
        ),
        if (_filter.serverKind != null) ...[
          const SizedBox(height: 12),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 18),
            child: Row(
              children: [
                Expanded(
                  child: SearchField(
                    controller: _searchController,
                    hint: '搜索程序',
                    onChanged: (v) {
                      _search(v);
                      setState(() {});
                    },
                    onClear: () {
                      _searchController.clear();
                      _search('');
                      setState(() {});
                    },
                  ),
                ),
                const SizedBox(width: 8),
                IconBtn(
                  icon: Icons.add_rounded,
                  tooltip: '添加 / 扫描',
                  onPressed: _scanning ? null : _showAddMenu,
                ),
              ],
            ),
          ),
          if (_scanning || state.isLoadingLibrary)
            const Padding(
              padding: EdgeInsets.only(top: 12),
              child: LinearProgressIndicator(
                minHeight: 2,
                color: AppColors.primary,
              ),
            ),
        ],
        const SizedBox(height: 12),
        Expanded(child: _buildContent(state, settings)),
      ],
    );
  }

  Widget _buildContent(RemoteControlState state, ConnectionConfig settings) {
    switch (_filter) {
      case LibraryFilter.common:
        return _buildCommon(settings);
      case LibraryFilter.recent:
        return _buildRecent(state, settings);
      case LibraryFilter.steam:
      case LibraryFilter.software:
      case LibraryFilter.other:
        return _buildCatalog(state, settings);
    }
  }

  Widget _buildCommon(ConnectionConfig settings) {
    final apps = settings.quickApps;
    if (apps.isEmpty) {
      return const EmptyState(
        icon: Icons.library_add_outlined,
        title: '常用还是空的',
        message: '去「软件」或「其他」里挑选程序加入常用。',
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      color: AppColors.primary,
      child: ListView.separated(
        padding: const EdgeInsets.fromLTRB(18, 0, 18, 96),
        itemCount: apps.length,
        separatorBuilder: (_, _) => const SizedBox(height: 8),
        itemBuilder: (context, index) {
          final app = apps[index];
          return _LibraryAppTile(
            app: app,
            launching: _launching.contains(app.id),
            onLaunch: () => _launchLibraryApp(app),
            onToggleHome: () => _toggleHome(app),
            onMenu: () => _showLibraryMenu(app),
          );
        },
      ),
    );
  }

  Widget _buildRecent(RemoteControlState state, ConnectionConfig settings) {
    final items = state.recentLauncherItems;
    if (items.isEmpty) {
      return const EmptyState(
        icon: Icons.history_rounded,
        title: '还没有最近使用',
        message: '启动程序后会出现在这里。',
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      color: AppColors.primary,
      child: ListView.separated(
        padding: const EdgeInsets.fromLTRB(18, 0, 18, 96),
        itemCount: items.length,
        separatorBuilder: (_, _) => const SizedBox(height: 8),
        itemBuilder: (context, index) {
          final item = items[index];
          return _CatalogTile(
            item: item,
            added: settings.isInLibrary(item.id),
            launching: _launching.contains(item.id),
            iconRevision: _iconRevisions[item.id] ?? 0,
            onMenu: () => _showCatalogMenu(item),
            onLaunch: () => _launchCatalogItem(item),
            onAdd: () => _addToLibrary(item),
            onRemove: () => _removeAddedCatalogItem(item),
            onDelete: _isUserAdded(item.source)
                ? () => _deleteCatalogItem(item)
                : null,
          );
        },
      ),
    );
  }

  Widget _buildCatalog(RemoteControlState state, ConnectionConfig settings) {
    final items = state.launcherItems;
    if (items.isEmpty) {
      return EmptyState(
        icon: _filter == LibraryFilter.steam
            ? Icons.sports_esports_outlined
            : Icons.search_off_rounded,
        title: state.isLoadingLibrary ? '正在读取…' : '没有找到程序',
        message: switch (_filter) {
          LibraryFilter.steam => '未检测到 Steam 游戏，可点击右上角「＋」重新扫描。',
          LibraryFilter.software => '试试搜索，或用右上角「＋」扫描开始菜单。',
          LibraryFilter.other => '暂无扫描或手动添加的条目，「＋」→ 扫描文件夹。',
          _ => '试试搜索，或用右上角「＋」扫描。',
        },
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      color: AppColors.primary,
      child: ListView.separated(
        controller: _scrollController,
        padding: const EdgeInsets.fromLTRB(18, 0, 18, 96),
        itemCount: items.length + (state.libraryHasMore ? 1 : 0),
        separatorBuilder: (_, _) => const SizedBox(height: 8),
        itemBuilder: (context, index) {
          if (index >= items.length) {
            return const Padding(
              padding: EdgeInsets.all(16),
              child: Center(
                child: SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            );
          }
          final item = items[index];
          return _CatalogTile(
            item: item,
            added: settings.isInLibrary(item.id),
            launching: _launching.contains(item.id),
            iconRevision: _iconRevisions[item.id] ?? 0,
            onMenu: () => _showCatalogMenu(item),
            onLaunch: () => _launchCatalogItem(item),
            onAdd: () => _addToLibrary(item),
            onRemove: () => _removeAddedCatalogItem(item),
            onDelete: _isUserAdded(item.source)
                ? () => _deleteCatalogItem(item)
                : null,
          );
        },
      ),
    );
  }
}

/// 常用条目：置顶、重命名、移除、启动。
class _LibraryAppTile extends StatelessWidget {
  const _LibraryAppTile({
    required this.app,
    required this.launching,
    required this.onLaunch,
    required this.onToggleHome,
    required this.onMenu,
  });

  final QuickApp app;
  final bool launching;
  final VoidCallback onLaunch;
  final VoidCallback onToggleHome;
  final VoidCallback onMenu;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 68,
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      onTap: launching ? null : onLaunch,
      child: Row(
        children: [
          AppIcon(id: app.scannerId ?? app.path, size: 40, radius: 10),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  app.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.cardTitle.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 3),
                Text(
                  app.pinnedToHome ? '已置顶到首页' : '常用',
                  style: AppText.meta.copyWith(
                    color: app.pinnedToHome
                        ? AppColors.primary
                        : palette.textSubtle,
                  ),
                ),
              ],
            ),
          ),
          IconBtn(
            icon: app.pinnedToHome
                ? Icons.push_pin_rounded
                : Icons.push_pin_outlined,
            tooltip: app.pinnedToHome ? '取消置顶' : '置顶到首页',
            size: 38,
            iconSize: 17,
            radius: 9,
            color: app.pinnedToHome ? AppColors.amber : palette.textMuted,
            onPressed: onToggleHome,
          ),
          IconBtn(
            icon: Icons.more_horiz_rounded,
            tooltip: '管理',
            size: 38,
            iconSize: 17,
            radius: 9,
            onPressed: onMenu,
          ),
          if (launching)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 11),
              child: SizedBox(
                width: 16,
                height: 16,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            )
          else
            IconBtn(
              icon: Icons.play_arrow_rounded,
              tooltip: '启动',
              size: 38,
              iconSize: 18,
              radius: 9,
              color: AppColors.primary,
              background: AppColors.primary.withValues(alpha: 0.13),
              onPressed: onLaunch,
            ),
        ],
      ),
    );
  }
}

/// 全部 / 最近条目：启动按钮启动，已添加标签点击后取消添加。
class _CatalogTile extends StatelessWidget {
  const _CatalogTile({
    required this.item,
    required this.added,
    required this.launching,
    required this.onLaunch,
    required this.onAdd,
    this.onRemove,
    this.onDelete,
    this.onMenu,
    this.iconRevision = 0,
  });

  final LauncherItem item;
  final bool added;
  final bool launching;
  final VoidCallback onLaunch;
  final VoidCallback onAdd;
  final VoidCallback? onRemove;
  final VoidCallback? onDelete;
  final VoidCallback? onMenu;
  final int iconRevision;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 68,
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      child: Row(
        children: [
          AppIcon(
            id: item.id,
            size: 40,
            radius: 10,
            revision: iconRevision,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  item.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.cardTitle.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 3),
                Text(
                  _sourceLabel(item.source),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          if (launching)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 11),
              child: SizedBox(
                width: 16,
                height: 16,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            )
          else
            IconBtn(
              icon: Icons.play_arrow_rounded,
              tooltip: '启动',
              size: 38,
              iconSize: 18,
              radius: 9,
              color: AppColors.primary,
              background: AppColors.primary.withValues(alpha: 0.13),
              onPressed: onLaunch,
            ),
          if (added)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4),
              child: Tooltip(
                message: '点击取消添加',
                child: GestureDetector(
                  onTap: launching ? null : onRemove,
                  behavior: HitTestBehavior.opaque,
                  child: const StatusTag('已添加'),
                ),
              ),
            )
          else
            Padding(
              padding: const EdgeInsets.only(left: 6),
              child: IconBtn(
                icon: Icons.add_rounded,
                tooltip: '添加到常用',
                size: 38,
                iconSize: 18,
                radius: 9,
                color: palette.textPrimary,
                onPressed: onAdd,
              ),
            ),
          if (onDelete != null)
            Padding(
              padding: const EdgeInsets.only(left: 2),
              child: IconBtn(
                icon: Icons.delete_outline_rounded,
                tooltip: '删除',
                size: 38,
                iconSize: 18,
                radius: 9,
                color: AppColors.red,
                onPressed: onDelete,
              ),
            ),
          if (onMenu != null)
            Padding(
              padding: const EdgeInsets.only(left: 2),
              child: IconBtn(
                icon: Icons.more_horiz_rounded,
                tooltip: '管理',
                size: 38,
                iconSize: 17,
                radius: 9,
                onPressed: onMenu,
              ),
            ),
        ],
      ),
    );
  }
}
