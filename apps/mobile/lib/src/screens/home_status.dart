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

/// 状态卡要展示的内容：指示灯、标题、主按钮与进度提示。
class HomeStatusView {
  const HomeStatusView({
    required this.label,
    required this.dot,
    required this.title,
    required this.action,
    required this.primaryLabel,
    required this.primaryIcon,
    this.progressLabel,
    this.busy = false,
  });

  final String label;
  final Color dot;
  final String title;
  final HomeStatusAction action;
  final String primaryLabel;
  final IconData primaryIcon;
  final String? progressLabel;
  final bool busy;
}

/// 由唤醒状态与当前连接状态推导首页状态卡内容。
HomeStatusView resolveHomeStatus({
  required WakeState wakeState,
  required bool online,
  required bool locked,
  required bool uac,
  required bool canWake,
}) {
  if (wakeState == WakeState.waking) {
    return const HomeStatusView(
      label: '正在唤醒',
      dot: AppColors.amber,
      title: '正在唤醒客厅电脑',
      action: HomeStatusAction.none,
      primaryLabel: '正在唤醒…',
      primaryIcon: Icons.rotate_right_rounded,
      progressLabel: '正在发送唤醒信号…',
      busy: true,
    );
  }
  if (wakeState == WakeState.checking || wakeState == WakeState.connecting) {
    return const HomeStatusView(
      label: '连接中',
      dot: AppColors.amber,
      title: '正在建立连接',
      action: HomeStatusAction.none,
      primaryLabel: '连接中…',
      primaryIcon: Icons.rotate_right_rounded,
      progressLabel: '正在等待电脑响应…',
      busy: true,
    );
  }
  if (wakeState == WakeState.failed) {
    return const HomeStatusView(
      label: '连接失败',
      dot: AppColors.red,
      title: '没有连上电脑',
      action: HomeStatusAction.wake,
      primaryLabel: '重新连接',
      primaryIcon: Icons.refresh_rounded,
    );
  }
  if (uac) {
    return const HomeStatusView(
      label: 'UAC 确认中',
      dot: AppColors.amber,
      title: '当前账户已登录，正在等待 UAC 确认',
      action: HomeStatusAction.none,
      primaryLabel: 'UAC 确认中',
      primaryIcon: Icons.verified_user_rounded,
    );
  }
  if (locked) {
    return const HomeStatusView(
      label: '需要解锁',
      dot: AppColors.amber,
      title: '解锁后即可使用',
      action: HomeStatusAction.unlock,
      primaryLabel: '解锁 Windows',
      primaryIcon: Icons.lock_open_rounded,
    );
  }
  if (!canWake) {
    return const HomeStatusView(
      label: '未连接',
      dot: AppColors.primary,
      title: '连接电脑，开始使用',
      action: HomeStatusAction.wake,
      primaryLabel: '连接电脑',
      primaryIcon: Icons.link_rounded,
    );
  }
  return const HomeStatusView(
    label: '已关机',
    dot: AppColors.primary,
    title: '唤醒电脑，开始游戏',
    action: HomeStatusAction.wake,
    primaryLabel: '唤醒电脑',
    primaryIcon: Icons.power_settings_new_rounded,
  );
}
