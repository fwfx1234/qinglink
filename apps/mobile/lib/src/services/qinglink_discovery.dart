import 'dart:async';
import 'dart:convert';
import 'dart:io';

class QingLinkDevice {
  const QingLinkDevice({
    required this.id,
    required this.name,
    required this.host,
    required this.port,
    required this.version,
  });

  final String id;
  final String name;
  final String host;
  final int port;
  final int version;

  String get baseUrl {
    final formattedHost = host.contains(':') ? '[$host]' : host;
    return 'http://$formattedHost:$port';
  }
}

class QingLinkDiscovery {
  static List<QingLinkDevice> deduplicate(Iterable<QingLinkDevice> records) {
    final devices = <String, QingLinkDevice>{};
    for (final device in records) {
      if (device.version >= 2) devices[device.id] = device;
    }
    return devices.values.toList();
  }

  Future<List<QingLinkDevice>> scan({
    Duration timeout = const Duration(seconds: 3),
  }) async {
    return _scanUdp(timeout);
  }

  Future<List<QingLinkDevice>> _scanUdp(Duration timeout) async {
    final socket = await RawDatagramSocket.bind(InternetAddress.anyIPv4, 0);
    socket.broadcastEnabled = true;
    final result = <String, QingLinkDevice>{};
    final deadline = DateTime.now().add(timeout);
    socket.send(
      utf8.encode('qinglink-discover-v1'),
      InternetAddress('255.255.255.255'),
      3723,
    );
    try {
      await for (final event in socket.timeout(
        timeout,
        onTimeout: (sink) => sink.close(),
      )) {
        if (event != RawSocketEvent.read) continue;
        final datagram = socket.receive();
        if (datagram == null) continue;
        try {
          final json =
              jsonDecode(utf8.decode(datagram.data)) as Map<String, dynamic>;
          final id = json['id']?.toString();
          if (id == null || id.isEmpty) continue;
          result[id] = QingLinkDevice(
            id: id,
            name: json['name']?.toString() ?? id,
            host: datagram.address.address,
            port: (json['port'] as num?)?.toInt() ?? 3721,
            version: (json['version'] as num?)?.toInt() ?? 0,
          );
        } catch (_) {}
        if (DateTime.now().isAfter(deadline)) break;
      }
    } finally {
      socket.close();
    }
    return deduplicate(result.values);
  }

}
