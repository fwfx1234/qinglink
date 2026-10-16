/// 按 PC 路径显示图标预览缩略图。
///
/// 手机看不到 PC 的文件，候选列表与文件浏览器里的缩略图都要向 PC 取一次
/// （`get_icon_preview`：只解码、不落库）。同一路径只请求一次，结果——包括
/// "这个文件没有可用预览"——缓存在内存里，避免滚动时反复请求。
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/launcher_controller.dart';
import '../theme/app_theme.dart';

/// 预览缓存：路径 → PNG 字节；值为 `null` 表示取过但没有可用预览。
abstract final class IconPreviewCache {
  /// 缓存上限；超出后整体清空（预览只是展示用的短命数据）。
  static const int _maxEntries = 240;
  static final Map<String, Uint8List?> _entries = <String, Uint8List?>{};

  static bool isLoaded(String path) => _entries.containsKey(path);

  static Uint8List? bytesOf(String path) => _entries[path];

  static void store(String path, Uint8List? bytes) {
    if (_entries.length >= _maxEntries) _entries.clear();
    _entries[path] = bytes;
  }
}

/// 一个图标文件的缩略图；取不到预览时显示占位图标。
class IconPreviewThumb extends ConsumerStatefulWidget {
  const IconPreviewThumb({
    super.key,
    required this.path,
    this.size = 40,
    this.radius = 10,
    this.fallbackIcon = Icons.image_outlined,
  });

  /// PC 上的文件路径。
  final String path;
  final double size;
  final double radius;
  final IconData fallbackIcon;

  @override
  ConsumerState<IconPreviewThumb> createState() => _IconPreviewThumbState();
}

class _IconPreviewThumbState extends ConsumerState<IconPreviewThumb> {
  Uint8List? _bytes;

  @override
  void initState() {
    super.initState();
    _start();
  }

  @override
  void didUpdateWidget(IconPreviewThumb oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.path != widget.path) _start();
  }

  /// 取缓存值；未缓存时在首帧之后再请求（避免在 build 期间发请求）。
  void _start() {
    _bytes = IconPreviewCache.bytesOf(widget.path);
    if (IconPreviewCache.isLoaded(widget.path)) return;
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    if (!mounted) return;
    final encoded = await ref
        .read(launcherControllerProvider)
        .previewIcon(widget.path);
    Uint8List? bytes;
    if (encoded != null && encoded.isNotEmpty) {
      try {
        bytes = base64Decode(encoded);
      } catch (_) {
        bytes = null;
      }
    }
    IconPreviewCache.store(widget.path, bytes);
    if (!mounted) return;
    setState(() => _bytes = bytes);
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final bytes = _bytes;
    if (bytes == null) {
      return Container(
        width: widget.size,
        height: widget.size,
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: palette.surface2,
          borderRadius: BorderRadius.circular(widget.radius),
        ),
        child: Icon(
          widget.fallbackIcon,
          size: widget.size * 0.5,
          color: palette.textSubtle,
        ),
      );
    }
    return ClipRRect(
      borderRadius: BorderRadius.circular(widget.radius),
      child: Image.memory(
        bytes,
        width: widget.size,
        height: widget.size,
        fit: BoxFit.contain,
        gaplessPlayback: true,
      ),
    );
  }
}
