import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/feature_facades.dart';
import '../providers/process_controller.dart';
import '../providers/settings_provider.dart';
import '../providers/power_controller.dart';
import '../theme/app_theme.dart';
import 'snackbar_extension.dart';
import 'tv_power_follow.dart';

/// 通用确认弹窗，返回用户是否确认。
Future<bool> showConfirmDialog(
  BuildContext context, {
  required String title,
  required String message,
  String confirmLabel = '确定',
  String cancelLabel = '取消',
  bool danger = false,
}) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: Text(title),
      content: Text(message),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx, false),
          child: Text(cancelLabel),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(ctx, true),
          style: danger
              ? FilledButton.styleFrom(backgroundColor: AppColors.red)
              : null,
          child: Text(confirmLabel),
        ),
      ],
    ),
  );
  return result ?? false;
}

/// 电源操作（睡眠/重启/关机）确认并执行。
Future<void> confirmPowerAction(
  BuildContext context,
  WidgetRef ref,
  String mode,
) async {
  // 标题里带上真实设备名（设置里可能已经改过名），不要写死成「客厅电脑」。
  final pcName = ref.read(settingsProvider).pcName;
  final device = (pcName == null || pcName.isEmpty) ? '电脑' : pcName;
  final copy = <String, (String, String, String)>{
    'sleep': (
      '让$device进入睡眠？',
      '电脑会立即进入睡眠，当前应用保持打开。',
      '睡眠',
    ),
    'restart': (
      '重启$device？',
      '正在运行的应用会被关闭，电脑上线后仍可继续连接。',
      '重启',
    ),
    'shutdown': (
      '关闭$device？',
      '这会中断所有正在运行的游戏和软件，请确认已保存进度。',
      '关机',
    ),
  };
  final entry = copy[mode];
  if (entry == null) return;
  final (title, message, label) = entry;

  if (!ref.read(connectionReadyProvider)) {
    context.showWarningSnackBar('电脑未连接，无法执行电源操作');
    return;
  }

  final confirmed = await showConfirmDialog(
    context,
    title: title,
    message: message,
    confirmLabel: label,
    danger: mode == 'shutdown',
  );
  if (!confirmed || !context.mounted) return;

  try {
    final power = ref.read(powerControllerProvider);
    if (mode == 'sleep') {
      await power.sleep();
      await followPcWithTvPower(ref, TvFollowAction.powerOff);
    } else if (mode == 'restart') {
      await power.restart();
    } else {
      await power.shutdown();
      await followPcWithTvPower(ref, TvFollowAction.powerOff);
    }
  } catch (error) {
    if (context.mounted) context.showErrorSnackBar('$label失败: $error');
  }
}

/// 结束进程确认并执行。
Future<void> confirmKillProcess(
  BuildContext context,
  WidgetRef ref, {
  required int pid,
  required String name,
}) async {
  final confirmed = await showConfirmDialog(
    context,
    title: '结束 $name？',
    message: 'PID $pid 将被强制结束，未保存的数据可能丢失。',
    confirmLabel: '结束进程',
    danger: true,
  );
  if (!confirmed || !context.mounted) return;

  try {
    await ref.read(processControllerProvider.notifier).kill(pid);
    if (context.mounted) context.showSuccessSnackBar('已结束: $name');
  } catch (error) {
    if (context.mounted) context.showErrorSnackBar('结束失败: $error');
  }
}
