part of 'remote_control_provider.dart';

/// 鼠标键盘输入、显示设置、音量与音频输出相关动作。
mixin _InputActions on _RemoteControlBase {
  // ========== Display & remote input ==========

  Future<List<DisplayInfo>> getDisplays() async {
    _setState(state.copyWith(isLoadingDisplays: true, displayError: ''));
    try {
      final resp = await _wsRequest({'type': 'get_displays'});
      if (resp['type'] != 'displays') {
        throw StateError('PC 返回了无效的显示器响应');
      }
      final data = resp['data'];
      if (data is! Map<String, dynamic> || data['displays'] is! List) {
        throw StateError('PC 返回的显示器数据无效');
      }
      final displays = (data['displays'] as List)
          .map((item) => DisplayInfo.fromJson(item as Map<String, dynamic>))
          .toList();
      _setState(
        state.copyWith(
          displays: displays,
          isLoadingDisplays: false,
          displayError: '',
        ),
      );
      return displays;
    } catch (error) {
      _setState(
        state.copyWith(
          isLoadingDisplays: false,
          displayError: error.toString(),
        ),
      );
      return const [];
    }
  }

  Future<void> setDisplayMode(String displayId, DisplayModeInfo mode) async {
    await _wsCommand({
      'type': 'set_display_mode',
      'display_id': displayId,
      'width': mode.width,
      'height': mode.height,
      'refresh_millihz': mode.refreshMillihz,
    });
    await getDisplays();
  }

  Future<void> sendMouseMove(int dx, int dy) async {
    if (dx == 0 && dy == 0) return;
    // Coalesce high-frequency touchpad events to one frame per ~16ms. This
    // keeps the service pipe responsive and avoids spawning helpers per pixel.
    _pendingMouseDx += dx;
    _pendingMouseDy += dy;
    if (_mouseFlushTimer != null) return;
    _mouseFlushTimer = Timer(const Duration(milliseconds: 16), () {
      _mouseFlushTimer = null;
      final flushDx = _pendingMouseDx;
      final flushDy = _pendingMouseDy;
      _pendingMouseDx = 0;
      _pendingMouseDy = 0;
      if (flushDx != 0 || flushDy != 0) {
        unawaited(_sendMouseMoveFrame(flushDx, flushDy));
      }
    });
  }

  /// Sends one movement frame immediately. Used by the touchpad command
  /// queue when a button-state transition must precede the movement.
  Future<void> sendMouseMoveOrdered(int dx, int dy) async {
    if (dx == 0 && dy == 0) return;
    final sent = await _session.sendBinaryFrame(
      MouseMoveFrame(
        sequence: _mouseSequence,
        dx: dx,
        dy: dy,
        timestampMs: DateTime.now().millisecondsSinceEpoch & 0xffffffff,
      ).encode(),
    );
    _mouseSequence = (_mouseSequence + 1) & 0xffff;
    if (!sent) {
      await _wsSend({'type': 'mouse_move', 'dx': dx, 'dy': dy});
    }
  }

  Future<void> _sendMouseMoveFrame(int dx, int dy) async {
    final frame = MouseMoveFrame(
      sequence: _mouseSequence,
      dx: dx,
      dy: dy,
      timestampMs: DateTime.now().millisecondsSinceEpoch & 0xffffffff,
    );
    _mouseSequence = (_mouseSequence + 1) & 0xffff;
    try {
      // 优先走独立二进制输入通道，与慢命令分离避免队头阻塞。
      final sent = await _session.sendBinaryFrame(frame.encode());
      if (!sent) {
        // 输入通道暂不可用时降级为 JSON mouse_move。
        await _wsSend({'type': 'mouse_move', 'dx': dx, 'dy': dy});
      }
    } catch (_) {
      // 高频输入路径不阻塞 UI，连接状态由 wsState 统一反映。
    }
  }

  Future<void> mouseButton(String button) async {
    await _wsCommand({'type': 'mouse_button', 'button': button});
  }

  Future<void> mouseButtonState(String button, {required bool pressed}) async {
    await _wsCommand({
      'type': 'mouse_button_state',
      'button': button,
      'pressed': pressed,
    });
  }

  Future<void> mouseWheel(int delta) async {
    await _wsCommand({'type': 'mouse_wheel', 'delta': delta});
  }

  Future<void> sendText(String text) async {
    if (text.isEmpty) return;
    await _wsCommand({'type': 'send_text', 'text': text});
  }

  /// 与服务端 is_supported_remote_key 白名单保持一致。
  static const supportedRemoteKeys = <String>{
    'backspace',
    'enter',
    'escape',
    'tab',
    'delete',
    'home',
    'end',
    'pageup',
    'pagedown',
    'up',
    'down',
    'left',
    'right',
    'space',
    'f1',
    'f2',
    'f3',
    'f4',
    'f5',
    'f6',
    'f7',
    'f8',
    'f9',
    'f10',
    'f11',
    'f12',
  };

  Future<void> pressKey(String key) async {
    if (!supportedRemoteKeys.contains(key)) {
      throw ArgumentError.value(key, 'key', 'Unsupported remote key');
    }
    await _wsCommand({'type': 'key_press', 'key': key});
  }

  Future<void> keyEvent({
    required int usage,
    int modifiers = 0,
    required bool pressed,
  }) async {
    await _wsCommand({
      'type': 'key_event',
      'usage': usage,
      'modifiers': modifiers,
      'pressed': pressed,
    });
  }

  /// Sends one configured shortcut stroke to the remote PC.
  Future<void> sendShortcut(String label) async {
    final stroke = ShortcutMapper.fromLabel(label);
    if (stroke == null) {
      throw ArgumentError.value(label, 'label', 'Unsupported shortcut');
    }
    await keyEvent(
      usage: stroke.usage,
      modifiers: stroke.modifiers,
      pressed: true,
    );
    await keyEvent(
      usage: stroke.usage,
      modifiers: stroke.modifiers,
      pressed: false,
    );
  }

  // ========== Volume & Media ==========

  Future<VolumeStatus?> getVolume() async {
    try {
      final resp = await _wsRequest({'type': 'get_volume'});
      if (resp['type'] == 'volume_status') {
        final status = WsEventAdapter.parseVolumeStatus(resp);
        if (status != null) {
          _setState(state.copyWith(volumeStatus: status));
        }
        return status;
      }
    } catch (_) {}
    return null;
  }

  Future<List<AudioOutputDevice>> getAudioOutputs() async {
    _setState(
      state.copyWith(isLoadingAudioOutputs: true, audioOutputsError: ''),
    );
    try {
      final resp = await _wsRequest({'type': 'get_audio_outputs'});
      if (resp['type'] != 'audio_outputs') {
        throw StateError('PC 返回了无效的音频输出设备响应');
      }
      final outputs = WsEventAdapter.parseAudioOutputs(resp);
      if (outputs == null) {
        throw StateError('PC 返回的音频输出设备数据无效');
      }
      _setState(
        state.copyWith(
          audioOutputs: outputs,
          isLoadingAudioOutputs: false,
          audioOutputsError: '',
        ),
      );
      return outputs;
    } catch (error) {
      _setState(
        state.copyWith(
          isLoadingAudioOutputs: false,
          audioOutputsError: error.toString(),
        ),
      );
      return const [];
    }
  }

  Future<void> setAudioOutput(String deviceId) async {
    await _wsCommand({'type': 'set_audio_output', 'device_id': deviceId});
    await Future.wait([getAudioOutputs(), getVolume()]);
  }

  Future<void> setVolume(int percent) async {
    final clamped = percent.clamp(0, 100).toInt();
    await _wsCommand({'type': 'set_volume', 'percent': clamped});
    _setState(
      state.copyWith(
        volumeStatus:
            (state.volumeStatus ?? const VolumeStatus(percent: 0, muted: false))
                .copyWith(percent: clamped),
      ),
    );
  }

  Future<void> setMuted(bool muted) async {
    await _wsCommand({'type': 'set_muted', 'muted': muted});
    _setState(
      state.copyWith(
        volumeStatus:
            (state.volumeStatus ?? const VolumeStatus(percent: 0, muted: false))
                .copyWith(muted: muted),
      ),
    );
  }
}
