import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/services/qinglink_discovery.dart';

void main() {
  test('UDP records deduplicate by stable PC id and keep the latest IP', () {
    final devices = QingLinkDiscovery.deduplicate(const [
      QingLinkDevice(
        id: 'pc-1',
        name: 'Desk',
        host: '192.168.1.10',
        port: 3721,
        version: 2,
      ),
      QingLinkDevice(
        id: 'pc-1',
        name: 'Desk',
        host: '192.168.1.42',
        port: 3721,
        version: 2,
      ),
    ]);

    expect(devices, hasLength(1));
    expect(devices.single.id, 'pc-1');
    expect(devices.single.host, '192.168.1.42');
  });

  test('unsupported protocol versions are ignored', () {
    final devices = QingLinkDiscovery.deduplicate(const [
      QingLinkDevice(
        id: 'legacy',
        name: 'Old PC',
        host: '192.168.1.5',
        port: 3721,
        version: 1,
      ),
    ]);
    expect(devices, isEmpty);
  });

  test('formats IPv6 endpoints for HTTP URLs', () {
    const device = QingLinkDevice(
      id: 'pc-v6',
      name: 'Desk',
      host: 'fe80::42',
      port: 3721,
      version: 2,
    );
    expect(device.baseUrl, 'http://[fe80::42]:3721');
  });
}
