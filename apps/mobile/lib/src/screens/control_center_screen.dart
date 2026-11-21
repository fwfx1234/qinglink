import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/system_controller.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import 'control_center/display_mode_sheet.dart';
import 'control_center/display_section.dart';
import 'control_center/keep_awake_section.dart';
import '../widgets/app_sheet.dart';

class ControlCenterScreen extends ConsumerStatefulWidget {
  const ControlCenterScreen({super.key});

  @override
  ConsumerState<ControlCenterScreen> createState() =>
      _ControlCenterScreenState();
}

class _ControlCenterScreenState extends ConsumerState<ControlCenterScreen> {
  bool _switchingAudioOutput = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final system = ref.read(systemControllerProvider.notifier);
      system.refreshDisplays();
      system.refreshVolume();
      system.refreshAudioOutputs();
      system.refreshForeground();
      system.refreshKeepAwake();
    });
  }

  @override
  Widget build(BuildContext context) {
    final system = ref.watch(systemControllerProvider);
    final volumePercent = system.volume?.percent ?? 0;
    final muted = system.volume?.muted ?? false;
    final selectedOutput = system.audioOutputs
        .where((d) => d.isDefault)
        .firstOrNull;

    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(
            AppMetrics.pagePadding,
            AppSpacing.sm,
            AppMetrics.pagePadding,
            AppSpacing.xxl,
          ),
          children: [
            _TopBar(onRefresh: _refreshAll),
            _VolumeSection(
              volumePercent: volumePercent,
              muted: muted,
              volumeAvailable: system.volume != null,
              outputName: selectedOutput?.name ?? '默认设备',
              onToggleMute: () => ref
                  .read(systemControllerProvider.notifier)
                  .setMuted(!muted),
              onVolumeChanged: (value) => ref
                  .read(systemControllerProvider.notifier)
                  .setVolume(value.round()),
              onPickOutput: _pickAudioOutput,
            ),
            const SizedBox(height: 12),
            DisplaySection(
              displays: system.displays,
              activeDisplay: system.activeDisplay,
              loading: system.displaysLoading,
              error: system.displayError,
              onRetry: () =>
                  ref.read(systemControllerProvider.notifier).refreshDisplays(),
              onSelect: (id) =>
                  ref.read(systemControllerProvider.notifier).selectDisplay(id),
              onPickMode: _pickDisplayMode,
            ),
            const SizedBox(height: 12),
            const KeepAwakeSection(),
            const SizedBox(height: 12),
            _PowerSection(
              onSleep: () => confirmPowerAction(context, ref, 'sleep'),
              onRestart: () => confirmPowerAction(context, ref, 'restart'),
              onShutdown: () => confirmPowerAction(context, ref, 'shutdown'),
            ),
          ],
        ),
      ),
    );
  }

  /// 顶栏刷新：把控制中心用到的五路状态一起重拉。
  void _refreshAll() {
    final system = ref.read(systemControllerProvider.notifier);
    system.refreshDisplays();
    system.refreshVolume();
    system.refreshAudioOutputs();
    system.refreshForeground();
    system.refreshKeepAwake();
  }

  Future<void> _pickAudioOutput() async {
    final system = ref.read(systemControllerProvider.notifier);
    if (ref.read(systemControllerProvider).audioOutputs.isEmpty) {
      await system.refreshAudioOutputs();
      if (!mounted) return;
    }
    final current = ref.read(systemControllerProvider).audioOutputs;
    if (!mounted || current.isEmpty) return;
    final selectedId = await showAppSheet<String>(
      context,
      builder: (sheetContext) => SafeArea(
        child: ListView(
          shrinkWrap: true,
          children: [
            const ListTile(title: Text('音频输出')),
            ...current.map(
              (device) => ListTile(
                leading: Icon(
                  device.isDefault
                      ? Icons.check_circle_rounded
                      : Icons.speaker_rounded,
                ),
                title: Text(device.name),
                onTap: () => Navigator.of(sheetContext).pop(device.id),
              ),
            ),
          ],
        ),
      ),
    );
    if (selectedId == null || !mounted || _switchingAudioOutput) return;
    setState(() => _switchingAudioOutput = true);
    try {
      await ref.read(systemControllerProvider.notifier).setAudioOutput(selectedId);
      if (mounted) context.showSuccessSnackBar('音频输出已切换');
    } catch (error) {
      if (mounted) context.showErrorSnackBar('切换声音输出失败: $error');
    } finally {
      if (mounted) setState(() => _switchingAudioOutput = false);
    }
  }

  /// 选择分辨率与刷新率。
  ///
  /// 服务端返回的是 1080p 及以上的可选模式（同一分辨率最多 4 个刷新率），弹层按分辨率分组
  /// 列出刷新率；切换成功后重新拉一次列表，因为 Windows 可能拒绝并在 15 秒确认期内回滚。
  Future<void> _pickDisplayMode() async {
    final notifier = ref.read(systemControllerProvider.notifier);
    final display = ref.read(systemControllerProvider).activeDisplay;
    if (display == null) {
      context.showInfoSnackBar('还没有读到显示器，请先点右上角刷新');
      return;
    }
    if (display.modes.isEmpty) {
      context.showInfoSnackBar('这台显示器没有返回可用模式');
      return;
    }
    final mode = await DisplayModeSheet.show(context, display);
    if (mode == null || !mounted) return;
    try {
      await notifier.setDisplayMode(display.id, mode);
      if (!mounted) return;
      context.showSuccessSnackBar('已切换为 ${mode.label}');
    } catch (error) {
      if (!mounted) return;
      context.showErrorSnackBar('切换显示模式失败: $error');
    }
  }
}

