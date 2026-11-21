/// S12 扫描结果（对应设计稿 `S12-scan-result-*.png`）。
///
/// 结构：顶栏（返回 + 标题 + 全选）→ 目录与统计卡 → 推荐/全部分段 →
/// 结果卡（复选 + 图标 + 名称 + exe 文件名与推荐标记）→ 底部「加入应用库」。
///
/// 默认只勾选推荐项：每个目录挑一个最像主程序的 exe，避免把崩溃上报、
/// 安装器之类的可执行文件一起加入库。
library;

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

  /// 长按改名后的显示名称（key 为 exe 路径）；不改动服务端数据，提交时再带上。
  final Map<String, String> _renamed = <String, String>{};
  _CandidateFilter _viewFilter = _CandidateFilter.recommended;
  bool _loading = false;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _kind = widget.initialKind == 'app' ? 'app' : 'game';
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
      _renamed.clear();
      _viewFilter = _CandidateFilter.recommended;
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
    final recommended = candidates
        .where((item) => item.recommended)
        .map((item) => item.executablePath)
        .toSet();
    setState(() {
      _candidates = candidates;
      _renamed.clear();
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
    });
    if (candidates.isEmpty) {
      context.showInfoSnackBar('该文件夹内没有可添加的程序');
    }
  }

  /// 长按改名：加入库前先调整显示名称。
  Future<void> _rename(LauncherItem item) async {
    final controller = TextEditingController(text: item.name);
    final name = await showDialog<String>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('重命名'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(labelText: '显示名称'),
          onSubmitted: (value) =>
              Navigator.of(dialogContext).pop(value.trim()),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () =>
                Navigator.of(dialogContext).pop(controller.text.trim()),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (name == null || name.isEmpty || !mounted) return;
    setState(() => _renamed[item.executablePath] = name);
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
        .map((item) => _renamed[item.executablePath] ?? item.name)
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
    final palette = context.palette;
    final visible = _visibleCandidates;
    final allSelected =
        visible.isNotEmpty &&
        visible.every((item) => _selected.contains(item.executablePath));
    return Scaffold(
      backgroundColor: palette.background,
      body: SafeArea(
        child: Column(
          children: [
            _buildNavBar(allSelected, visible),
            if (_loading)
              LinearProgressIndicator(
                minHeight: 2,
                color: palette.brand,
                backgroundColor: Colors.transparent,
              ),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.fromLTRB(
                  AppMetrics.pagePadding,
                  0,
                  AppMetrics.pagePadding,
                  AppSpacing.lg,
                ),
                children: [
                  _buildFolderCard(),
                  if (_candidates.isNotEmpty) ...[
                    const SizedBox(height: AppSpacing.md),
                    _FilterSegment(
                      recommendedCount: _recommendedCount,
                      totalCount: _candidates.length,
                      value: _viewFilter,
                      onChanged: (value) =>
                          setState(() => _viewFilter = value),
                    ),
                    const SizedBox(height: AppSpacing.md),
                    if (visible.isEmpty)
                      _buildEmptyCandidateHint()
                    else
                      _buildCandidateCard(visible),
                    const SizedBox(height: AppSpacing.md),
                    Text(
                      '长按某一行可先重命名再加入',
                      style: AppText.meta.copyWith(color: palette.textSubtle),
                    ),
                  ] else if (!_loading)
                    Padding(
                      padding: const EdgeInsets.only(top: 60),
                      child: _buildEmptyCandidateHint(),
                    ),
                ],
              ),
            ),
            if (_candidates.isNotEmpty) _buildSaveBar(),
          ],
        ),
      ),
    );
  }

  /// 顶栏：返回 + 标题 + 全选/选择文件夹。
  Widget _buildNavBar(bool allSelected, List<LauncherItem> visible) {
    final palette = context.palette;
    return SizedBox(
      height: AppMetrics.navHeight,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppMetrics.pagePadding,
        ),
        child: Row(
          children: [
            IconBtn(
              icon: Icons.chevron_left_rounded,
              tooltip: '返回',
              tonal: true,
              iconSize: 22,
              onPressed: () => Navigator.of(context).maybePop(),
            ),
            const SizedBox(width: AppSpacing.md),
            Expanded(
              child: Text(
                '扫描结果',
                style: AppText.pageTitle.copyWith(color: palette.textPrimary),
              ),
            ),
            if (_candidates.isEmpty)
              TextButton(
                onPressed: _loading || _saving ? null : _chooseFolder,
                child: Text(
                  '选择文件夹',
                  style: AppText.body.copyWith(
                    color: palette.brand,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              )
            else
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
                child: Text(
                  allSelected ? '全不选' : '全选',
                  style: AppText.body.copyWith(
                    color: palette.brand,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  /// 目录与统计卡：路径 + 发现/已选数量。
  Widget _buildFolderCard() {
    final palette = context.palette;
    final folder = _folder;
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(AppMetrics.cardPadding),
      onTap: _loading || _saving ? null : _chooseFolder,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  folder ?? '还没选择文件夹',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textMuted),
                ),
              ),
              const SizedBox(width: AppSpacing.sm),
              Icon(
                Icons.folder_open_rounded,
                size: 16,
                color: palette.textSubtle,
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.sm),
          Text(
            folder == null
                ? '选择电脑上的程序目录后开始扫描'
                : '发现 ${_candidates.length} 个可执行文件 · '
                      '已选 ${_selected.length}',
            style: AppText.sectionTitle.copyWith(color: palette.textPrimary),
          ),
        ],
      ),
    );
  }

  /// 候选列表：单卡内多行，行间分隔线。
  Widget _buildCandidateCard(List<LauncherItem> visible) {
    final palette = context.palette;
    return AppCard(
      radius: AppRadius.lg,
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          for (var i = 0; i < visible.length; i++) ...[
            if (i > 0)
              Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: AppMetrics.cardPadding,
                ),
                child: Divider(height: 1, color: palette.line),
              ),
            _CandidateRow(
              item: visible[i],
              displayName:
                  _renamed[visible[i].executablePath] ?? visible[i].name,
              selected: _selected.contains(visible[i].executablePath),
              onToggle: () => setState(() {
                final path = visible[i].executablePath;
                if (!_selected.remove(path)) _selected.add(path);
              }),
              onLongPress: () => _rename(visible[i]),
            ),
          ],
        ],
      ),
    );
  }

  /// 候选列表为空时的占位提示。
  Widget _buildEmptyCandidateHint() {
    return EmptyState(
      icon: Icons.search_off_rounded,
      title: _folder == null ? '选择文件夹后开始扫描' : '没有可添加的 EXE',
      message: _folder == null
          ? '扫描会列出目录下的可执行文件，并挑出每个目录的主程序作为推荐项。'
          : '这类目里没有推荐项，切到「全部」查看所有可执行文件。',
      action: _folder == null
          ? SecondaryButton(
              label: '选择文件夹',
              icon: Icons.folder_open_rounded,
              onPressed: _chooseFolder,
            )
          : null,
    );
  }

  /// 底部提交栏：已选数量 + 加入应用库。
  Widget _buildSaveBar() {
    final palette = context.palette;
    return Container(
      padding: EdgeInsets.fromLTRB(
        AppMetrics.pagePadding,
        AppSpacing.md,
        AppMetrics.pagePadding,
        MediaQuery.of(context).viewPadding.bottom + AppSpacing.md,
      ),
      color: palette.background,
      child: Row(
        children: [
          Text(
            '已选 ${_selected.length} 项',
            style: AppText.body.copyWith(color: palette.textMuted),
          ),
          const Spacer(),
          SizedBox(
            width: 180,
            child: PrimaryButton(
              label: '加入应用库',
              expand: true,
              busy: _saving,
              onPressed: _saving || _selected.isEmpty ? null : _save,
            ),
          ),
        ],
      ),
    );
  }
}

