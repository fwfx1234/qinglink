/// Wire DTOs for the versioned QingLink HTTP pairing contract.
///
/// These classes intentionally contain no UI or connection state. They mirror
/// the Rust `qinglink-protocol` DTOs and are kept JSON-only for compatibility.
library;

class PairingRequestDto {
  const PairingRequestDto({required this.deviceId, required this.deviceName});

  final String deviceId;
  final String deviceName;

  Map<String, dynamic> toJson() => {
    'device_id': deviceId,
    'device_name': deviceName,
  };
}

class PairingConfirmDto {
  const PairingConfirmDto({
    required this.requestId,
    required this.pin,
    required this.deviceId,
    required this.deviceName,
  });

  final String requestId;
  final String pin;
  final String deviceId;
  final String deviceName;

  Map<String, dynamic> toJson() => {
    'request_id': requestId,
    'pin': pin,
    'device_id': deviceId,
    'device_name': deviceName,
  };
}

class PairingRequestResponseDto {
  const PairingRequestResponseDto({
    required this.requestId,
    required this.expiresAtUnix,
  });

  final String requestId;
  final int expiresAtUnix;

  factory PairingRequestResponseDto.fromJson(Map<String, dynamic> json) =>
      PairingRequestResponseDto(
        requestId: json['request_id'] as String,
        expiresAtUnix: (json['expires_at_unix'] as num?)?.toInt() ?? 0,
      );
}

class PairingTokenResponseDto {
  const PairingTokenResponseDto({
    required this.token,
    required this.pcId,
    required this.pcName,
    required this.macAddress,
    this.wakeNetworks = const [],
  });

  final String token;
  final String pcId;
  final String pcName;
  final String? macAddress;
  final List<WakeNetworkAdapterDto> wakeNetworks;

  factory PairingTokenResponseDto.fromJson(Map<String, dynamic> json) =>
      PairingTokenResponseDto(
        token: json['token'] as String,
        pcId: json['pc_id'] as String,
        pcName: json['pc_name'] as String,
        macAddress: json['mac_address'] as String?,
        wakeNetworks: (json['wake_networks'] as List<dynamic>? ?? const [])
            .whereType<Map>()
            .map(
              (e) =>
                  WakeNetworkAdapterDto.fromJson(Map<String, dynamic>.from(e)),
            )
            .toList(growable: false),
      );
}

class WakeNetworkAdapterDto {
  const WakeNetworkAdapterDto({
    required this.macAddress,
    required this.ipv4Address,
    required this.prefixLength,
    required this.broadcastAddress,
  });
  final String macAddress;
  final String ipv4Address;
  final int prefixLength;
  final String broadcastAddress;
  factory WakeNetworkAdapterDto.fromJson(Map<String, dynamic> json) =>
      WakeNetworkAdapterDto(
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

class HealthResponseDto {
  const HealthResponseDto({
    required this.pcId,
    required this.name,
    required this.version,
    required this.port,
  });

  final String pcId;
  final String name;
  final int version;
  final int port;

  factory HealthResponseDto.fromJson(Map<String, dynamic> json) =>
      HealthResponseDto(
        pcId: json['pc_id'] as String,
        name: json['name'] as String,
        version: (json['version'] as num).toInt(),
        port: (json['port'] as num).toInt(),
      );
}
