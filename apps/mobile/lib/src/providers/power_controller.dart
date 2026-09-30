import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'remote_command_bus.dart';

class PowerController {
  PowerController(this._bus);
  final RemoteCommandBus _bus;

  Future<void> sleep() => _bus.request({'type': 'sleep'}).then((_) {});
  Future<void> restart() => _bus.request({'type': 'restart'}).then((_) {});
  Future<void> shutdown() => _bus.request({'type': 'shutdown'}).then((_) {});
  Future<void> lock() => _bus.request({'type': 'lock'}).then((_) {});
}

final powerControllerProvider = Provider<PowerController>((ref) => PowerController(ref.watch(remoteCommandBusProvider)));
