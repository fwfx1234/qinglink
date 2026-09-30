import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import 'remote_command_bus.dart';

class WindowController {
  WindowController(this._bus);
  final RemoteCommandBus _bus;

  Future<List<WindowInfo>> list({bool forceRefresh = false}) async {
    final response = await _bus.request({'type': 'list_windows', 'force_refresh': forceRefresh});
    final data = response['data'];
    if (response['type'] != 'windows' || data is! Map<String, dynamic> || data['windows'] is! List) return const [];
    return (data['windows'] as List).map((item) => WindowInfo.fromJson(item as Map<String, dynamic>)).toList(growable: false);
  }

  Future<void> focus(int hwnd) => _bus.request({'type': 'focus_window', 'hwnd': hwnd}).then((_) {});
  Future<void> minimize(int hwnd) => _bus.request({'type': 'minimize_window', 'hwnd': hwnd}).then((_) {});
  Future<void> maximize(int hwnd) => _bus.request({'type': 'maximize_window', 'hwnd': hwnd}).then((_) {});
  Future<void> close(int hwnd) => _bus.request({'type': 'close_window', 'hwnd': hwnd}).then((_) {});
}

final windowControllerProvider = Provider<WindowController>((ref) => WindowController(ref.watch(remoteCommandBusProvider)));
