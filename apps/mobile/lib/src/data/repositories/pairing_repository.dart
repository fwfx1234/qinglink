import '../../services/qinglink_pairing.dart';
import '../../core/network/http_transport.dart';

abstract interface class PairingRepository {
  Future<String> request({
    required String baseUrl,
    required String deviceId,
    required String deviceName,
  });

  Future<Map<String, dynamic>> confirm({
    required String baseUrl,
    required String requestId,
    required String pin,
    required String deviceId,
    required String deviceName,
  });

  Future<void> cancel({required String baseUrl, required String requestId});
}

final class QingLinkPairingRepository implements PairingRepository {
  QingLinkPairingRepository({QingLinkPairing? pairing, HttpTransport? transport})
    : _pairing = pairing ?? QingLinkPairing(transport: transport);

  final QingLinkPairing _pairing;

  QingLinkPairing get pairing => _pairing;

  @override
  Future<String> request({
    required String baseUrl,
    required String deviceId,
    required String deviceName,
  }) => _pairing.requestAt(
    baseUrl,
    deviceId: deviceId,
    deviceName: deviceName,
  );

  @override
  Future<Map<String, dynamic>> confirm({
    required String baseUrl,
    required String requestId,
    required String pin,
    required String deviceId,
    required String deviceName,
  }) => _pairing.confirmAt(
    baseUrl,
    requestId: requestId,
    pin: pin,
    deviceId: deviceId,
    deviceName: deviceName,
  );

  @override
  Future<void> cancel({required String baseUrl, required String requestId}) =>
      _pairing.cancelAt(baseUrl, requestId);
}
