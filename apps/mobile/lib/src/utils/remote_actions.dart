import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import '../providers/remote_control_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import 'app_dialogs.dart';
import 'snackbar_extension.dart';

/// 对当前前台窗口执行操作（最小化/最大化/关闭），统一确认与错误反馈。
Future<void> runWindowAction(
  BuildContext context,
  WidgetRef ref,
  String action,
) async {
  if (!ref.read(remoteControlProvider).isConnected) {
    context.showWarningSnackBar('电脑未连接，无法操作窗口');
    return;
  }
  final remote = ref.read(remoteControlProvider.notifier);
  var hwnd = ref.read(remoteControlProvider).foregroundApp?.hwnd ?? 0;
  if (hwnd == 0) {
    // Foreground metadata can be missing before the PC pushes an update.
    // Refresh it, then fall back to the window catalog's foreground entry.
    try {
      await remote.refreshForeground();
      hwnd = ref.read(remoteControlProvider).foregroundApp?.hwnd ?? 0;
      if (hwnd == 0) {
        final windows = await remote.listWindows();
        final foreground = windows.where((window) => window.isForeground);
        if (foreground.isNotEmpty) hwnd = foreground.first.hwnd;
      }
    } catch (_) {}
  }
  if (hwnd == 0) {
    if (!context.mounted) return;
    context.showWarningSnackBar('没有可操作的前台窗口');
    return;
  }
  try {
    if (action == 'minimize') {
      await remote.minimizeWindow(hwnd);
      if (context.mounted) context.showSuccessSnackBar('当前窗口已最小化');
    } else if (action == 'maximize') {
      await remote.maximizeWindow(hwnd);
      if (context.mounted) context.showSuccessSnackBar('当前窗口已最大化');
    } else if (action == 'close') {
      if (!context.mounted) return;
      final ok = await showConfirmDialog(
        context,
        title: '关闭当前窗口',
        message: '确定关闭当前窗口吗？若窗口无响应，将强制结束该进程。',
        confirmLabel: '关闭',
        danger: true,
      );
      if (!ok || !context.mounted) return;
      await remote.closeWindow(hwnd);
      if (context.mounted) context.showSuccessSnackBar('已关闭当前窗口');
    }
    await remote.refreshForeground();
  } catch (error) {
    if (context.mounted) context.showErrorSnackBar('窗口操作失败: $error');
  }
}

/// 打开窗口列表底部弹层，可选择切到前台或关闭窗口。
Future<void> showWindowListSheet(BuildContext context, WidgetRef ref) async {
  if (!ref.read(remoteControlProvider).isConnected) {
    context.showWarningSnackBar('电脑未连接，无法读取窗口');
    return;
  }
  try {
    final windows = await ref
        .read(remoteControlProvider.notifier)
        .listWindows(forceRefresh: true);
    if (!context.mounted) return;
    if (windows.isEmpty) {
      context.showInfoSnackBar('没有可切换的窗口');
      return;
    }
    await showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (sheet) => SafeArea(
        child: ListView(
          shrinkWrap: true,
          padding: const EdgeInsets.fromLTRB(8, 0, 8, 16),
          children: windows
              .map(
                (window) => ListTile(
                  leading: Icon(
                    window.isForeground
                        ? Icons.check_circle_rounded
                        : Icons.desktop_windows_rounded,
                  ),
                  title: Text(
                    _windowLabel(window),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  subtitle: Text('PID ${window.pid}'),
                  onTap: () async {
                    try {
                      await ref
                          .read(remoteControlProvider.notifier)
                          .focusWindow(window.hwnd);
                      if (sheet.mounted) Navigator.pop(sheet);
                    } catch (error) {
                      if (sheet.mounted) {
                        sheet.showErrorSnackBar('切换窗口失败: $error');
                      }
                    }
                  },
                  trailing: IconButton(
                    tooltip: '关闭',
                    icon: const Icon(Icons.close_rounded),
                    onPressed: () async {
                      await ref
                          .read(remoteControlProvider.notifier)
                          .closeWindow(window.hwnd);
                      await ref
                          .read(remoteControlProvider.notifier)
                          .refreshForeground();
                      if (sheet.mounted) Navigator.pop(sheet);
                    },
                  ),
                ),
              )
              .toList(),
        ),
      ),
    );
  } catch (error) {
    if (context.mounted) context.showErrorSnackBar('读取窗口失败: $error');
  }
}

