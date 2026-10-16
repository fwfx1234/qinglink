import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/network/remote_command_exception.dart';
import 'remote_command_bus.dart';

/// 进程图标缓存：键为小写可执行文件路径，值为图标 Base64。
///
/// 缺键表示"还没请求过"，值为 `null` 表示"请求过但这个程序没有图标"，两者对
/// 渲染都是占位图标，区别只在于要不要再发一次请求。
class ProcessIconController extends StateNotifier<Map<String, String?>> {
  ProcessIconController(this._bus) : super(const <String, String?>{});

  final RemoteCommandBus _bus;

  /// 已在途或已排队的路径，避免同一程序被重复请求。
  final Set<String> _queued = <String>{};
  final List<_IconRequest> _pending = <_IconRequest>[];

  int _active = 0;

  /// 同时最多两个在途请求：一次滑动会 build 出很多行，放任并发会让服务端的
  /// 图标解析（每个请求都要读文件资源并重新编码）占住输入门闩。
  static const _maxActive = 2;

  /// 排队上限；超出的行等重建时再请求，不排队就能自然丢掉。
  static const _maxQueued = 64;

  /// 缓存条目上限，按插入顺序淘汰最旧的。
  static const _maxEntries = 512;

  /// 请求一个进程的图标；已缓存、在途或排队中的路径会被忽略。
  void request({required String? path, required int pid}) {
    final key = cacheKey(path);
    if (key == null || state.containsKey(key) || _queued.contains(key)) return;
    if (_pending.length >= _maxQueued) return;
    _queued.add(key);
    _pending.add(_IconRequest(key: key, pid: pid));
    _drain();
  }

  /// 丢弃解析失败的记录，使它们在下一次刷新时重试。
  void clearFailures() {
    if (!state.values.any((icon) => icon == null)) return;
    state = Map<String, String?>.fromEntries(
      state.entries.where((entry) => entry.value != null),
    );
  }

  /// 缓存键：路径大小写不敏感；空路径表示没有可请求的目标。
  static String? cacheKey(String? path) {
    final trimmed = path?.trim();
    if (trimmed == null || trimmed.isEmpty) return null;
    return trimmed.toLowerCase();
  }

  /// 在并发上限内依次处理排队中的请求。
  void _drain() {
    while (_active < _maxActive && _pending.isNotEmpty) {
      final request = _pending.removeAt(0);
      _active += 1;
      unawaited(_resolve(request));
    }
  }

  Future<void> _resolve(_IconRequest request) async {
    String? icon;
    try {
      final response = await _bus.request(<String, dynamic>{
        'type': 'get_process_icon',
        'pid': request.pid,
      });
      if (response['type'] == 'process_icon') {
        icon = _readPayload(response['data']);
      }
    } on RemoteCommandException catch (error) {
      // 服务端明确拒绝（例如旧版本没有这条命令）：记负结果，避免每行重建都
      // 再问一次。
      debugPrint('process icon rejected: ${error.code}');
      icon = null;
    } catch (error) {
      // 连接层异常（未连接、超时）：不写缓存，等下一次重建时重试。
      debugPrint('process icon failed: $error');
      _finish(request.key);
      return;
    }
    if (mounted) _store(request.key, icon);
    _finish(request.key);
  }

  /// 把一条结果写进缓存，并在超出上限时淘汰最旧的条目。
  void _store(String key, String? icon) {
    final next = Map<String, String?>.from(state)..[key] = icon;
    while (next.length > _maxEntries) {
      next.remove(next.keys.first);
    }
    state = next;
  }

  void _finish(String key) {
    _active -= 1;
    _queued.remove(key);
    if (mounted) _drain();
  }

  static String? _readPayload(Object? data) {
    if (data is! Map) return null;
    final payload = data['icon_base64'];
    return payload is String && payload.isNotEmpty ? payload : null;
  }
}

/// 一条待解析的图标请求。
class _IconRequest {
  const _IconRequest({required this.key, required this.pid});

  final String key;
  final int pid;
}

final processIconControllerProvider =
    StateNotifierProvider<ProcessIconController, Map<String, String?>>(
  (ref) => ProcessIconController(ref.watch(remoteCommandBusProvider)),
);
