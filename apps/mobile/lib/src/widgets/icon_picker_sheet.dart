import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/launcher_controller.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import 'app_icon.dart';
import 'app_ui.dart';

/// 图标来源的中文说明。`shell` 意味着只拿到系统默认图标，通常就是"图标不对"。
String iconSourceLabel(String? source) {
  switch (source) {
    case 'manual':
      return '手动指定';
    case 'steam':
      return 'Steam 官方封面';
    case 'shortcut':
      return '快捷方式目标';
    case 'executable':
      return '程序内嵌资源';
    case 'image':
      return '同目录图片文件';
    case 'shell':
      return '系统默认图标（很可能不准确）';
    case 'none':
      return '未找到可用图标';
    default:
      return '尚未解析，打开列表时会自动解析';
  }
}

/// 单条目图标修复面板。
///
/// 自动扫描只看得到程序内嵌资源，而很多程序的图标其实是快捷方式指定的
/// 外部文件或同目录的 PNG/ICO。这个面板把服务端能找到的全部候选列出来，
/// 让用户一次点掉"图标不对"的问题。
///
/// 返回值为 `true` 表示图标已改变，调用方需要刷新对应条目的图标。
Future<bool> showIconPickerSheet(
  BuildContext context, {
  required LauncherItem item,
}) async {
  final changed = await showModalBottomSheet<bool>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    builder: (_) => FractionallySizedBox(
      heightFactor: 0.86,
      child: _IconPickerSheet(item: item),
    ),
  );
  return changed ?? false;
}

class _IconPickerSheet extends ConsumerStatefulWidget {
  const _IconPickerSheet({required this.item});
  final LauncherItem item;

  @override
  ConsumerState<_IconPickerSheet> createState() => _IconPickerSheetState();
}