/// 窗口列表显示名：优先窗口标题，其次进程名，最后「未知窗口」。
String _windowLabel(WindowInfo window) {
  final title = window.title.trim();
  if (title.isNotEmpty) return title;
  final process = window.processName.trim();
  if (process.isNotEmpty) return process;
  return '未知窗口';
}

/// 打开音量调节弹层，支持静音、音量滑动与输出设备选择。
Future<void> showVolumeSheet(BuildContext context, WidgetRef ref) async {
  final remote = ref.read(remoteControlProvider.notifier);
  await Future.wait([remote.getVolume(), remote.getAudioOutputs()]);
  if (!context.mounted) return;
  await showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    isScrollControlled: true,
    builder: (_) => Consumer(
      builder: (context, ref, _) {
        final palette = context.palette;
        final volume = ref.watch(
          remoteControlProvider.select((s) => s.volumeStatus),
        );
        final outputs = ref.watch(
          remoteControlProvider.select((s) => s.audioOutputs),
        );
        final muted = volume?.muted ?? false;
        final percent = volume?.percent ?? 0;
        return SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  children: [
                    Text(
                      '电脑音量',
                      style: AppText.cardTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                    const Spacer(),
                    Text(
                      muted ? '静音' : '$percent%',
                      style: AppText.label.copyWith(color: AppColors.primary),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    SizedBox(
                      width: 40,
                      height: 40,
                      child: IconButton(
                        tooltip: '静音',
                        onPressed: volume == null
                            ? null
                            : () => remote.setMuted(!muted).catchError((
                                Object e,
                              ) {
                                if (context.mounted) {
                                  context.showErrorSnackBar('静音失败: $e');
                                }
                              }),
                        icon: Icon(
                          muted
                              ? Icons.volume_off_rounded
                              : Icons.volume_up_rounded,
                          size: 17,
                        ),
                        style: IconButton.styleFrom(
                          backgroundColor: AppColors.primary,
                          foregroundColor: AppColors.onPrimary,
                          shape: const CircleBorder(),
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Slider(
                        value: percent.toDouble(),
                        max: 100,
                        onChanged: volume == null
                            ? null
                            : (v) => remote.setVolume(v.round()).catchError((
                                Object e,
                              ) {
                                if (context.mounted) {
                                  context.showErrorSnackBar('设置音量失败: $e');
                                }
                              }),
                      ),
                    ),
                    Text(
                      '$percent',
                      style: AppText.body.copyWith(color: palette.textMuted),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Divider(color: palette.line, height: 1),
                const SizedBox(height: 12),
                Text(
                  '输出设备',
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
                const SizedBox(height: 4),
                if (outputs.isEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 10),
                    child: Text(
                      '没有可用的输出设备',
                      style: AppText.body.copyWith(color: palette.textMuted),
                    ),
                  )
                else
                  ...outputs.map(
                    (device) => AppListRow(
                      icon: device.isDefault
                          ? Icons.check_circle_rounded
                          : Icons.speaker_rounded,
                      iconTone: device.isDefault
                          ? AppColors.primary
                          : palette.textMuted,
                      title: device.name,
                      minHeight: 48,
                      trailing: device.isDefault ? const StatusTag('默认') : null,
                      onTap: () async {
                        try {
                          await remote.setAudioOutput(device.id);
                          if (context.mounted) {
                            context.showSuccessSnackBar('输出设备已切换');
                          }
                        } catch (error) {
                          if (context.mounted) {
                            context.showErrorSnackBar('切换输出失败: $error');
                          }
                        }
                      },
                    ),
                  ),
              ],
            ),
          ),
        );
      },
    ),
  );
}
