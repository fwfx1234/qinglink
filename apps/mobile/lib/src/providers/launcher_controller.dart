import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'remote_command_bus.dart';

class LauncherController {
  LauncherController(this._bus);
  final RemoteCommandBus _bus;

  Future<String?> icon(String id) async {
    final response = await _bus.request({'type': 'get_launcher_icon', 'id': id});
    if (response['type'] != 'launcher_icon') return null;
    final data = response['data'];
    return data is Map<String, dynamic> ? data['icon_base64'] as String? : null;
  }

  Future<void> launch(String id) => _bus.request({'type': 'launch_scanner_app', 'id': id}).then((_) {});
  Future<void> launchPath(String path, {List<String> args = const []}) => _bus.request({'type': 'launch_app', 'path': path, 'args': args}).then((_) {});
  Future<bool> update({required String id, required String name, required String exePath, String? iconBase64, List<String>? args}) async {
    final payload = <String, dynamic>{'type': 'update_scanner_app', 'id': id, 'name': name, 'exe_path': exePath};
    if (iconBase64 != null) payload['icon_base64'] = iconBase64;
    if (args != null) payload['args'] = args;
    final response = await _bus.request(payload);
    return response['type'] != 'error';
  }

  Future<bool> delete(String id) async {
    final response = await _bus.request({'type': 'delete_scanner_app', 'id': id});
    return response['type'] != 'error';
  }

  Future<void> rename(String id, String originalName, String name) => _bus
      .request({'type': 'rename_scanner_app', 'id': id, 'original_name': originalName, 'name': name})
      .then((_) {});

  Future<void> refreshIcon(String id) => _bus
      .request({'type': 'refresh_scanner_app_icon', 'id': id})
      .then((_) {});

  Future<void> refreshCatalog(String kind) =>
      _bus.request({'type': 'refresh_launcher_catalog', 'kind': kind}).then((_) {});

  Future<String?> add({required String name, required String exePath}) async {
    final response = await _bus.request({
      'type': 'add_scanner_app',
      'name': name,
      'exe_path': exePath,
    });
    final data = response['data'];
    return data is Map ? data['id'] as String? : response['id'] as String?;
  }
}

final launcherControllerProvider = Provider<LauncherController>((ref) => LauncherController(ref.watch(remoteCommandBusProvider)));
