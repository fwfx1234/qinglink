import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/launcher_controller.dart';
import '../theme/app_theme.dart';

/// 从 PC 按需读取程序图标（base64），失败时回退到图标底衬。
class AppIcon extends ConsumerStatefulWidget {
  const AppIcon({
    super.key,
    required this.id,
    this.initialBase64,
    this.size = 40,
    this.radius = 10,
    this.fallback = Icons.apps_rounded,
    this.tone = AppColors.cyan,
    this.revision = 0,
  });

  final String? id;
  final String? initialBase64;
  final double size;
  final double radius;
  final IconData fallback;
  final Color tone;

  /// Bump to force a reload after the PC icon has been refreshed.
  final int revision;

  @override
  ConsumerState<AppIcon> createState() => _AppIconState();
}

class _AppIconState extends ConsumerState<AppIcon> {
  Uint8List? _bytes;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    _decodeInitialIcon();
    _load();
  }

  @override
  void didUpdateWidget(covariant AppIcon oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.id != widget.id ||
        oldWidget.revision != widget.revision ||
        oldWidget.initialBase64 != widget.initialBase64) {
      _bytes = null;
      _decodeInitialIcon();
      _load();
    }
  }

  void _decodeInitialIcon() {
    final encoded = widget.initialBase64;
    if (encoded == null || encoded.isEmpty) return;
    try {
      _bytes = base64Decode(encoded);
    } catch (_) {
      _bytes = null;
    }
  }

  Future<void> _load() async {
    final id = widget.id;
    if (id == null || id.isEmpty || _loading) return;
    _loading = true;
    final base64Icon = await ref
        .read(launcherControllerProvider)
        .icon(id);
    _loading = false;
    if (!mounted || base64Icon == null || base64Icon.isEmpty) return;
    try {
      final bytes = base64Decode(base64Icon);
      if (mounted) setState(() => _bytes = bytes);
    } catch (_) {
      // 图标数据损坏时保留回退图标。
    }
  }

  @override
  Widget build(BuildContext context) {
    final bytes = _bytes;
    final cacheWidth = (widget.size * MediaQuery.devicePixelRatioOf(context))
        .round();
    return Container(
      width: widget.size,
      height: widget.size,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: bytes == null ? widget.tone.withValues(alpha: 0.16) : null,
        borderRadius: BorderRadius.circular(widget.radius),
      ),
      child: bytes == null
          ? Icon(widget.fallback, size: widget.size * 0.5, color: widget.tone)
          : Image.memory(
              bytes,
              fit: BoxFit.contain,
              cacheWidth: cacheWidth,
              filterQuality: FilterQuality.high,
              isAntiAlias: true,
              gaplessPlayback: true,
              errorBuilder: (_, _, _) => Icon(
                widget.fallback,
                size: widget.size * 0.5,
                color: widget.tone,
              ),
            ),
    );
  }
}
