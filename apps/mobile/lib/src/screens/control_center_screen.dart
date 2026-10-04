import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/display_info.dart';
import '../providers/system_controller.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';

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
    final display = system.displays.isNotEmpty ? system.displays.first : null;

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        toolbarHeight: 0,
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(18, 8, 18, 28),
          children: [
            PageHeading(
              title: '控制中心',
              trailing: IconBtn(
                icon: Icons.close_rounded,
                tooltip: '返回',
                onPressed: () => Navigator.of(context).maybePop(),
              ),
            ),
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
            _DisplaySection(
              display: display,
              onPickMode: () => _pickDisplayMode(display),
            ),
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

  Future<void> _pickAudioOutput() async {
    final system = ref.read(systemControllerProvider.notifier);
    if (ref.read(systemControllerProvider).audioOutputs.isEmpty) {
      await system.refreshAudioOutputs();
      if (!mounted) return;
    }
    final current = ref.read(systemControllerProvider).audioOutputs;
    if (!mounted || current.isEmpty) return;
    final selectedId = await showModalBottomSheet<String>(
      context: context,
      showDragHandle: true,
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

  Future<void> _pickDisplayMode(DisplayInfo? display) async {
    if (display == null || display.modes.isEmpty) {
      if (display != null) context.showInfoSnackBar('没有可选的分辨率');
      return;
    }
    final current = display.currentMode;
    final mode = await showModalBottomSheet<DisplayModeInfo>(
      context: context,
      showDragHandle: true,
      builder: (sheetContext) => SafeArea(
        child: ListView(
          shrinkWrap: true,
          children: [
            const ListTile(title: Text('分辨率与刷新率')),
            ...display.modes.map(
              (m) => ListTile(
                leading: Icon(
                  m.resolution == current.resolution &&
                          m.refreshMillihz == current.refreshMillihz
                      ? Icons.check_circle_rounded
                      : Icons.monitor_rounded,
                ),
                title: Text('${m.resolution} · ${m.refreshText}'),
                trailing:
                    m.resolution == current.resolution &&
                        m.refreshMillihz == current.refreshMillihz
                    ? const StatusTag('当前')
                    : null,
                onTap: () => Navigator.of(sheetContext).pop(m),
              ),
            ),
          ],
        ),
      ),
    );
    if (mode == null || !mounted) return;
    try {
      await ref.read(systemControllerProvider.notifier).setDisplayMode(display.id, mode);
      if (mounted) {
        setState(() {});
        context.showSuccessSnackBar('显示模式已切换');
      }
    } catch (error) {
      if (mounted) context.showErrorSnackBar('切换显示模式失败: $error');
    }
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
          width: 40,
          height: 40,
          child: IconButton(
            tooltip: '静音',
            onPressed: volumeAvailable ? onToggleMute : null,
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
          child: SliderTheme(
            data: SliderTheme.of(context).copyWith(
              trackHeight: 5,
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
          style: AppText.body.copyWith(color: palette.textMuted),
        ),
      ],
    );
  }
}

/// 显示器设置分组：当前显示器信息与分辨率入口。
class _DisplaySection extends StatelessWidget {
  const _DisplaySection({required this.display, required this.onPickMode});

  final DisplayInfo? display;
  final VoidCallback onPickMode;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final info = display;
    return SettingsGroup(
      title: '显示器',
      children: [
        Row(
          children: [
            TileIcon(
              icon: Icons.monitor_rounded,
              color: AppColors.cyan,
              size: 32,
              radius: 9,
              iconSize: 17,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    info?.name ?? '未检测到显示器',
                    style: AppText.body.copyWith(
                      fontWeight: FontWeight.w700,
                      color: palette.textPrimary,
                    ),
                  ),
                  Text(
                    info != null
                        ? '${info.currentMode.resolution} ${info.currentMode.refreshText}'
                        : '请检查连接',
                    style: AppText.meta.copyWith(
                      color: palette.textSubtle,
                    ),
                  ),
                ],
              ),
            ),
            if (info != null && info.primary)
              const StatusTag('主显示器'),
          ],
        ),
        const SizedBox(height: 6),
        AppListRow(
          icon: Icons.aspect_ratio_rounded,
          title: '分辨率与刷新率',
          showChevron: true,
          onTap: onPickMode,
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