/// 推荐 / 全部 分段控件（设计稿 S12 中段）。
class _FilterSegment extends StatelessWidget {
  const _FilterSegment({
    required this.recommendedCount,
    required this.totalCount,
    required this.value,
    required this.onChanged,
  });

  final int recommendedCount;
  final int totalCount;
  final _CandidateFilter value;
  final ValueChanged<_CandidateFilter> onChanged;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      height: AppMetrics.minTap,
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: palette.surface2,
        border: Border.all(color: palette.line),
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Row(
        children: [
          _segment(
            context,
            label: '推荐 $recommendedCount',
            active: value == _CandidateFilter.recommended,
            onTap: () => onChanged(_CandidateFilter.recommended),
          ),
          _segment(
            context,
            label: '全部 $totalCount',
            active: value == _CandidateFilter.all,
            onTap: () => onChanged(_CandidateFilter.all),
          ),
        ],
      ),
    );
  }

  Widget _segment(
    BuildContext context, {
    required String label,
    required bool active,
    required VoidCallback onTap,
  }) {
    final palette = context.palette;
    return Expanded(
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onTap,
        child: Container(
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: active ? palette.brandTint : Colors.transparent,
            border: active
                ? Border.all(color: palette.brand.withValues(alpha: 0.45))
                : null,
            borderRadius: BorderRadius.circular(AppRadius.sm),
          ),
          child: Text(
            label,
            style: AppText.label.copyWith(
              color: active ? palette.brand : palette.textMuted,
              fontWeight: active ? FontWeight.w600 : FontWeight.w400,
            ),
          ),
        ),
      ),
    );
  }
}