class _IconPickerSheetState extends ConsumerState<_IconPickerSheet> {
  final _manualController = TextEditingController();
  List<IconCandidate> _candidates = const [];
  bool _loading = true;
  bool _busy = false;
  int _revision = 0;
  String _source = '';
  bool _changed = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _source = widget.item.iconSource ?? '';
    _loadCandidates();
  }

  @override
  void dispose() {
    _manualController.dispose();
    super.dispose();
  }

  Future<void> _loadCandidates() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final candidates = await ref
          .read(launcherControllerProvider)
          .iconCandidates(widget.item.id);
      if (!mounted) return;
      setState(() {
        _candidates = candidates;
        _loading = false;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = '$error';
      });
    }
  }

  Future<void> _apply({String? iconPath}) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      final result = await ref
          .read(launcherControllerProvider)
          .refreshIcon(widget.item.id, iconPath: iconPath);
      if (!mounted) return;
      setState(() {
        _busy = false;
        _changed = true;
        _revision += 1;
        _source = result.source;
      });
      if (result.iconBase64 == null) {
        context.showWarningSnackBar('没有找到可用图标，已保留原来的图标');
      } else if (result.source == 'shell') {
        context.showWarningSnackBar('只找到系统默认图标，可试试下面的候选文件');
      } else {
        context.showSuccessSnackBar('图标已更新（${iconSourceLabel(result.source)}）');
      }
      await _loadCandidates();
    } catch (error) {
      if (!mounted) return;
      setState(() => _busy = false);
      context.showErrorSnackBar('换图标失败: $error');
    }
  }

  String _formatBytes(int bytes) {
    if (bytes >= 1024 * 1024) return '${(bytes / 1024 / 1024).toStringAsFixed(1)} MB';
    if (bytes >= 1024) return '${(bytes / 1024).round()} KB';
    return '$bytes B';
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _IconPickerHeader(
          item: widget.item,
          source: _source,
          revision: _revision,
        ),
        Expanded(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(18, 8, 18, 8),
            children: [
              PrimaryButton(
                label: '自动重新扫描',
                icon: Icons.refresh_rounded,
                expand: true,
                busy: _busy,
                onPressed: _busy ? null : () => _apply(),
              ),
              const SizedBox(height: 6),
              Text(
                '依次查找：快捷方式指定的图标 → 程序内嵌资源 → 同目录 icon.png / 同名 .png、.ico → 系统默认图标。再次自动重扫会清除手动指定的图标。',
                style: AppText.meta.copyWith(color: palette.textSubtle),
              ),
              const SizedBox(height: 16),
              SectionHeading(
                title: 'PC 上检测到的图标文件',
                trailing: _loading
                    ? const SizedBox(
                        width: 14,
                        height: 14,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : null,
              ),
              ..._buildCandidateSection(palette),
              const SizedBox(height: 8),
              SectionHeading(title: '手动指定图标文件'),
              TextField(
                controller: _manualController,
                autocorrect: false,
                enableSuggestions: false,
                decoration: const InputDecoration(
                  labelText: '图标文件路径',
                  hintText: r'例如 D:\Games\Example\icon.png',
                  isDense: true,
                ),
              ),
              const SizedBox(height: 8),
              SecondaryButton(
                label: '使用该文件',
                icon: Icons.check_rounded,
                expand: true,
                onPressed: _busy ? null : _applyManualPath,
              ),
              const SizedBox(height: 20),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 4, 18, 16),
          child: SecondaryButton(
            label: '完成',
            expand: true,
            onPressed: () => Navigator.of(context).pop(_changed),
          ),
        ),
      ],
    );
  }

  /// 使用手动填写的路径换图标；路径为空时提示用户先填写。
  void _applyManualPath() {
    final path = _manualController.text.trim();
    if (path.isEmpty) {
      context.showWarningSnackBar('请先填写图标文件路径');
      return;
    }
    _apply(iconPath: path);
  }

  /// 候选图标区块：按错误 / 空结果 / 有候选三种情况产出列表子项。
  List<Widget> _buildCandidateSection(AppPalette palette) {
    if (_error != null) {
      return [
        Text(_error!, style: AppText.meta.copyWith(color: AppColors.red)),
      ];
    }
    if (!_loading && _candidates.isEmpty) {
      return [
        Text(
          '没有找到其他可用的图标文件，可以手动填写路径。',
          style: AppText.meta.copyWith(color: palette.textSubtle),
        ),
      ];
    }
    return [
      for (final candidate in _candidates) ...[
        _CandidateCard(
          candidate: candidate,
          sizeLabel: _formatBytes(candidate.sizeBytes),
          onUse: _busy ? null : () => _apply(iconPath: candidate.path),
        ),
        const SizedBox(height: 8),
      ],
    ];
  }
}

/// 面板顶部：当前图标预览、条目名称与图标来源。
class _IconPickerHeader extends StatelessWidget {
  const _IconPickerHeader({
    required this.item,
    required this.source,
    required this.revision,
  });

  final LauncherItem item;
  final String source;
  final int revision;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.fromLTRB(18, 0, 18, 4),
      child: Row(
        children: [
          AppIcon(
            id: item.id,
            initialBase64: item.iconBase64,
            size: 52,
            radius: 13,
            revision: revision,
          ),
          const SizedBox(width: 12),
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
                  '当前来源：${iconSourceLabel(source)}',
                  maxLines: 2,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 单条候选图标文件：类型、路径、体积与「使用」按钮。
class _CandidateCard extends StatelessWidget {
  const _CandidateCard({
    required this.candidate,
    required this.sizeLabel,
    required this.onUse,
  });

  final IconCandidate candidate;
  final String sizeLabel;

  /// 为 null 表示正在忙碌，此时禁用「使用」按钮。
  final VoidCallback? onUse;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 54,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  candidate.kindLabel,
                  style: AppText.body.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 2),
                Text(
                  candidate.path,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          const SizedBox(width: 6),
          Text(
            sizeLabel,
            style: AppText.meta.copyWith(color: palette.textSubtle),
          ),
          const SizedBox(width: 6),
          SecondaryButton(label: '使用', height: 34, onPressed: onUse),
        ],
      ),
    );
  }
}
