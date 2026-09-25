part of 'remote_control_provider.dart';

/// 电源控制相关动作。
mixin _PowerActions on _RemoteControlBase {
  // ========== Power Commands ==========

  Future<void> shutdown({bool force = false, int delaySecs = 0}) async {
    await _wsCommand({
      'type': 'shutdown',
      'force': force,
      'delay_secs': delaySecs,
    });
    // 立即关机：命令发出后 3s 内确认离线；延迟关机不抢先判定。
    if (delaySecs <= 0) expectPcOfflineSoon();
  }

  Future<void> sleep({bool hibernate = false}) async {
    await _wsCommand({'type': 'sleep', 'hibernate': hibernate});
    // 睡眠/休眠后 socket 会快速断开，3s 内确认离线并进入快速感知。
    expectPcOfflineSoon();
  }

  Future<void> restart({bool force = false, int delaySecs = 0}) async {
    await _wsCommand({
      'type': 'restart',
      'force': force,
      'delay_secs': delaySecs,
    });
    // 重启后 socket 断开，3s 内确认离线并进入快速感知。
    expectPcOfflineSoon();
  }

  Future<void> lock() async {
    await _wsCommand({'type': 'lock'});
  }
}
