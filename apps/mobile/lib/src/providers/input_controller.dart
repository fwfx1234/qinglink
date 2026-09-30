import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/mouse_move_frame.dart';
import 'remote_command_bus.dart';

class InputController {
  InputController(this._bus);
  final RemoteCommandBus _bus;
  int _sequence = 0;

  Future<void> mouseMove(int dx, int dy) async {
    if (dx == 0 && dy == 0) return;
    final sent = await _bus.sendBinary(MouseMoveFrame(sequence: _sequence++, dx: dx, dy: dy, timestampMs: DateTime.now().millisecondsSinceEpoch & 0xffffffff).encode());
    if (!sent) await _bus.request({'type': 'mouse_move', 'dx': dx, 'dy': dy});
  }

  Future<void> mouseButton(String button) => _bus.request({'type': 'mouse_button', 'button': button});
  Future<void> mouseButtonState(String button, {required bool pressed}) => _bus.request({'type': 'mouse_button_state', 'button': button, 'pressed': pressed});
  Future<void> mouseWheel(int delta) => _bus.request({'type': 'mouse_wheel', 'delta': delta});
  Future<void> sendText(String text) async { if (text.isNotEmpty) await _bus.request({'type': 'send_text', 'text': text}); }
  Future<void> pressKey(String key) => _bus.request({'type': 'key_press', 'key': key});

  Future<void> sendShortcut(String shortcut) =>
      _bus.request({'type': 'shortcut', 'shortcut': shortcut});

  Future<void> sendMouseMoveOrdered(int dx, int dy) => mouseMove(dx, dy);
}

final inputControllerProvider = Provider<InputController>((ref) => InputController(ref.watch(remoteCommandBusProvider)));
