/// 电脑电源/连接状态机，对齐原型 `app.js` 里的 statusMeta 状态。
enum WakeState {
  /// 初始/离线，尚未尝试连接。
  idle,

  /// 正在检查电脑状态。
  checking,

  /// 正在发送局域网唤醒信号。
  waking,

  /// 电脑已上线，正在建立连接。
  connecting,

  /// 已连接并可用。
  connected,

  /// 已连接但 Windows 处于锁屏。
  locked,

  /// 唤醒或连接失败。
  failed,
}
