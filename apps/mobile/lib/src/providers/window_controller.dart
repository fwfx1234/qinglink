import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import 'remote_command_bus.dart';

class WindowController {
  WindowController(this._bus);
  final RemoteCommandBus _bus;

  /// 拉取 PC 上的可见窗口列表。
  ///
  /// 服务端的响应信封是 `{"type":"window_list","data":[{...}]}`（`WsServerResponse::WindowList`
  /// 承载 `Vec<WindowInfo>`）；列表为空或响应类型不符时返回空列表，由调用方给出提示。
  Future<List<WindowInfo>> list({bool forceRefresh = false}) async {
    final response = await _bus.request({'type': 'list_windows', 'force_refresh': forceRefresh});
    final data = response['data'];
    if (response['type'] != 'window_list' || data is! List) return const [];
    return data
        .whereType<Map>()
        .map((item) => WindowInfo.fromJson(Map<String, dynamic>.from(item)))
        .toList(growable: false);
  }

  Future<void> focus(int hwnd) => _bus.request({'type': 'focus_window', 'hwnd': hwnd}).then((_) {});
  Future<void> minimize(int hwnd) => _bus.request({'type': 'minimize_window', 'hwnd': hwnd}).then((_) {});
  Future<void> maximize(int hwnd) => _bus.request({'type': 'maximize_window', 'hwnd': hwnd}).then((_) {});
  Future<void> close(int hwnd) => _bus.request({'type': 'close_window', 'hwnd': hwnd}).then((_) {});
}

final windowControllerProvider = Provider<WindowController>((ref) => WindowController(ref.watch(remoteCommandBusProvider)));
