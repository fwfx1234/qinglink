import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/favorites_controller.dart';
import '../providers/file_controller.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import '../widgets/file_picker_sheet.dart';

/// 扫描目录时只看推荐项还是看全部候选。
enum _CandidateFilter { recommended, all }

/// Dedicated flow for scanning a folder and reviewing executable candidates.
///
/// 之前这个流程有两个让人放弃的问题：
///  1. 扫描出的所有 exe 默认全选，一个游戏目录能勾出几十个崩溃上报/安装器；
///  2. 加入库时一律归类为"游戏"，扫一遍软件目录就把游戏列表灌满。
/// 现在默认只勾选每个目录里最像主程序的那一个，并且由用户先选类型。
class ScanFolderScreen extends ConsumerStatefulWidget {
  const ScanFolderScreen({super.key, this.initialKind = 'game'});

  /// `game` 或 `app`。
  final String initialKind;

  @override
  ConsumerState<ScanFolderScreen> createState() => _ScanFolderScreenState();
}

class _ScanFolderScreenState extends ConsumerState<ScanFolderScreen> {
  String? _folder;
  String _kind = 'game';
  List<LauncherItem> _candidates = const [];
  final Set<String> _selected = <String>{};
  final Map<String, TextEditingController> _names = {};
  _CandidateFilter _viewFilter = _CandidateFilter.recommended;
  bool _loading = false;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _kind = widget.initialKind == 'app' ? 'app' : 'game';
  }

  @override
  void dispose() {
    for (final controller in _names.values) {
      controller.dispose();
    }
    super.dispose();
  }

  List<LauncherItem> get _visibleCandidates => _viewFilter == _CandidateFilter.all
      ? _candidates
      : _candidates.where((item) => item.recommended).toList(growable: false);

  int get _recommendedCount =>
      _candidates.where((item) => item.recommended).length;

  Future<void> _chooseFolder() async {
    final folder = await pickFolder(context, ref);
    if (folder == null || !mounted) return;
    setState(() {
      _folder = folder;
      _candidates = const [];
      _selected.clear();
      _viewFilter = _CandidateFilter.recommended;
      for (final controller in _names.values) {
        controller.dispose();
      }
      _names.clear();
    });
    await _scan(folder);
  }

  Future<void> _scan(String folder) async {
    if (!mounted) return;
    setState(() => _loading = true);
    GameScanCandidatesResponse? result;
    try {
      result = await ref
          .read(fileControllerProvider)
          .scanDirectory(folder, kind: _kind);
    } catch (error) {
      if (!mounted) return;
      setState(() => _loading = false);
      context.showErrorSnackBar('扫描失败: $error');
      return;
    }
    if (!mounted) return;
    setState(() => _loading = false);
    if (result == null) {
      context.showErrorSnackBar('扫描失败，请检查目录权限');
      return;
    }
    final candidates = result.candidates;
    // 默认只勾选推荐项：每个目录挑一个最像主程序的 exe。
    final recommended = candidates
        .where((item) => item.recommended)
        .map((item) => item.executablePath)
        .toSet();
    for (final controller in _names.values) {
      controller.dispose();
    }
    setState(() {
      _candidates = candidates;
      _selected
        ..clear()
        ..addAll(recommended);
      // 没有任何推荐项时（例如目录里全是平铺的软件），退回全选，避免空选。
      if (recommended.isEmpty) {
        _selected.addAll(candidates.map((item) => item.executablePath));
      }
      _viewFilter = recommended.isEmpty
          ? _CandidateFilter.all
          : _CandidateFilter.recommended;
      _names
        ..clear()
        ..addEntries(
          candidates.map(
            (item) => MapEntry(
              item.executablePath,
              TextEditingController(text: item.name),
            ),
          ),
        );
    });
    if (candidates.isEmpty) {
      context.showInfoSnackBar('该文件夹内没有可添加的程序');
    }
  }

  Future<void> _save() async {
    final folder = _folder;
    final selected = _candidates
        .where((item) => _selected.contains(item.executablePath))
        .toList();
    if (folder == null || selected.isEmpty) return;
    FocusManager.instance.primaryFocus?.unfocus();
    setState(() => _saving = true);
    final files = ref.read(fileControllerProvider);
    final paths = selected.map((item) => item.executablePath).toList();
    final names = selected
        .map((item) => _names[item.executablePath]?.text.trim() ?? item.name)
        .toList(growable: false);
    var added = 0;
    try {
      // 批量提交：扫描和数据库写入都只在 PC 上做一次，服务端也不会再重扫目录。
      final addedItems = await files.addScannedItems(
        folder,
        paths,
        names: names,
        kind: _kind,
      );
      if (addedItems.length != selected.length) {
        throw StateError('电脑未能添加所选程序');
      }
      for (final catalogItem in addedItems) {
        // 常用数据存在服务端：这里只是把它标记为常用，不再往手机写本地副本。
        await ref.read(favoritesProvider.notifier).add(catalogItem.id);
        added++;
      }
    } catch (error) {
      if (mounted) context.showErrorSnackBar('添加失败: $error');
      if (mounted) setState(() => _saving = false);
      return;
    }
    if (!mounted) return;
    setState(() => _saving = false);
    if (added == 0) {
      context.showErrorSnackBar('没有成功添加程序');
      return;
    }
    context.showSuccessSnackBar('已添加 $added 个程序到常用');
    Navigator.of(context).pop(true);
  }

  @override
  Widget build(BuildContext context) {
    final visible = _visibleCandidates;
    final allSelected =
        visible.isNotEmpty && visible.every((item) => _selected.contains(item.executablePath));
    return Scaffold(
      resizeToAvoidBottomInset: true,
      appBar: AppBar(title: Text(_kind == 'game' ? '扫描游戏文件夹' : '扫描软件文件夹')),
      body: Column(
        children: [
          _buildKindSelector(),
          _buildFolderCard(),
          if (_loading) const LinearProgressIndicator(minHeight: 2),
          if (_candidates.isNotEmpty) ...[
            _buildScanSummaryRow(allSelected, visible),
            _buildViewFilterRow(),
          ],
          Expanded(
            child: visible.isEmpty
                ? _buildEmptyCandidateHint()
                : _buildCandidateList(visible),
          ),
          if (_candidates.isNotEmpty) _buildSaveBar(),
        ],
      ),
    );
  }

  /// 顶部「游戏 / 软件」类型选择行。
  Widget _buildKindSelector() {
    return Padding(
      padding: const EdgeInsets.fromLTRB(18, 8, 18, 8),
      child: Row(
        children: [
          FilterChipX(
            label: '游戏',
            active: _kind == 'game',
            onTap: () => _selectKind('game'),
          ),
          const SizedBox(width: 7),
          FilterChipX(
            label: '软件',
            active: _kind == 'app',
            onTap: () => _selectKind('app'),
          ),
        ],
      ),
    );
  }

  /// 切换扫描类型；已有目录时按新类型重新扫描。
  void _selectKind(String kind) {
    if (_kind == kind || _loading) return;
    setState(() => _kind = kind);
    final folder = _folder;
    if (folder != null) _scan(folder);
  }

  /// 目录选择卡片：显示当前目录并提供选择入口。
  Widget _buildFolderCard() {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.fromLTRB(18, 0, 18, 12),
      child: AppCard(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: [
            const TileIcon(
              icon: Icons.folder_open_rounded,
              color: AppColors.amber,
              size: 38,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                _folder ?? '先选择需要扫描的文件夹',
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: AppText.body.copyWith(color: palette.textPrimary),
              ),
            ),
            IconBtn(
              icon: Icons.folder_rounded,
              tooltip: '选择文件夹',
              onPressed: _loading || _saving ? null : _chooseFolder,
            ),
          ],
        ),
      ),
    );
  }

  /// 扫描结果概要行：数量说明与「全选 / 全不选」。
  Widget _buildScanSummaryRow(bool allSelected, List<LauncherItem> visible) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.fromLTRB(18, 0, 12, 4),
      child: Row(
        children: [
          Expanded(
            child: Text(
              '扫描到 ${_candidates.length} 个程序，推荐 $_recommendedCount 个',
              style: AppText.cardTitle.copyWith(color: palette.textPrimary),
            ),
          ),
          TextButton(
            onPressed: () => setState(() {
              if (allSelected) {
                for (final item in visible) {
                  _selected.remove(item.executablePath);
                }
              } else {
                _selected.addAll(visible.map((e) => e.executablePath));
              }
            }),
            child: Text(allSelected ? '全不选' : '全选'),
          ),
        ],
      ),
    );
  }

  /// 「仅推荐 / 全部」视图过滤行。
  Widget _buildViewFilterRow() {
    return Padding(
      padding: const EdgeInsets.fromLTRB(18, 0, 18, 6),
      child: Row(
        children: [
          FilterChipX(
            label: '仅推荐',
            active: _viewFilter == _CandidateFilter.recommended,
            onTap: () =>
                setState(() => _viewFilter = _CandidateFilter.recommended),
          ),
          const SizedBox(width: 7),
          FilterChipX(
            label: '全部',
            active: _viewFilter == _CandidateFilter.all,
            onTap: () => setState(() => _viewFilter = _CandidateFilter.all),
          ),
        ],
      ),
    );
  }

  /// 候选列表为空时的占位提示。
  Widget _buildEmptyCandidateHint() {
    final palette = context.palette;
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 24),
        child: Text(
          _folder == null
              ? '选择文件夹后开始扫描'
              : _candidates.isEmpty
                  ? '没有可添加的 EXE'
                  : '这类目里没有推荐项，切到「全部」查看所有可执行文件',
          textAlign: TextAlign.center,
          style: AppText.body.copyWith(color: palette.textSubtle),
        ),
      ),
    );
  }

  /// 候选程序列表：每项含勾选框、图标、名称输入框与路径信息。
  Widget _buildCandidateList(List<LauncherItem> visible) {
    final palette = context.palette;
    return ListView.separated(
      padding: const EdgeInsets.fromLTRB(18, 0, 18, 18),
      itemCount: visible.length,
      separatorBuilder: (_, _) => const SizedBox(height: 8),
      itemBuilder: (context, index) {
        final item = visible[index];
        final path = item.executablePath;
        return AppCard(
          padding: const EdgeInsets.symmetric(
            horizontal: 8,
            vertical: 6,
          ),
          child: Row(
            children: [
              Checkbox(
                value: _selected.contains(path),
                onChanged: (value) => setState(() {
                  if (value == true) {
                    _selected.add(path);
                  } else {
                    _selected.remove(path);
                  }
                }),
              ),
              _CandidateIcon(item.iconBase64),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: TextField(
                            controller: _names[path],
                            decoration: const InputDecoration(
                              labelText: '显示名称',
                              isDense: true,
                              border: InputBorder.none,
                            ),
                          ),
                        ),
                        if (item.recommended)
                          const StatusTag('推荐'),
                      ],
                    ),
                    Text(
                      _candidateSubtitle(item),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.meta.copyWith(
                        color: palette.textSubtle,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        );
      },
    );
  }

  /// 底部提交栏：显示已选数量并提交添加。
  Widget _buildSaveBar() {
    return Padding(
      padding: const EdgeInsets.fromLTRB(18, 8, 18, 18),
      child: PrimaryButton(
        label: _selected.isEmpty
            ? '请选择程序'
            : '添加 ${_selected.length} 个程序到常用',
        icon: Icons.add_rounded,
        expand: true,
        busy: _saving,
        onPressed: _saving || _selected.isEmpty ? null : _save,
      ),
    );
  }

  String _candidateSubtitle(LauncherItem item) {
    final size = item.sizeBytes;
    if (size == null || size <= 0) return item.executablePath;
    final megabytes = size / 1024 / 1024;
    final readable = megabytes >= 1
        ? '${megabytes.toStringAsFixed(0)} MB'
        : '${(size / 1024).toStringAsFixed(0)} KB';
    return '$readable · ${item.executablePath}';
  }
}

class _CandidateIcon extends StatelessWidget {
  const _CandidateIcon(this.base64Icon);
  final String? base64Icon;

  @override
  Widget build(BuildContext context) {
    Uint8List? bytes;
    if (base64Icon != null) {
      try {
        bytes = base64Decode(base64Icon!);
      } catch (_) {}
    }
    return Container(
      width: 40,
      height: 40,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: AppColors.cyan.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(10),
      ),
      child: bytes == null
          ? const Icon(Icons.desktop_windows_outlined, color: AppColors.cyan)
          : Image.memory(
              bytes,
              fit: BoxFit.contain,
              filterQuality: FilterQuality.high,
              cacheWidth: (40 * MediaQuery.devicePixelRatioOf(context)).round(),
              errorBuilder: (_, _, _) => const Icon(
                Icons.desktop_windows_outlined,
                color: AppColors.cyan,
              ),
            ),
    );
  }
}