/// 顶栏：返回 + 居中标题 + 刷新（设计稿 S3）。
class _TopBar extends StatelessWidget {
  const _TopBar({required this.onRefresh});

  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: AppMetrics.navHeight,
      child: Row(
        children: [
          IconBtn(
            icon: Icons.chevron_left_rounded,
            tooltip: '返回',
            tonal: true,
            iconSize: 22,
            onPressed: () => Navigator.of(context).maybePop(),
          ),
          Expanded(
            child: Text(
              '控制中心',
              textAlign: TextAlign.center,
              style: AppText.sectionTitle.copyWith(color: palette.textPrimary),
            ),
          ),
          IconBtn(
            icon: Icons.refresh_rounded,
            tooltip: '刷新',
            tonal: true,
            onPressed: onRefresh,
          ),
        ],
      ),
    );
  }
}

/// 声音设置分组：音量控制行与音频输出入口。
class _VolumeSection extends StatelessWidget {
  const _VolumeSection({
    required this.volumePercent,
    required this.muted,
    required this.volumeAvailable,
    required this.outputName,
    required this.onToggleMute,
    required this.onVolumeChanged,
    required this.onPickOutput,
  });

  /// 当前音量百分比。
  final int volumePercent;

  /// 是否处于静音状态。
  final bool muted;

  /// 音量数据是否可用；不可用时音量控件禁用。
  final bool volumeAvailable;

  /// 当前默认音频输出名称。
  final String outputName;

  /// 切换静音。
  final VoidCallback onToggleMute;

  /// 调整音量。
  final ValueChanged<double> onVolumeChanged;

  /// 打开音频输出选择。
  final VoidCallback onPickOutput;

  @override
  Widget build(BuildContext context) {
    return SettingsGroup(
      title: '声音',
      trailing: Text(
        muted ? '静音' : '$volumePercent%',
        style: AppText.label.copyWith(color: AppColors.primary),
      ),
      children: [
        _VolumeSliderRow(
          volumePercent: volumePercent,
          muted: muted,
          volumeAvailable: volumeAvailable,
          onToggleMute: onToggleMute,
          onVolumeChanged: onVolumeChanged,
        ),
        const SizedBox(height: 4),
        AppListRow(
          icon: Icons.audio_file_outlined,
          title: '音频输出',
          subtitle: outputName,
          showChevron: true,
          onTap: onPickOutput,
        ),
      ],
    );
  }
}

/// 静音按钮与音量滑杆。
class _VolumeSliderRow extends StatelessWidget {
  const _VolumeSliderRow({
    required this.volumePercent,
    required this.muted,
    required this.volumeAvailable,
    required this.onToggleMute,
    required this.onVolumeChanged,
  });

  final int volumePercent;
  final bool muted;
  final bool volumeAvailable;
  final VoidCallback onToggleMute;
  final ValueChanged<double> onVolumeChanged;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      children: [
        SizedBox(
          width: AppMetrics.minTap,
          height: AppMetrics.minTap,
          child: IconButton(
            tooltip: '静音',
            onPressed: volumeAvailable ? onToggleMute : null,
            icon: Icon(
              muted ? Icons.volume_off_rounded : Icons.volume_up_rounded,
              size: 20,
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
          child: SliderTheme(
            data: SliderTheme.of(context).copyWith(
              trackHeight: 6,
              activeTrackColor: AppColors.primary,
              inactiveTrackColor: palette.surface2,
              thumbColor: AppColors.primary,
              thumbShape: const RoundSliderThumbShape(
                enabledThumbRadius: 8,
              ),
              overlayShape: const RoundSliderOverlayShape(
                overlayRadius: 16,
              ),
            ),
            child: Slider(
              value: volumePercent.toDouble(),
              max: 100,
              onChanged: volumeAvailable ? onVolumeChanged : null,
            ),
          ),
        ),
        Text(
          '$volumePercent',
          style: AppText.meta.copyWith(color: palette.textMuted),
        ),
      ],
    );
  }
}

/// 电源设置分组：睡眠 / 重启 / 关机。
class _PowerSection extends StatelessWidget {
  const _PowerSection({
    required this.onSleep,
    required this.onRestart,
    required this.onShutdown,
  });

  final VoidCallback onSleep;
  final VoidCallback onRestart;
  final VoidCallback onShutdown;

  @override
  Widget build(BuildContext context) {
    return SettingsGroup(
      title: '电源',
      children: [
        Row(
          children: [
            Expanded(
              child: PowerActionButton(
                icon: Icons.bedtime_rounded,
                label: '睡眠',
                onPressed: onSleep,
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: PowerActionButton(
                icon: Icons.restart_alt_rounded,
                label: '重启',
                onPressed: onRestart,
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: PowerActionButton(
                icon: Icons.power_settings_new_rounded,
                label: '关机',
                danger: true,
                onPressed: onShutdown,
              ),
            ),
          ],
        ),
      ],
    );
  }
}