/// 单个候选行：复选 + 图标 + 名称 + 文件名与推荐状态。
class _CandidateRow extends StatelessWidget {
  const _CandidateRow({
    required this.item,
    required this.displayName,
    required this.selected,
    required this.onToggle,
    required this.onLongPress,
  });

  final LauncherItem item;
  final String displayName;
  final bool selected;
  final VoidCallback onToggle;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final fileName = item.executablePath.split(RegExp(r'[\\/]')).last;
    final state = item.recommended
        ? '推荐'
        : selected
        ? '已选中'
        : '未选中';
    return GestureDetector(
      onLongPress: onLongPress,
      child: InkWell(
        onTap: onToggle,
        child: Container(
          constraints: const BoxConstraints(minHeight: 72),
          padding: const EdgeInsets.symmetric(
            horizontal: AppMetrics.cardPadding,
            vertical: AppSpacing.md,
          ),
          child: Row(
            children: [
              _SelectBox(selected: selected),
              const SizedBox(width: AppSpacing.md),
              _CandidateIcon(item.iconBase64),
              const SizedBox(width: AppSpacing.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      displayName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.rowTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '$fileName · $state',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.meta.copyWith(color: palette.textMuted),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 方形复选框：选中时品牌填充 + 对勾。
class _SelectBox extends StatelessWidget {
  const _SelectBox({required this.selected});

  final bool selected;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      width: 24,
      height: 24,
      decoration: BoxDecoration(
        color: selected ? palette.brand : Colors.transparent,
        border: Border.all(
          color: selected ? palette.brand : palette.lineStrong,
          width: 1.4,
        ),
        borderRadius: BorderRadius.circular(7),
      ),
      child: selected
          ? Icon(Icons.check_rounded, size: 16, color: palette.onBrand)
          : null,
    );
  }
}

class _CandidateIcon extends StatelessWidget {
  const _CandidateIcon(this.base64Icon);
  final String? base64Icon;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
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
        color: palette.info.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(AppRadius.sm),
      ),
      child: bytes == null
          ? Icon(Icons.desktop_windows_outlined, color: palette.info, size: 20)
          : Image.memory(
              bytes,
              fit: BoxFit.contain,
              filterQuality: FilterQuality.high,
              cacheWidth: (40 * MediaQuery.devicePixelRatioOf(context)).round(),
              errorBuilder: (_, _, _) => Icon(
                Icons.desktop_windows_outlined,
                color: palette.info,
                size: 20,
              ),
            ),
    );
  }
}
