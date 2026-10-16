import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/launcher_controller.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import 'app_icon.dart';
import 'app_ui.dart';
import 'file_picker_sheet.dart';
import 'icon_candidate_section.dart';
import 'icon_picker_pc_section.dart';

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
/// 返回值为 `true` 表示图标已改变，调用方需要刷新对应条目的图标。变更记在
/// 调用方持有的 [_IconChangeTracker] 上而不是面板状态里：面板被拖拽关闭时
/// `showModalBottomSheet` 只会返回 `null`，用面板内的标志会丢掉"图标已经换了"
/// 这件事，列表就会一直显示旧图标。
Future<bool> showIconPickerSheet(
  BuildContext context, {
  required LauncherItem item,
}) async {
  final tracker = _IconChangeTracker();
  await showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    builder: (_) => FractionallySizedBox(
      heightFactor: 0.86,
      child: _IconPickerSheet(item: item, tracker: tracker),
    ),
  );
  return tracker.changed;
}

/// 图标面板的变更标记；由调用方持有，面板关闭后仍可读。
class _IconChangeTracker {
  bool changed = false;
}

class _IconPickerSheet extends ConsumerStatefulWidget {
  const _IconPickerSheet({required this.item, required this.tracker});
  final LauncherItem item;
  final _IconChangeTracker tracker;

  @override
  ConsumerState<_IconPickerSheet> createState() => _IconPickerSheetState();
}

class _IconPickerSheetState extends ConsumerState<_IconPickerSheet> {
  /// 在 PC 上选好、还没应用的图片文件路径。
  String? _pickedPath;
  List<IconCandidate> _candidates = const [];
  bool _loading = true;
  bool _busy = false;
  int _revision = 0;
  String _source = '';
  String? _error;

  @override
  void initState() {
    super.initState();
    _source = widget.item.iconSource ?? '';
    _loadCandidates();
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

  /// 用指定文件（或自动来源链）换图标；返回是否成功。
  Future<bool> _apply({String? iconPath}) async {
    if (_busy) return false;
    setState(() => _busy = true);
    try {
      final result = await ref
          .read(launcherControllerProvider)
          .refreshIcon(widget.item.id, iconPath: iconPath);
      if (!mounted) return false;
      setState(() {
        _busy = false;
        _revision += 1;
        _source = result.source;
      });
      widget.tracker.changed = true;
      if (result.iconBase64 == null) {
        context.showWarningSnackBar('没有找到可用图标，已保留原来的图标');
      } else if (result.source == 'shell') {
        context.showWarningSnackBar('只找到系统默认图标，可试试下面的候选文件');
      } else {
        context.showSuccessSnackBar('图标已更新（${iconSourceLabel(result.source)}）');
      }
      await _loadCandidates();
      return true;
    } catch (error) {
      if (!mounted) return false;
      setState(() => _busy = false);
      context.showErrorSnackBar('换图标失败: $error');
      return false;
    }
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
              IconCandidateSection(
                candidates: _candidates,
                loading: _loading,
                busy: _busy,
                error: _error,
                onUse: (path) => _apply(iconPath: path),
              ),
              const SizedBox(height: 16),
              SectionHeading(title: '浏览 PC 上的图片'),
              IconPickerPcSection(
                pickedPath: _pickedPath,
                busy: _busy,
                onBrowse: _browsePcImage,
                onApply: _applyPickedPath,
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
            onPressed: () => Navigator.of(context).pop(),
          ),
        ),
      ],
    );
  }

  /// 打开 PC 文件浏览器挑一张图片；用户取消时保持原状。
  Future<void> _browsePcImage() async {
    final path = await pickIconImage(context, ref);
    if (!mounted || path == null) return;
    setState(() => _pickedPath = path);
  }

  /// 用选中的 PC 图片换图标：只把路径交给服务端，由它重新解析并落库。
  Future<void> _applyPickedPath() async {
    final path = _pickedPath;
    if (path == null) return;
    final ok = await _apply(iconPath: path);
    if (ok && mounted) setState(() => _pickedPath = null);
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
