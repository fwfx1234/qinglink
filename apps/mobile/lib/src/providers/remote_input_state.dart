import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'remote_command_bus.dart';

/// PC 侧鼠标按键的实时状态。
///
/// 由服务端 `input_state` 事件推送（状态变化时 + 每 10 秒一次补发）。它同时反映物理按压
/// 与远端注入，所以手机端不用自己维护"我按下了什么"之外的额外状态。
class RemoteInputState {
  const RemoteInputState({
    this.left = false,
    this.right = false,
    this.middle = false,
  });

  final bool left;
  final bool right;
  final bool middle;

  /// 是否有任一按键处于按下状态。
  bool get anyPressed => left || right || middle;

  /// 触控板角标文案；没有按下时为空串。
  String get label {
    final parts = <String>[
      if (left) '左键',
      if (right) '右键',
      if (middle) '中键',
    ];
    return parts.isEmpty ? '' : '${parts.join(' / ')}按住';
  }

  RemoteInputState copyWith({bool? left, bool? right, bool? middle}) =>
      RemoteInputState(
        left: left ?? this.left,
        right: right ?? this.right,
        middle: middle ?? this.middle,
      );
}

class RemoteInputStateController extends StateNotifier<RemoteInputState> {
  RemoteInputStateController(RemoteCommandBus bus)
    : super(const RemoteInputState()) {
    _subscription = bus.events.listen((event) {
      if (event['type'] != 'input_state') return;
      final data = event['data'];
      if (data is! Map) return;
      state = RemoteInputState(
        left: data['left'] == true,
        right: data['right'] == true,
        middle: data['middle'] == true,
      );
    });
  }

  late final StreamSubscription<Map<String, dynamic>> _subscription;

  @override
  void dispose() {
    unawaited(_subscription.cancel());
    super.dispose();
  }
}

final remoteInputStateProvider =
    StateNotifierProvider<RemoteInputStateController, RemoteInputState>((ref) {
      return RemoteInputStateController(ref.watch(remoteCommandBusProvider));
    });
