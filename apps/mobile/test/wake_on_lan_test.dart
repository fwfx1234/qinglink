import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/utils/wake_on_lan.dart';

void main() {
  test('builds a standard 102-byte magic packet', () {
    final mac = Uint8List.fromList([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    final packet = WakeOnLan.buildMagicPacket(mac);

    expect(packet, hasLength(102));
    expect(packet.take(6), everyElement(0xFF));
    for (var repeat = 0; repeat < 16; repeat++) {
      expect(packet.sublist(6 + repeat * 6, 12 + repeat * 6), mac);
    }
  });

  test('calculates a directed subnet broadcast address', () {
    expect(getSubnetBroadcast('192.168.1.42', 24), '192.168.1.255');
    expect(getSubnetBroadcast('10.20.30.40', 16), '10.20.255.255');
    expect(getSubnetBroadcast('10.20.30.40', 32), '10.20.30.40');
    expect(getSubnetBroadcast('invalid', 24), '255.255.255.255');
  });

  test('plans Sunshine-compatible targets and ports for every network', () {
    final targets = WakeOnLan.planTargets(
      networks: const [
        WakeNetwork(
          macAddress: 'AA:BB:CC:DD:EE:FF',
          ipv4Address: '192.168.1.42',
          prefixLength: 24,
          broadcastAddress: '192.168.1.255',
        ),
      ],
    );
    expect(targets.map((target) => target.address).toSet(), {
      '255.255.255.255',
      '192.168.1.255',
      '192.168.1.42',
    });
    expect(targets.map((target) => target.port).toSet(), {
      7,
      9,
      47009,
      47998,
      47999,
      48000,
      48002,
      48010,
    });
    expect(targets, hasLength(24));
  });

  test('plans each network independently', () {
    final targets = WakeOnLan.planTargets(
      networks: const [
        WakeNetwork(
          macAddress: 'AA:BB:CC:DD:EE:FF',
          ipv4Address: '192.168.1.42',
          prefixLength: 24,
          broadcastAddress: '192.168.1.255',
        ),
        WakeNetwork(
          macAddress: '11:22:33:44:55:66',
          ipv4Address: '10.0.0.8',
          prefixLength: 24,
          broadcastAddress: '10.0.0.255',
        ),
      ],
    );
    expect(targets, hasLength(48));
    expect(
      targets.where((target) => target.macAddress == '11:22:33:44:55:66'),
      hasLength(24),
    );
  });
}
