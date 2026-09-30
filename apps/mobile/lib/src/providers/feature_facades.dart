import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import '../models/process_info.dart';
import '../models/system_status.dart';
import 'remote_command_bus.dart';
import 'system_controller.dart';
import 'process_controller.dart';
import 'library_controller.dart';
import 'dependencies.dart';
import 'auth_provider.dart';
import '../models/auth.dart';

final connectionReadyProvider = Provider<bool>((ref) {
  final auth = ref.watch(connectionProvider);
  return auth.connection == AppConnectionState.connected;
});

/// Connection-only projection used by status bars and guards.
final connectionControllerProvider = Provider<RemoteControlConnectionFacade>(
  (ref) => RemoteControlConnectionFacade(ref),
);

/// System feature projection. It intentionally exposes only system actions.
final systemControllerFacadeProvider = Provider<SystemControllerFacade>(
  (ref) => SystemControllerFacade(ref),
);

/// Process feature projection.
final processControllerFacadeProvider = Provider<ProcessControllerFacade>(
  (ref) => ProcessControllerFacade(ref),
);

/// Launcher/library feature projection.
final libraryControllerFacadeProvider = Provider<LibraryControllerFacade>(
  (ref) => LibraryControllerFacade(ref),
);

class RemoteControlConnectionFacade {
  RemoteControlConnectionFacade(this._ref);

  final Ref _ref;

  bool get isConnected => _ref.read(remoteSessionRepositoryProvider).isReady;
  bool get isOnline => _ref.read(remoteSessionRepositoryProvider).isReady;

  Future<bool> waitUntilReady() =>
      _ref.read(remoteCommandBusProvider).waitUntilReady();
}

class SystemControllerFacade {
  SystemControllerFacade(this._ref);

  final Ref _ref;

  SystemState get state => _ref.read(systemControllerProvider);
  SystemStatus? get status => state.status;
  ForegroundResponse? get foreground => state.foreground;
  VolumeStatus? get volume => _ref.read(systemControllerProvider).volume;

  Future<void> refreshStatus() =>
      _ref.read(systemControllerProvider.notifier).refreshStatus();
  Future<void> refreshForeground() =>
      _ref.read(systemControllerProvider.notifier).refreshForeground();
  Future<UnlockStatus?> unlock({String? password}) =>
      _ref.read(systemControllerProvider.notifier).refreshUnlock();
}

class ProcessControllerFacade {
  ProcessControllerFacade(this._ref);

  final Ref _ref;

  ProcessState get state => _ref.read(processControllerProvider);
  List<ProcessInfo> get processes => state.items;

  Future<void> refresh({String? search, String? filter}) => _ref
      .read(processControllerProvider.notifier)
      .refresh(search: search, filter: filter);
  Future<void> loadMore() =>
      _ref.read(processControllerProvider.notifier).loadMore();
  Future<void> kill(int pid) =>
      _ref.read(processControllerProvider.notifier).kill(pid);
}

class LibraryControllerFacade {
  LibraryControllerFacade(this._ref);

  final Ref _ref;

  LibraryState get state => _ref.read(libraryControllerProvider);
  List<LauncherItem> get applications => state.items;
  List<LauncherItem> get games => const [];
  List<LauncherItem> get recent => state.recent;

  Future<bool> refresh({String? kind, String? query}) => _ref
      .read(libraryControllerProvider.notifier)
      .refresh(kind: kind, query: query);
  Future<bool> refreshRecent() =>
      _ref.read(libraryControllerProvider.notifier).refreshRecent();
}
