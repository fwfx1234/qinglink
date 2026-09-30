import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/file_controller.dart';
import '../theme/app_theme.dart';
import 'app_ui.dart';

/// 选择程序可执行文件（从文件树点选，无需输入路径）。
Future<String?> pickExecutable(BuildContext context, WidgetRef ref) =>
    _showFilePicker(context, ref, folderMode: false);

/// 选择文件夹（用于扫描其中的程序）。
Future<String?> pickFolder(BuildContext context, WidgetRef ref) =>
    _showFilePicker(context, ref, folderMode: true);

Future<String?> _showFilePicker(
  BuildContext context,
  WidgetRef ref, {
  required bool folderMode,
}) {
  return showModalBottomSheet<String>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    builder: (_) => FractionallySizedBox(
      heightFactor: 0.86,
      child: _FilePickerSheet(folderMode: folderMode),
    ),
  );
}

class _FilePickerSheet extends ConsumerStatefulWidget {
  const _FilePickerSheet({required this.folderMode});
  final bool folderMode;

  @override
  ConsumerState<_FilePickerSheet> createState() => _FilePickerSheetState();
}

class _FilePickerSheetState extends ConsumerState<_FilePickerSheet> {
  RemoteDirectoryListing? _listing;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load(null));
  }

  Future<void> _load(String? path) async {
    if (!mounted) return;
    setState(() {
      _loading = true;
      _error = null;
    });
    final listing = await ref.read(fileControllerProvider).browse(path);
    if (!mounted) return;
    setState(() {
      _loading = false;
      _listing = listing;
      if (listing == null) _error = '无法读取目录';
    });
  }

  bool _isExecutable(String name) => name.toLowerCase().endsWith('.exe');

  String? _parentPath(RemoteDirectoryListing? listing) {
    if (listing == null || listing.currentPath.isEmpty) return null;
    final normalized = listing.currentPath.replaceAll('/', '\\');
    if (RegExp(r'^[A-Za-z]:\\?$').hasMatch(normalized)) {
      // At a drive root the previous level is the virtual "This PC" listing.
      return '';
    }
    final explicit = listing.parentPath;
    if (explicit != null && explicit != listing.currentPath) return explicit;
    final trimmed = normalized.replaceFirst(RegExp(r'\\+$'), '');
    final separator = trimmed.lastIndexOf('\\');
    if (separator < 0) {
      return null;
    }
    if (separator == 2 && trimmed.length >= 2) {
      return '${trimmed.substring(0, 2)}\\';
    }
    return trimmed.substring(0, separator);
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final listing = _listing;
    final parentPath = _parentPath(listing);
    final dirs = <RemoteFileEntry>[];
    final files = <RemoteFileEntry>[];
    for (final entry in listing?.entries ?? const <RemoteFileEntry>[]) {
      if (entry.isDir) {
        dirs.add(entry);
      } else if (!widget.folderMode && _isExecutable(entry.name)) {
        files.add(entry);
      }
    }
    dirs.sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));
    files.sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 0, 12, 8),
          child: Row(
            children: [
              IconBtn(
                icon: Icons.arrow_upward_rounded,
                tooltip: '上一级',
                size: 38,
                iconSize: 18,
                onPressed: parentPath != null ? () => _load(parentPath) : null,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      widget.folderMode ? '选择文件夹' : '选择程序',
                      style: AppText.cardTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                    Text(
                      (listing?.currentPath.isEmpty ?? true)
                          ? '此电脑'
                          : listing!.currentPath,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.meta.copyWith(color: palette.textSubtle),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 8),
            child: Text(
              _error!,
              style: AppText.body.copyWith(color: AppColors.error),
            ),
          ),
        if (_loading)
          const LinearProgressIndicator(minHeight: 2, color: AppColors.primary),
        if (listing != null)
          if (dirs.isEmpty && files.isEmpty && !_loading)
            Expanded(
              child: Center(
                child: Text(
                  widget.folderMode ? '此文件夹没有子文件夹' : '此文件夹没有可执行文件',
                  style: AppText.body.copyWith(color: palette.textSubtle),
                ),
              ),
            )
          else
            Expanded(
              child: ListView(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                children: [
                  ...dirs.map(
                    (entry) => ListTile(
                      leading: const Icon(Icons.folder_rounded),
                      title: Text(
                        entry.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      trailing: const Icon(Icons.chevron_right_rounded),
                      onTap: () => _load(entry.path),
                    ),
                  ),
                  ...files.map(
                    (entry) => ListTile(
                      leading: const Icon(Icons.desktop_windows_outlined),
                      title: Text(
                        entry.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      trailing: const Icon(Icons.add_rounded),
                      onTap: () => Navigator.of(context).pop(entry.path),
                    ),
                  ),
                ],
              ),
            ),
        if (widget.folderMode && listing != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(18, 8, 18, 16),
            child: PrimaryButton(
              label: '扫描此文件夹',
              icon: Icons.search_rounded,
              expand: true,
              onPressed: listing.currentPath.isEmpty
                  ? null
                  : () => Navigator.of(context).pop(listing.currentPath),
            ),
          ),
      ],
    );
  }
}
