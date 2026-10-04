import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../data/repositories/device_discovery_repository.dart';
import '../models/auth.dart';
import 'auth_state_writer.dart';
import 'dependencies.dart';
import 'settings_provider.dart';

/// 局域网设备发现与端点应用。
///
/// 负责周期扫描、把扫描结果写回状态，并在需要时把选中的端点写入设置；
/// 连接与配对决策留给调用方。
class DiscoveryCoordinator {
  DiscoveryCoordinator(
    this._ref,
    this._state, {
    DeviceDiscoveryRepository? discovery,
  }) : _discovery = discovery ?? _ref.read(deviceDiscoveryRepositoryProvider);

  final Ref _ref;
  final AuthStateWriter _state;
  final DeviceDiscoveryRepository _discovery;
  Timer? _discoveryTimer;

  /// 启动 5 秒周期的后台扫描。
  void startPeriodicRefresh() {
    _discoveryTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => unawaited(refresh()),
    );
  }

  /// 扫描局域网并写回设备列表。
  ///
  /// [updateEndpoint] 为 `false` 时只刷新列表，不改动已保存的端点。
  Future<void> refresh({bool updateEndpoint = true}) async {
    if (_state.isDisposed) return;
    try {
      final devices = await _discovery.scan(
        timeout: const Duration(seconds: 2),
      );
      if (_state.isDisposed) return;
      _state.update(_state.state.copyWith(discoveredDevices: devices));
      if (devices.isEmpty) return;
      final config = _ref.read(settingsProvider);
      DiscoveredPc? selected;
      if (config.pcId != null) {
        for (final device in devices) {
          if (device.id == config.pcId) selected = device;
        }
      } else if (devices.length == 1) {
        selected = devices.single;
      }
      if (selected == null) return;
      if (!updateEndpoint) return;
      // Keep a healthy direct-IP session stable. The discovered address is
      // applied after a disconnect or during initial setup, when it is needed
      // to recover from a DHCP change.
      if (_ref.read(remoteSessionRepositoryProvider).isReady &&
          _state.state.isAuthenticated) {
        return;
      }
      await applyEndpoint(selected);
    } catch (error) {
      // Keep the cached endpoint, but surface discovery failures for diagnosis.
      if (!_state.isDisposed) {
        _state.update(
          _state.state.copyWith(errorMessage: '局域网发现失败：$error'),
        );
      }
    }
  }

  /// 把发现到的端点写入设置。
  Future<void> applyEndpoint(DiscoveredPc device) {
    return _ref.read(settingsProvider.notifier).applyDiscoveredEndpoint(
      pcId: device.id,
      pcName: device.name,
      address: device.host,
      port: device.port,
    );
  }

  /// 停止后台扫描。
  void dispose() {
    _discoveryTimer?.cancel();
  }
}
