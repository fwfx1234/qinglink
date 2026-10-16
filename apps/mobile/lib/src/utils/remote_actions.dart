import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import '../models/system_status.dart';
import '../providers/feature_facades.dart';
import '../providers/system_controller.dart';
import '../providers/window_controller.dart';
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
  if (!ref.read(connectionReadyProvider)) {
    context.showWarningSnackBar('电脑未连接，无法操作窗口');
    return;
  }
  final system = ref.read(systemControllerProvider.notifier);
  var hwnd = ref.read(systemControllerProvider).foreground?.hwnd ?? 0;
  if (hwnd == 0) {
    // Foreground metadata can be missing before the PC pushes an update.
    // Refresh it, then fall back to the window catalog's foreground entry.
    try {
      await system.refreshForeground();
      hwnd = ref.read(systemControllerProvider).foreground?.hwnd ?? 0;
      if (hwnd == 0) {
        final windows = await ref.read(windowControllerProvider).list();
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
      await ref.read(windowControllerProvider).minimize(hwnd);
      if (context.mounted) context.showSuccessSnackBar('当前窗口已最小化');
    } else if (action == 'maximize') {
      await ref.read(windowControllerProvider).maximize(hwnd);
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
      await ref.read(windowControllerProvider).close(hwnd);
      if (context.mounted) context.showSuccessSnackBar('已关闭当前窗口');
    }
    await system.refreshForeground();
  } catch (error) {
    if (context.mounted) context.showErrorSnackBar('窗口操作失败: $error');
  }
}

/// 打开窗口列表底部弹层，可选择切到前台或关闭窗口。
Future<void> showWindowListSheet(BuildContext context, WidgetRef ref) async {
  if (!ref.read(connectionReadyProvider)) {
    context.showWarningSnackBar('电脑未连接，无法读取窗口');
    return;
  }
  try {
    final windows = await ref.read(windowControllerProvider).list(forceRefresh: true);
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
                          .read(windowControllerProvider)
                          .focus(window.hwnd);
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
                      // 与上面的「切换窗口」一样必须自己兜住异常：PC 侧窗口可能
                      // 已经被关掉/拒绝，异常逃逸会打断弹层的关闭流程。
                      try {
                        await ref.read(windowControllerProvider).close(window.hwnd);
                        await ref.read(systemControllerProvider.notifier).refreshForeground();
                        if (sheet.mounted) Navigator.pop(sheet);
                      } catch (error) {
                        if (sheet.mounted) {
                          sheet.showErrorSnackBar('关闭窗口失败: $error');
                        }
                      }
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
  final system = ref.read(systemControllerProvider.notifier);
  await Future.wait([system.refreshVolume(), system.refreshAudioOutputs()]);
  if (!context.mounted) return;
  await showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    isScrollControlled: true,
    builder: (_) => const _VolumeSheetBody(),
  );
}

/// 音量弹层内容：读取音量与输出设备状态，组合标题、调节区与设备列表。
class _VolumeSheetBody extends ConsumerWidget {
  const _VolumeSheetBody();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final system = ref.watch(systemControllerProvider);
    final volume = system.volume;
    final muted = volume?.muted ?? false;
    final percent = volume?.percent ?? 0;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 0, 20, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _VolumeHeader(muted: muted, percent: percent),
            const SizedBox(height: 8),
            _VolumeSliderRow(volume: volume, muted: muted, percent: percent),
            const SizedBox(height: 8),
            Divider(color: palette.line, height: 1),
            const SizedBox(height: 12),
            Text(
              '输出设备',
              style: AppText.meta.copyWith(color: palette.textSubtle),
            ),
            const SizedBox(height: 4),
            _AudioOutputList(outputs: system.audioOutputs),
          ],
        ),
      ),
    );
  }
}

/// 音量标题行：左侧标题，右侧静音状态或音量百分比。
class _VolumeHeader extends StatelessWidget {
  const _VolumeHeader({required this.muted, required this.percent});

  final bool muted;
  final int percent;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      children: [
        Text(
          '电脑音量',
          style: AppText.cardTitle.copyWith(color: palette.textPrimary),
        ),
        const Spacer(),
        Text(
          muted ? '静音' : '$percent%',
          style: AppText.label.copyWith(color: AppColors.primary),
        ),
      ],
    );
  }
}

/// 静音开关与音量滑动条：两个写入操作各自处理失败提示。
class _VolumeSliderRow extends ConsumerWidget {
  const _VolumeSliderRow({
    required this.volume,
    required this.muted,
    required this.percent,
  });

  final VolumeStatus? volume;
  final bool muted;
  final int percent;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    return Row(
      children: [
        SizedBox(
          width: 40,
          height: 40,
          child: IconButton(
            tooltip: '静音',
            onPressed: volume == null
                ? null
                : () => ref.read(systemControllerProvider.notifier).setMuted(!muted).catchError((
                    Object e,
                  ) {
                    if (context.mounted) {
                      context.showErrorSnackBar('静音失败: $e');
                    }
                  }),
            icon: Icon(
              muted ? Icons.volume_off_rounded : Icons.volume_up_rounded,
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
                : (v) => ref.read(systemControllerProvider.notifier).setVolume(v.round()).catchError((
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
    );
  }
}

/// 输出设备列表：无可用设备时给出占位提示，否则逐项展示切换入口。
class _AudioOutputList extends ConsumerWidget {
  const _AudioOutputList({required this.outputs});

  final List<AudioOutputDevice> outputs;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    if (outputs.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 10),
        child: Text(
          '没有可用的输出设备',
          style: AppText.body.copyWith(color: palette.textMuted),
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final device in outputs) _AudioOutputRow(device: device),
      ],
    );
  }
}

/// 单个输出设备行：点击后把该设备切换为默认输出设备。
class _AudioOutputRow extends ConsumerWidget {
  const _AudioOutputRow({required this.device});

  final AudioOutputDevice device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    return AppListRow(
      icon: device.isDefault
          ? Icons.check_circle_rounded
          : Icons.speaker_rounded,
      iconTone: device.isDefault ? AppColors.primary : palette.textMuted,
      title: device.name,
      minHeight: 48,
      trailing: device.isDefault ? const StatusTag('默认') : null,
      onTap: () async {
        try {
          await ref
              .read(systemControllerProvider.notifier)
              .setAudioOutput(device.id);
          if (context.mounted) {
            context.showSuccessSnackBar('输出设备已切换');
          }
        } catch (error) {
          if (context.mounted) {
            context.showErrorSnackBar('切换输出失败: $error');
          }
        }
      },
    );
  }
}
