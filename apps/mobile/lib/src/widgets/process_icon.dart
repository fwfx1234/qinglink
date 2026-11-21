import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/process_icon_controller.dart';
import '../theme/app_theme.dart';
import 'app_card.dart';

/// 进程行图标：命中 PC 返回的 Base64 时显示图片，否则退回 [TileIcon] 占位。
///
/// 按需取图是刻意的：进程列表一次几十行，把图标塞进列表载荷会让每次搜索和
/// 下拉刷新都重传几百 KiB；这里一行的图标只请求一次、也只解码一次。
class ProcessIcon extends ConsumerStatefulWidget {
  const ProcessIcon({
    super.key,
    required this.path,
    required this.pid,
    this.size = 32,
    this.radius = 9,
    this.iconSize = 16,
    this.color,
    this.fallback = Icons.memory_rounded,
  });

  /// 进程可执行文件路径；为空时不发请求，只显示占位图标。
  final String? path;
  /// 目标进程 PID。
  final int pid;
  final double size;
  final double radius;
  final double iconSize;

  /// 占位图标的颜色；为空时取当前主题的信息色。
  final Color? color;
  final IconData fallback;

  @override
  ConsumerState<ProcessIcon> createState() => _ProcessIconState();
}

class _ProcessIconState extends ConsumerState<ProcessIcon> {
  /// 已解码的字节与它对应的 Base64，避免每帧重复解码。
  Uint8List? _bytes;
  String? _decodedFrom;

  @override
  void initState() {
    super.initState();
    _request();
  }

  @override
  void didUpdateWidget(covariant ProcessIcon oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.path != widget.path || oldWidget.pid != widget.pid) {
      _bytes = null;
      _decodedFrom = null;
      _request();
    }
  }

  /// 请一次图标；控制器对已缓存 / 在途的路径会直接忽略。
  void _request() {
    ref
        .read(processIconControllerProvider.notifier)
        .request(path: widget.path, pid: widget.pid);
  }

  /// 解码当前路径的图标；同一份数据只解码一次。
  Uint8List? _decode(String? encoded) {
    if (encoded == null || encoded.isEmpty) return null;
    if (encoded == _decodedFrom) return _bytes;
    _decodedFrom = encoded;
    try {
      _bytes = base64Decode(encoded);
    } catch (error) {
      debugPrint('process icon decode failed: $error');
      _bytes = null;
    }
    return _bytes;
  }

  Widget _placeholder() {
    return TileIcon(
      icon: widget.fallback,
      color: widget.color ?? context.palette.info,
      size: widget.size,
      radius: widget.radius,
      iconSize: widget.iconSize,
      // 设计稿里每行图标都有浅色底衬，取不到真实图标时也保持一致。
      filled: true,
    );
  }

  @override
  Widget build(BuildContext context) {
    final path = ProcessIconController.cacheKey(widget.path);
    final bytes = path == null
        ? null
        : _decode(ref.watch(processIconControllerProvider)[path]);
    if (bytes == null) return _placeholder();

    final cacheWidth = (widget.size * MediaQuery.devicePixelRatioOf(context))
        .round();
    return Container(
      width: widget.size,
      height: widget.size,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(widget.radius),
      ),
      child: Image.memory(
        bytes,
        fit: BoxFit.contain,
        cacheWidth: cacheWidth,
        filterQuality: FilterQuality.high,
        isAntiAlias: true,
        gaplessPlayback: true,
        errorBuilder: (_, _, _) => _placeholder(),
      ),
    );
  }
}
