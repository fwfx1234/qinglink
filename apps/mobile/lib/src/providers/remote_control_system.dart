part of 'remote_control_provider.dart';

@visibleForTesting
Map<String, dynamic> autoUnlockRequestPayload({String? lockPassword}) {
  final payload = <String, dynamic>{'type': 'auto_unlock'};
  if (lockPassword != null && lockPassword.isNotEmpty) {
    payload['pin'] = lockPassword;
  }
  return payload;
}

/// 系统状态、前台窗口、解锁、子网/MAC 相关动作。
mixin _SystemStatusActions on _RemoteControlBase {
  // ========== System Status ==========

  @override
  Future<void> refreshSystemStatus() async {
    try {
      final resp = await _wsRequest({'type': 'get_system_status'});
      final status = WsEventAdapter.parseSystemStatus(resp);
      if (status != null) {
        _setState(
          state.copyWith(
            isConnected: true,
            systemStatus: status,
            clearError: true,
          ),
        );
      }
    } catch (e) {
      _setState(state.copyWith(errorMessage: e.toString()));
    }
  }

  // ========== Foreground ==========

  @override
  Future<void> refreshForeground() async {
    try {
      final resp = await _wsRequest({'type': 'get_foreground'});
      final fg = WsEventAdapter.parseForegroundResponse(resp);
      if (fg != null) {
        _setState(state.copyWith(foregroundApp: fg));
      }
    } catch (_) {}
  }

  // ========== Unlock ==========
  @override
  Future<UnlockStatus?> getUnlockStatus() async {
    try {
      final resp = await _wsRequest({'type': 'get_unlock_status'});
      if (resp['type'] == 'unlock_status') {
        final status = WsEventAdapter.parseUnlockStatus(resp);
        if (status != null) {
          _setState(state.copyWith(unlockStatus: status));
        }
        return status;
      }
    } catch (_) {}
    return null;
  }

  Future<UnlockStatus?> autoUnlock({String? lockPassword}) async {
    await _wsCommand(
      autoUnlockRequestPayload(lockPassword: lockPassword),
      timeout: const Duration(seconds: 25),
    );
    return getUnlockStatus();
  }

  /// Unlock using the phone's configured credential policy. A configured PIN
  /// always sends that PIN; without one, use the confirmation flow. An
  /// explicit user request is always sent; lock state is refreshed only after
  /// the unlock action finishes.
  Future<UnlockStatus?> unlockWithMode({String? lockPassword}) async {
    final configuredPin = lockPassword?.trim();
    return autoUnlock(
      lockPassword: configuredPin == null || configuredPin.isEmpty
          ? null
          : configuredPin,
    );
  }

  Future<LockInputStatus?> getLockInputStatus() async {
    try {
      final resp = await _wsRequest({'type': 'get_lock_input_status'});
      if (resp['type'] == 'lock_input_status') {
        final data = resp['data'];
        if (data is Map<String, dynamic>) {
          final status = LockInputStatus.fromJson(data);
          _setState(state.copyWith(lockInputStatus: status));
          return status;
        }
      }
    } catch (_) {}
    return null;
  }

  Future<void> submitLockPin(String pin) async {
    await _wsCommand({'type': 'submit_lock_pin', 'pin': pin});
  }

  @override
  Future<SecureDesktopStatus?> getSecureDesktopStatus() async {
    try {
      final response = await _wsRequest({'type': 'get_secure_desktop_status'});
      final data = response['data'];
      if (data is Map<String, dynamic>) {
        final status = SecureDesktopStatus.fromJson(data);
        _setState(state.copyWith(secureDesktopStatus: status));
        return status;
      }
    } catch (e) {
      debugPrint('getSecureDesktopStatus failed: $e');
    }
    return null;
  }

  Future<void> sendSecureAttention() async {
    debugPrint('Sending secure attention sequence (Ctrl+Alt+Del)');
    await _wsCommand({'type': 'secure_attention'});
    await getSecureDesktopStatus();
  }

  Future<void> sendSecureAscii(String text) async {
    if (text.isEmpty ||
        text.length > 256 ||
        !text.codeUnits.every((unit) => unit >= 0x20 && unit <= 0x7e)) {
      throw ArgumentError('Secure input only accepts printable ASCII');
    }
    debugPrint('Sending secure ASCII input (length: ${text.length})');
    await _wsCommand({'type': 'send_secure_ascii', 'text': text});
  }

  // ========== System ==========

  int? _subnetPrefix;

  int? get subnetPrefix => _subnetPrefix;

  Future<int?> refreshSubnetPrefix() async {
    try {
      final resp = await _wsRequest({'type': 'get_subnet_prefix'});
      if (resp['type'] == 'subnet_prefix') {
        final data = resp['data'] as Map<String, dynamic>?;
        final prefix = data?['prefix'] as int?;
        if (prefix != null) {
          _subnetPrefix = prefix;
          return prefix;
        }
      }
    } catch (_) {}
    return null;
  }

  Future<String?> getMacAddress() async {
    try {
      final resp = await _wsRequest({'type': 'get_mac_address'});
      if (resp['type'] == 'mac_address') {
        final data = resp['data'] as Map<String, dynamic>?;
        return data?['mac_address'] as String?;
      }
    } catch (_) {}
    return null;
  }

  Future<List<WakeNetwork>> getWakeNetworks() async {
    try {
      final resp = await _wsRequest({'type': 'get_wake_networks'});
      if (resp['type'] == 'wake_networks') {
        final data = resp['data'];
        if (data is Map<String, dynamic> && data['networks'] is List) {
          return (data['networks'] as List)
              .whereType<Map>()
              .map((e) => WakeNetwork.fromJson(Map<String, dynamic>.from(e)))
              .toList(growable: false);
        }
      }
    } catch (error) {
      debugPrint('getWakeNetworks failed: $error');
    }
    return const [];
  }
}
