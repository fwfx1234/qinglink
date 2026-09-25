import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';
import 'app_ui.dart';

/// 相对扫描根目录的路径，便于区分多层子目录中的同名程序。
String _relativePath(String path, String folder) {
  if (folder.isEmpty) return path;
  var base = folder.replaceAll('/', r'\');
  base = base.replaceFirst(RegExp(r'\\+$'), '');
  final normalized = path.replaceAll('/', r'\');
  if (normalized.toLowerCase().startsWith(base.toLowerCase())) {
    final rel = normalized
        .substring(base.length)
        .replaceFirst(RegExp(r'^\\+'), '');
    return rel.isEmpty ? normalized : rel;
  }
  return path;
}

/// 展示目录扫描出的程序候选，多选后返回选中的 exe 路径。
Future<List<String>?> showScanCandidatesSheet(
  BuildContext context, {
  required String folder,
  required List<LauncherItem> candidates,
}) {
  return showModalBottomSheet<List<String>>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    builder: (_) => FractionallySizedBox(
      heightFactor: 0.86,
      child: _ScanCandidatesSheet(folder: folder, candidates: candidates),
    ),
  );
}

class _ScanCandidatesSheet extends StatefulWidget {
  const _ScanCandidatesSheet({required this.folder, required this.candidates});
  final String folder;
  final List<LauncherItem> candidates;

  @override
  State<_ScanCandidatesSheet> createState() => _ScanCandidatesSheetState();
}

class _ScanCandidatesSheetState extends State<_ScanCandidatesSheet> {
  late final Set<String> _selected = {
    for (final item in widget.candidates) item.executablePath,
  };

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final allSelected = _selected.length == widget.candidates.length;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 0, 12, 8),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '扫描到 ${widget.candidates.length} 个程序',
                      style: AppText.cardTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                    Text(
                      widget.folder,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.meta.copyWith(color: palette.textSubtle),
                    ),
                  ],
                ),
              ),
              TextButton(
                onPressed: () => setState(() {
                  if (allSelected) {
                    _selected.clear();
                  } else {
                    _selected
                      ..clear()
                      ..addAll(widget.candidates.map((e) => e.executablePath));
                  }
                }),
                child: Text(allSelected ? '全不选' : '全选'),
              ),
            ],
          ),
        ),
        Expanded(
          child: ListView(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            children: widget.candidates
                .map(
                  (item) => CheckboxListTile(
                    value: _selected.contains(item.executablePath),
                    onChanged: (value) => setState(() {
                      if (value == true) {
                        _selected.add(item.executablePath);
                      } else {
                        _selected.remove(item.executablePath);
                      }
                    }),
                    title: Text(
                      item.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    subtitle: Text(
                      _relativePath(item.executablePath, widget.folder),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    controlAffinity: ListTileControlAffinity.leading,
                  ),
                )
                .toList(),
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 8, 18, 16),
          child: PrimaryButton(
            label: _selected.isEmpty ? '请选择程序' : '添加 ${_selected.length} 个程序',
            icon: Icons.add_rounded,
            expand: true,
            onPressed: _selected.isEmpty
                ? null
                : () => Navigator.of(context).pop(_selected.toList()),
          ),
        ),
      ],
    );
  }
}
