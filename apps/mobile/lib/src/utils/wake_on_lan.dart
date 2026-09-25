import 'dart:io';
import 'dart:typed_data';

/// Wake-on-LAN 魔法封包工具类
class WakeOnLan {
  static const List<int> defaultPorts = [
    9,
    7,
    47009,
    47998,
    47999,
    48000,
    48002,
    48010,
  ];

  /// 发送次数
  static const int sendCount = 3;

  /// 发送间隔（毫秒）
  static const int sendIntervalMs = 100;

  /// 发送魔法封包唤醒设备
  ///
  /// [macAddress] MAC 地址，格式如 "AA:BB:CC:DD:EE:FF" 或 "AA-BB-CC-DD-EE-FF"
  /// [port] 兼容端口；默认发送 Sunshine/Moonlight 使用的完整端口集合。
  /// [address] 目标地址，建议使用子网广播地址（如 192.168.1.255）
  ///   或 PC 的单播 IP 地址
  static Future<void> wake({
    required String macAddress,
    int port = 9,
    String? address,
    List<WakeNetwork>? networks,
  }) async {
    final targets = networks == null || networks.isEmpty
        ? planTargets(
            macAddress: macAddress,
            address: address,
            ports: _ports(port),
          )
        : planTargets(
            networks: networks,
            fallbackAddress: address,
            ports: _ports(port),
          );

    final socket = await RawDatagramSocket.bind(InternetAddress.anyIPv4, 0);
    socket.broadcastEnabled = true;
    final packets = <String, Uint8List>{};
    for (final target in targets) {
      packets.putIfAbsent(
        target.macAddress,
        () => buildMagicPacket(_parseMacAddress(target.macAddress)),
      );
    }

    try {
      var sentAny = false;
      Object? lastError;
      for (var i = 0; i < sendCount; i++) {
        for (final target in targets) {
          try {
            final sent = socket.send(
              packets[target.macAddress]!,
              InternetAddress(target.address),
              target.port,
            );
            if (sent == packets[target.macAddress]!.length) sentAny = true;
          } catch (error) {
            lastError = error;
          }
        }
        if (i < sendCount - 1) {
          await Future.delayed(const Duration(milliseconds: sendIntervalMs));
        }
      }
      if (!sentAny) {
        throw StateError(
          'Wake-on-LAN packet send failed${lastError == null ? '' : ': $lastError'}',
        );
      }
      await Future.delayed(const Duration(milliseconds: 100));
    } finally {
      socket.close();
    }
  }

  static List<int> _ports(int requestedPort) => requestedPort == 9
      ? defaultPorts
      : <int>{requestedPort, ...defaultPorts}.toList(growable: false);

  static List<WakeTarget> planTargets({
    String? macAddress,
    String? address,
    List<WakeNetwork>? networks,
    String? fallbackAddress,
    List<int>? ports,
  }) {
    final source =
        networks ??
        (macAddress == null
            ? const <WakeNetwork>[]
            : [
                WakeNetwork(
                  macAddress: macAddress,
                  ipv4Address: address ?? '',
                  prefixLength: 24,
                  broadcastAddress: '255.255.255.255',
                ),
              ]);
    final result = <String, WakeTarget>{};
    for (final network in source) {
      final addresses = <String>{
        '255.255.255.255',
        network.broadcastAddress,
        network.ipv4Address,
        if (fallbackAddress != null && fallbackAddress.isNotEmpty)
          fallbackAddress,
      };
      for (final targetAddress in addresses) {
        if (targetAddress.isEmpty) continue;
        for (final targetPort in ports ?? defaultPorts) {
          result['$targetAddress:$targetPort:${network.macAddress}'] =
              WakeTarget(
                macAddress: network.macAddress,
                address: targetAddress,
                port: targetPort,
              );
        }
      }
    }
    return result.values.toList(growable: false);
  }

