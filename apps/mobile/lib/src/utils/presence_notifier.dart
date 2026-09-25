// PC 在线/离线提醒与自动唤醒决策（纯函数/轻量簿记，便于单测）。

/// 在线/离线提醒簿记：记录“本轮离线是否已提示过”，恢复上线时补一条提示。
///
/// - [onOffline] 只在真实离线事件（宽限期到期，而非登出/显式断开）时调用。
/// - 仅前台、已认证且不在冷却期内的离线才提示，并标记本轮离线已提示。
/// - [onOnline] 在 pcOnline 恢复时调用，仅当本轮离线已提示过才补“PC已上线”。
/// - WS 重连成功、前后台切换、首次连接成功都不产生任何提示。
class PresenceNotificationTracker {
  PresenceNotificationTracker({
    this.offlineCooldown = const Duration(seconds: 30),
  });

  final Duration offlineCooldown;

  bool offlineEpisodeActive = false;
  DateTime? lastOfflineNotifiedAt;

  /// 处理真实离线事件，返回是否应提示“PC已离线”。
  bool onOffline({
    required bool foregrounded,
    required bool authenticated,
    DateTime? now,
  }) {
    final time = now ?? DateTime.now();
    if (!foregrounded || !authenticated) return false;
    if (lastOfflineNotifiedAt != null &&
        time.difference(lastOfflineNotifiedAt!) < offlineCooldown) {
      return false;
    }
    lastOfflineNotifiedAt = time;
    offlineEpisodeActive = true;
    return true;
  }

  /// 处理恢复上线，返回是否应提示“PC已上线”。
  bool onOnline({required bool foregrounded}) {
    final notify = foregrounded && offlineEpisodeActive;
    offlineEpisodeActive = false;
    return notify;
  }

  /// 后台→前台时清除未结算的上线回合：后台期间的离线/恢复连线不补弹窗。
  void resetEpisode() {
    offlineEpisodeActive = false;
  }
}

/// 自动唤醒流程动作。
enum AutoWakeAction { normalConnect, hintMissingMac, wakeAndWait }

/// 决定自动唤醒流程的动作。
///
/// - 未配对：一律走普通连接（触发配对弹窗）。
/// - 未开启自动唤醒：普通连接。
/// - 开启但未完成配对：提示需要先完成配对并普通连接。
/// - 冷启动：直接发 WOL（保证“打开即发唤醒包”）。
/// - 回前台：已知在线则普通重连；已知离线则 WOL。
AutoWakeAction decideAutoWakeAction({
  required bool enabled,
  required bool paired,
  required bool macConfigured,
  required bool coldStart,
  required bool pcOnline,
}) {
  if (!paired) return AutoWakeAction.normalConnect;
  if (!enabled) return AutoWakeAction.normalConnect;
  if (!macConfigured) return AutoWakeAction.hintMissingMac;
  if (coldStart) return AutoWakeAction.wakeAndWait;
  return pcOnline ? AutoWakeAction.normalConnect : AutoWakeAction.wakeAndWait;
}
