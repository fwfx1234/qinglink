import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/mouse_move_frame.dart';
import '../services/shortcut_mapper.dart';
import 'remote_command_bus.dart';

class InputController {
  InputController(this._bus);
  final RemoteCommandBus _bus;
  int _sequence = 0;

  /// 发送相对位移：优先走二进制输入通道，不可用时回退主通道。
  ///
  /// 回退必须用 fire-and-forget 的 [RemoteCommandBus.send]：服务端在已认证连接上
  /// **刻意不为 `mouse_move` 回包**（`ws_handler.rs` 的 `should_reply`），用 `request`
  /// 会一直等到 10 秒超时并让串行鼠标队列堵在队头。
  Future<void> mouseMove(int dx, int dy) async {
    if (dx == 0 && dy == 0) return;
    final sent = await _bus.sendBinary(MouseMoveFrame(sequence: _sequence++, dx: dx, dy: dy, timestampMs: DateTime.now().millisecondsSinceEpoch & 0xffffffff).encode());
    if (!sent) _bus.send({'type': 'mouse_move', 'dx': dx, 'dy': dy});
  }

  Future<void> mouseButton(String button) => _bus.request({'type': 'mouse_button', 'button': button});
  Future<void> mouseButtonState(String button, {required bool pressed}) => _bus.request({'type': 'mouse_button_state', 'button': button, 'pressed': pressed});
  Future<void> mouseWheel(int delta) => _bus.request({'type': 'mouse_wheel', 'delta': delta});
  Future<void> sendText(String text) async { if (text.isNotEmpty) await _bus.request({'type': 'send_text', 'text': text}); }
  Future<void> pressKey(String key) => _bus.request({'type': 'key_press', 'key': key});

  /// 发送一个快捷键：按下与抬起两条 `key_event`。
  ///
  /// 服务端没有名为 `shortcut` 的命令，组合键只能走原始 HID 用法码；标签无法翻译时
  /// 抛出 [ArgumentError]，由调用方提示用户改写方案，而不是发一条服务端不认识的命令。
  Future<void> sendShortcut(String label) async {
    final stroke = ShortcutMapper.fromLabel(label);
    if (stroke == null) {
      throw ArgumentError.value(label, 'label', '暂不支持这个快捷键，请改用 Ctrl + C 这类写法');
    }
    for (final pressed in [true, false]) {
      await _bus.request({
        'type': 'key_event',
        'usage': stroke.usage,
        'modifiers': stroke.modifiers,
        'pressed': pressed,
      });
    }
  }

  Future<void> sendMouseMoveOrdered(int dx, int dy) => mouseMove(dx, dy);
}

final inputControllerProvider = Provider<InputController>((ref) => InputController(ref.watch(remoteCommandBusProvider)));