  /// 构建魔法封包
  ///
  /// 魔法封包格式：
  /// - 前 6 个字节为 0xFF
  /// - 接着是 16 次重复的 MAC 地址（6 字节 × 16 = 96 字节）
  /// - 总长度：6 + 96 = 102 字节
  static Uint8List buildMagicPacket(Uint8List macBytes) {
    final packet = Uint8List(102);

    for (var i = 0; i < 6; i++) {
      packet[i] = 0xFF;
    }

    for (var i = 0; i < 16; i++) {
      final offset = 6 + i * 6;
      for (var j = 0; j < 6; j++) {
        packet[offset + j] = macBytes[j];
      }
    }

    return packet;
  }

  /// 解析 MAC 地址字符串为字节数组
  ///
  /// 支持格式：
  /// - "AA:BB:CC:DD:EE:FF"
  /// - "AA-BB-CC-DD-EE-FF"
  /// - "AABBCCDDEEFF"
  static Uint8List _parseMacAddress(String macAddress) {
    final cleanMac = macAddress.replaceAll(RegExp(r'[:-]'), '').toUpperCase();

    if (cleanMac.length != 12) {
      throw FormatException('Invalid MAC address format: $macAddress');
    }

    if (!RegExp(r'^[0-9A-F]{12}$').hasMatch(cleanMac)) {
      throw FormatException('Invalid MAC address characters: $macAddress');
    }

    final bytes = Uint8List(6);
    for (var i = 0; i < 6; i++) {
      final hex = cleanMac.substring(i * 2, i * 2 + 2);
      bytes[i] = int.parse(hex, radix: 16);
    }

    return bytes;
  }

  /// 验证 MAC 地址格式是否有效
  static bool isValidMacAddress(String macAddress) {
    try {
      _parseMacAddress(macAddress);
      return true;
    } catch (_) {
      return false;
    }
  }
}

class WakeNetwork {
  const WakeNetwork({
    required this.macAddress,
    required this.ipv4Address,
    required this.prefixLength,
    required this.broadcastAddress,
  });
  final String macAddress;
  final String ipv4Address;
  final int prefixLength;
  final String broadcastAddress;
  factory WakeNetwork.fromJson(Map<String, dynamic> json) => WakeNetwork(
    macAddress: json['mac_address'] as String,
    ipv4Address: json['ipv4_address'] as String,
    prefixLength: (json['prefix_length'] as num).toInt(),
    broadcastAddress: json['broadcast_address'] as String,
  );
  Map<String, dynamic> toJson() => {
    'mac_address': macAddress,
    'ipv4_address': ipv4Address,
    'prefix_length': prefixLength,
    'broadcast_address': broadcastAddress,
  };
}

class WakeTarget {
  const WakeTarget({
    required this.macAddress,
    required this.address,
    required this.port,
  });
  final String macAddress;
  final String address;
  final int port;
}

/// 根据 IPv4 地址和子网前缀计算定向广播地址。
String getSubnetBroadcast(String? ipAddress, int prefix) {
  if (ipAddress == null || ipAddress.isEmpty) return '255.255.255.255';
  if (prefix <= 0 || prefix > 32) return '255.255.255.255';

  final parts = ipAddress.split('.');
  if (parts.length != 4) return '255.255.255.255';
  final octets = parts.map(int.tryParse).toList();
  if (octets.any((part) => part == null || part < 0 || part > 255)) {
    return '255.255.255.255';
  }

  final ip =
      (octets[0]! << 24) | (octets[1]! << 16) | (octets[2]! << 8) | octets[3]!;
  final mask = prefix == 32 ? 0xFFFFFFFF : (0xFFFFFFFF << (32 - prefix));
  final broadcast = ip | (~mask & 0xFFFFFFFF);

  return '${(broadcast >> 24) & 0xFF}.${(broadcast >> 16) & 0xFF}.'
      '${(broadcast >> 8) & 0xFF}.${broadcast & 0xFF}';
}
