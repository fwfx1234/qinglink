part of 'remote_control_provider.dart';

/// 远程窗口管理与前台窗口操作相关动作。
mixin _WindowActions on _RemoteControlBase {
  // ========== Window Control ==========

  Future<List<WindowInfo>> listWindows({bool forceRefresh = false}) async {
    final resp = await _wsRequest({
      'type': 'list_windows',
      if (forceRefresh) 'force_refresh': true,
    });
    if (resp['type'] != 'window_list') {
      throw StateError('PC 返回了无效的窗口列表响应');
    }
    final data = resp['data'];
    if (data is! List) {
      throw StateError('PC 返回的窗口列表数据无效');
    }
    return data
        .whereType<Map>()
        .map((item) => WindowInfo.fromJson(Map<String, dynamic>.from(item)))
        .where((window) => window.hwnd != 0)
        .toList();
  }

  Future<void> focusWindow(int hwnd) async {
    await _wsCommand({'type': 'focus_window', 'hwnd': hwnd});
    await refreshForeground();
  }

  Future<void> switchWindow(String direction) async {
    await _wsCommand({'type': 'switch_window', 'direction': direction});
    await refreshForeground();
  }

  Future<void> minimizeWindow(int hwnd) async {
    await _wsCommand({'type': 'minimize_window', 'hwnd': hwnd});
  }

  Future<void> maximizeWindow(int hwnd) async {
    await _wsCommand({'type': 'maximize_window', 'hwnd': hwnd});
  }

  Future<void> restoreWindow(int hwnd) async {
    await _wsCommand({'type': 'restore_window', 'hwnd': hwnd});
  }

  Future<void> closeWindow(int hwnd) async {
    await _wsCommand({'type': 'close_window', 'hwnd': hwnd});
  }

  Future<void> minimizeForeground() async {
    await _wsCommand({'type': 'minimize_foreground'});
  }

  Future<void> maximizeForeground() async {
    await _wsCommand({'type': 'maximize_foreground'});
  }

  Future<void> closeForeground() async {
    await _wsCommand({'type': 'close_foreground'});
  }
}
