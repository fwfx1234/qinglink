import '../../models/auth.dart';
import '../../services/qinglink_discovery.dart';

abstract interface class DeviceDiscoveryRepository {
  Future<List<DiscoveredPc>> scan({Duration timeout});
}

final class QingLinkDeviceDiscoveryRepository
    implements DeviceDiscoveryRepository {
  QingLinkDeviceDiscoveryRepository({QingLinkDiscovery? discovery})
    : _discovery = discovery ?? QingLinkDiscovery();

  final QingLinkDiscovery _discovery;

  @override
  Future<List<DiscoveredPc>> scan({Duration timeout = const Duration(seconds: 2)}) async {
    final devices = await _discovery.scan(timeout: timeout);
    return devices
        .map(
          (device) => DiscoveredPc(
            id: device.id,
            name: device.name,
            host: device.host,
            port: device.port,
            version: device.version,
          ),
        )
        .toList(growable: false);
  }
}
