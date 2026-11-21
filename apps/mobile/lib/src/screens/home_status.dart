/// 首页状态卡的状态模型与状态推导。
///
/// 卡片渲染在 `home_status_card.dart`；本文件只放状态枚举、视图数据，以及
/// 由唤醒状态 / 在线 / 锁屏 / UAC / MAC 推导状态的纯逻辑。
library;

import 'package:flutter/material.dart';

import '../models/wake_state.dart';
import '../theme/app_theme.dart';

/// 状态卡主按钮点击后要执行的动作。
enum HomeStatusAction { wake, unlock, none }

/// 状态卡的形态。
///
/// [ready] 是一行紧凑展示（图标 + 设备名 + 在线药丸），[busy] / [interrupt]
/// 是整卡表达（含主按钮）。设计规则 2：一屏只出现一张状态卡。
enum HomeStatusKind { ready, busy, interrupt }

/// 状态卡要展示的内容：指示灯、标题、主按钮与进度提示。
class HomeStatusView {
  const HomeStatusView({
    required this.label,
    required this.dot,
    required this.title,
    required this.action,
    required this.primaryLabel,
    required this.primaryIcon,
    this.kind = HomeStatusKind.interrupt,
    this.subtitle,
    this.progressLabel,
    this.busy = false,
  });

  final String label;
  final Color dot;
  final String title;
  final HomeStatusAction action;
  final String primaryLabel;
  final IconData primaryIcon;
  final HomeStatusKind kind;

  /// 设备名下方的一行说明（系统、锁屏、运行时长）。
  final String? subtitle;
  final String? progressLabel;
  final bool busy;
}

/// 由唤醒状态与当前连接状态推导首页状态卡内容。
///
/// [deviceName] 与 [subtitle] 由页面提供：处于在线正常态时卡片只展示设备本身，
/// 中断态（离线 / 锁定 / UAC）则改用整卡 + 主按钮表达。
HomeStatusView resolveHomeStatus({
  required WakeState wakeState,
  required bool online,
  required bool locked,
  required bool uac,
  required bool canWake,
  String? deviceName,
  String? subtitle,
}) {
  final name = (deviceName?.trim().isNotEmpty ?? false)
      ? deviceName!.trim()
      : '客厅电脑';
  if (wakeState == WakeState.waking) {
    return HomeStatusView(
      label: '正在唤醒',
      dot: AppColors.amber,
      title: '正在唤醒 $name',
      action: HomeStatusAction.none,
      primaryLabel: '正在唤醒…',
      primaryIcon: Icons.rotate_right_rounded,
      kind: HomeStatusKind.busy,
      subtitle: subtitle,
      progressLabel: '正在发送唤醒信号…',
      busy: true,
    );
  }
  if (wakeState == WakeState.checking || wakeState == WakeState.connecting) {
    return HomeStatusView(
      label: '连接中',
      dot: AppColors.amber,
      title: '正在建立连接',
      action: HomeStatusAction.none,
      primaryLabel: '连接中…',
      primaryIcon: Icons.rotate_right_rounded,
      kind: HomeStatusKind.busy,
      subtitle: subtitle,
      progressLabel: '正在等待电脑响应…',
      busy: true,
    );
  }
  if (wakeState == WakeState.failed) {
    return HomeStatusView(
      label: '连接失败',
      dot: AppColors.red,
      title: '没有连上电脑',
      action: HomeStatusAction.wake,
      primaryLabel: '重新连接',
      primaryIcon: Icons.refresh_rounded,
      subtitle: '检查电脑是否开机，或在「更多」里配置唤醒',
    );
  }
  if (uac) {
    return HomeStatusView(
      label: 'UAC 确认中',
      dot: AppColors.amber,
      title: '当前账户已登录，等待 UAC 确认',
      action: HomeStatusAction.none,
      primaryLabel: 'UAC 确认中',
      primaryIcon: Icons.verified_user_rounded,
      subtitle: '请在电脑上完成管理员授权',
    );
  }
  if (locked) {
    return HomeStatusView(
      label: '需要解锁',
      dot: AppColors.amber,
      title: name,
      action: HomeStatusAction.unlock,
      primaryLabel: '解锁 Windows',
      primaryIcon: Icons.lock_open_rounded,
      subtitle: subtitle ?? '解锁后即可使用',
    );
  }
  if (online) {
    return HomeStatusView(
      label: '在线',
      dot: AppColors.primary,
      title: name,
      action: HomeStatusAction.none,
      primaryLabel: '',
      primaryIcon: Icons.check_rounded,
      kind: HomeStatusKind.ready,
      subtitle: subtitle,
    );
  }
  if (!canWake) {
    return HomeStatusView(
      label: '未连接',
      dot: AppColors.primary,
      title: name,
      action: HomeStatusAction.wake,
      primaryLabel: '连接电脑',
      primaryIcon: Icons.link_rounded,
      subtitle: '还没有配对记录，先连接一台电脑',
    );
  }
  return HomeStatusView(
    label: '已关机',
    dot: AppColors.primary,
    title: name,
    action: HomeStatusAction.wake,
    primaryLabel: '唤醒电脑',
    primaryIcon: Icons.power_settings_new_rounded,
    subtitle: '电脑当前离线，可以发送唤醒信号',
  );
}
