import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/file_controller.dart';
import '../theme/app_theme.dart';
import 'app_ui.dart';
import 'icon_preview_thumb.dart';

/// 文件浏览弹层的用途。
enum FilePickerMode {
  /// 挑一个可执行文件（新增条目）。
  executable,

  /// 挑一个文件夹（扫描其中的程序）。
  folder,

  /// 挑一张图片作为图标。
  image,
}

/// 可作图标的图片扩展名：与服务端 `get_icon_preview` 能解码的类型一致。
const Set<String> iconImageExtensions = <String>{
  'png',
  'jpg',
  'jpeg',
  'bmp',
  'ico',
};

/// 选择程序可执行文件（从文件树点选，无需输入路径）。
Future<String?> pickExecutable(BuildContext context, WidgetRef ref) =>
    _showFilePicker(context, ref, mode: FilePickerMode.executable);

/// 选择文件夹（用于扫描其中的程序）。
Future<String?> pickFolder(BuildContext context, WidgetRef ref) =>
    _showFilePicker(context, ref, mode: FilePickerMode.folder);

/// 选择 PC 上的一张图片作为图标（同样点选，不输入路径）。
Future<String?> pickIconImage(BuildContext context, WidgetRef ref) =>
    _showFilePicker(context, ref, mode: FilePickerMode.image);

Future<String?> _showFilePicker(
  BuildContext context,
  WidgetRef ref, {
  required FilePickerMode mode,
}) {
  return showModalBottomSheet<String>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    builder: (_) => FractionallySizedBox(
      heightFactor: 0.86,
      child: _FilePickerSheet(mode: mode),
    ),
  );
}

class _FilePickerSheet extends ConsumerStatefulWidget {
  const _FilePickerSheet({required this.mode});
  final FilePickerMode mode;

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

  /// 当前模式下这个文件是否可选；文件夹始终可见，用于继续导航。
  bool _matchesFile(String name) {
    final lower = name.toLowerCase();
    final dot = lower.lastIndexOf('.');
    final extension = dot < 0 ? '' : lower.substring(dot + 1);
    return switch (widget.mode) {
      FilePickerMode.executable => extension == 'exe',
      FilePickerMode.image => iconImageExtensions.contains(extension),
      FilePickerMode.folder => false,
    };
  }

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
    final listing = _listing;
    final parentPath = _parentPath(listing);
    final dirs = <RemoteFileEntry>[];
    final files = <RemoteFileEntry>[];
    for (final entry in listing?.entries ?? const <RemoteFileEntry>[]) {
      if (entry.isDir) {
        dirs.add(entry);
      } else if (_matchesFile(entry.name)) {
        files.add(entry);
      }
    }
    dirs.sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));
    files.sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));

    return Column(
      children: [
        _PickerHeader(
          mode: widget.mode,
          currentPath: listing?.currentPath,
          parentPath: parentPath,
          onNavigate: _load,
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
          Expanded(
            child: _PickerBody(
              mode: widget.mode,
              dirs: dirs,
              files: files,
              loading: _loading,
              onOpenDirectory: _load,
              onPickFile: (path) => Navigator.of(context).pop(path),
            ),
          ),
        if (widget.mode == FilePickerMode.folder && listing != null)
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

/// 选择器顶部：返回上一级按钮、标题与当前路径。
class _PickerHeader extends StatelessWidget {
  const _PickerHeader({
    required this.mode,
    required this.currentPath,
    required this.parentPath,
    required this.onNavigate,
  });

  final FilePickerMode mode;
  final String? currentPath;
  final String? parentPath;
  final ValueChanged<String> onNavigate;

  /// 标题按用途区分，避免用户以为是同一个选择器。
  String get _title => switch (mode) {
    FilePickerMode.executable => '选择程序',
    FilePickerMode.folder => '选择文件夹',
    FilePickerMode.image => '选择图标图片',
  };

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final parent = parentPath;
    final path = currentPath ?? '';
    return Padding(
      padding: const EdgeInsets.fromLTRB(18, 0, 12, 8),
      child: Row(
        children: [
          IconBtn(
            icon: Icons.arrow_upward_rounded,
            tooltip: '上一级',
            size: 38,
            iconSize: 18,
            onPressed: parent != null ? () => onNavigate(parent) : null,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _title,
                  style: AppText.cardTitle.copyWith(
                    color: palette.textPrimary,
                  ),
                ),
                Text(
                  path.isEmpty ? '此电脑' : path,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
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

/// 选择器内容区：空目录提示，或「子文件夹 + 可选文件」列表。
class _PickerBody extends StatelessWidget {
  const _PickerBody({
    required this.mode,
    required this.dirs,
    required this.files,
    required this.loading,
    required this.onOpenDirectory,
    required this.onPickFile,
  });

  final FilePickerMode mode;
  final List<RemoteFileEntry> dirs;
  final List<RemoteFileEntry> files;
  final bool loading;
  final ValueChanged<String> onOpenDirectory;
  final ValueChanged<String> onPickFile;

  /// 空目录文案按用途区分。
  String get _emptyText => switch (mode) {
    FilePickerMode.executable => '此文件夹没有可执行文件',
    FilePickerMode.folder => '此文件夹没有子文件夹',
    FilePickerMode.image => '此文件夹没有图片（png / jpg / bmp / ico）',
  };

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    if (dirs.isEmpty && files.isEmpty && !loading) {
      return Center(
        child: Text(
          _emptyText,
          style: AppText.body.copyWith(color: palette.textSubtle),
        ),
      );
    }
    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      // 惰性构建：图片模式下一行就要取一次缩略图，一次性铺满整个目录会瞬间打出
      // 上百个预览请求。
      itemCount: dirs.length + files.length,
      itemBuilder: (context, index) {
        if (index < dirs.length) {
          final entry = dirs[index];
          return ListTile(
            leading: const Icon(Icons.folder_rounded),
            title: Text(
              entry.name,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            trailing: const Icon(Icons.chevron_right_rounded),
            onTap: () => onOpenDirectory(entry.path),
          );
        }
        final entry = files[index - dirs.length];
        return ListTile(
          // 选图标时要看清是哪张图，所以直接给出缩略图。
          leading: mode == FilePickerMode.image
              ? IconPreviewThumb(path: entry.path, size: 36, radius: 8)
              : const Icon(Icons.desktop_windows_outlined),
          title: Text(
            entry.name,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          trailing: Icon(
            mode == FilePickerMode.image
                ? Icons.chevron_right_rounded
                : Icons.add_rounded,
          ),
          onTap: () => onPickFile(entry.path),
        );
      },
    );
  }
}
