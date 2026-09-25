import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../providers/remote_control_provider.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import '../widgets/file_picker_sheet.dart';

/// Dedicated flow for scanning a folder and reviewing executable candidates.
class ScanFolderScreen extends ConsumerStatefulWidget {
  const ScanFolderScreen({super.key});

  @override
  ConsumerState<ScanFolderScreen> createState() => _ScanFolderScreenState();
}

class _ScanFolderScreenState extends ConsumerState<ScanFolderScreen> {
  String? _folder;
  List<LauncherItem> _candidates = const [];
  final Set<String> _selected = <String>{};
  final Map<String, TextEditingController> _names = {};
  bool _loading = false;
  bool _saving = false;

  @override
  void dispose() {
    for (final controller in _names.values) {
      controller.dispose();
    }
    super.dispose();
  }

  Future<void> _chooseFolder() async {
    final folder = await pickFolder(context, ref);
    if (folder == null || !mounted) return;
    setState(() {
      _folder = folder;
      _candidates = const [];
      _selected.clear();
      for (final controller in _names.values) {
        controller.dispose();
      }
      _names.clear();
    });
    await _scan(folder);
  }

  Future<void> _scan(String folder) async {
    setState(() => _loading = true);
    GameScanCandidatesResponse? result;
    try {
      result = await ref
          .read(remoteControlProvider.notifier)
          .scanGameDirectory(folder);
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
    for (final controller in _names.values) {
      controller.dispose();
    }
    setState(() {
      _candidates = candidates;
      _selected
        ..clear()
        ..addAll(candidates.map((item) => item.executablePath));
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
    setState(() => _saving = true);
    var added = 0;
    for (final item in selected) {
      final name = _names[item.executablePath]?.text.trim();
      if (name == null || name.isEmpty) continue;
      final id = await ref
          .read(remoteControlProvider.notifier)
          .addScannerApp(
            name: name,
            exePath: item.executablePath,
            category: 'Game',
            iconBase64: item.iconBase64,
          );
      if (id == null) continue;
      await ref
          .read(settingsProvider.notifier)
          .addToLibrary(
            QuickApp(
              name: name,
              path: item.executablePath,
              scannerId: id,
              isGame: true,
              originalName: item.originalName,
              pinnedToHome: false,
            ),
          );
      added++;
    }
    if (!mounted) return;
    setState(() => _saving = false);
    if (added == 0) {
      context.showErrorSnackBar('没有成功添加程序');
      return;
    }
    context.showSuccessSnackBar('已添加 $added 个程序');
    Navigator.of(context).pop(true);
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final allSelected =
        _candidates.isNotEmpty && _selected.length == _candidates.length;
    return Scaffold(
      appBar: AppBar(title: const Text('扫描文件夹')),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(18, 8, 18, 12),
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
          ),
          if (_loading) const LinearProgressIndicator(minHeight: 2),
          if (_candidates.isNotEmpty)
            Padding(
              padding: const EdgeInsets.fromLTRB(18, 0, 12, 4),
              child: Row(
                children: [
                  Text(
                    '扫描到 ${_candidates.length} 个程序',
                    style: AppText.cardTitle.copyWith(
                      color: palette.textPrimary,
                    ),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed: () => setState(() {
                      if (allSelected) {
                        _selected.clear();
                      } else {
                        _selected
                          ..clear()
                          ..addAll(_candidates.map((e) => e.executablePath));
                      }
                    }),
                    child: Text(allSelected ? '全不选' : '全选'),
                  ),
                ],
              ),
            ),
          Expanded(
            child: _candidates.isEmpty
                ? Center(
                    child: Text(
                      _folder == null ? '选择文件夹后开始扫描' : '没有可添加的 EXE',
                      style: AppText.body.copyWith(color: palette.textSubtle),
                    ),
                  )
                : ListView.separated(
                    padding: const EdgeInsets.fromLTRB(18, 0, 18, 18),
                    itemCount: _candidates.length,
                    separatorBuilder: (_, _) => const SizedBox(height: 8),
                    itemBuilder: (context, index) {
                      final item = _candidates[index];
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
                                  TextField(
                                    controller: _names[path],
                                    decoration: const InputDecoration(
                                      labelText: '显示名称',
                                      isDense: true,
                                      border: InputBorder.none,
                                    ),
                                  ),
                                  Text(
                                    path,
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
                  ),
          ),
          if (_candidates.isNotEmpty)
            Padding(
              padding: const EdgeInsets.fromLTRB(18, 8, 18, 18),
              child: PrimaryButton(
                label: _selected.isEmpty
                    ? '请选择程序'
                    : '添加 ${_selected.length} 个程序',
                icon: Icons.add_rounded,
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
            ),
    );
  }
}
