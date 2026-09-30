import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../data/repositories/device_discovery_repository.dart';
import '../data/repositories/pairing_repository.dart';
import '../data/repositories/remote_session_repository.dart';
import '../core/storage/preferences_repository.dart';

/// Infrastructure dependencies. Override these providers in tests or during
/// platform bootstrap without changing application controllers.
final deviceDiscoveryRepositoryProvider = Provider<DeviceDiscoveryRepository>(
  (ref) => QingLinkDeviceDiscoveryRepository(),
);

final pairingRepositoryProvider = Provider<PairingRepository>(
  (ref) => QingLinkPairingRepository(),
);

final remoteSessionRepositoryProvider =
    Provider<QingLinkRemoteSessionRepository>((ref) {
      final repository = QingLinkRemoteSessionRepository();
      ref.onDispose(repository.dispose);
      return repository;
    });


final preferencesRepositoryProvider = Provider<PreferencesRepository>(
  (ref) => SharedPreferencesRepository(),
);
