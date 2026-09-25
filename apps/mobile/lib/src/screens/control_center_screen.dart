import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/display_info.dart';
import '../providers/remote_control_provider.dart';
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
      final remote = ref.read(remoteControlProvider.notifier);
      remote.getDisplays();
      remote.getVolume();
      remote.getAudioOutputs();
      remote.refreshForeground();
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(
      remoteControlProvider.select(
        (s) => (
          displays: s.displays,
          volumeStatus: s.volumeStatus,
          audioOutputs: s.audioOutputs,
        ),
      ),
    );
    final remote = ref.read(remoteControlProvider.notifier);
    final palette = context.palette;
    final volumePercent = state.volumeStatus?.percent ?? 0;
    final muted = state.volumeStatus?.muted ?? false;
    final selectedOutput = state.audioOutputs
        .where((d) => d.isDefault)
        .firstOrNull;
    final display = state.displays.isNotEmpty ? state.displays.first : null;

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
            SettingsGroup(
              title: '声音',
              trailing: Text(
                muted ? '静音' : '$volumePercent%',
                style: AppText.label.copyWith(color: AppColors.primary),
              ),
              children: [
                Row(
                  children: [
                    SizedBox(
                      width: 40,
                      height: 40,
                      child: IconButton(
                        tooltip: '静音',
                        onPressed: state.volumeStatus == null
                            ? null
                            : () => remote.setMuted(!muted),
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
                          onChanged: state.volumeStatus == null
                              ? null
                              : (value) => remote.setVolume(value.round()),
                        ),
                      ),
                    ),
                    Text(
                      '$volumePercent',
                      style: AppText.body.copyWith(color: palette.textMuted),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                AppListRow(
                  icon: Icons.audio_file_outlined,
                  title: '音频输出',
                  subtitle: selectedOutput?.name ?? '默认设备',
                  showChevron: true,
                  onTap: _pickAudioOutput,
                ),
              ],
            ),
            const SizedBox(height: 12),
            SettingsGroup(
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
                            display?.name ?? '未检测到显示器',
                            style: AppText.body.copyWith(
                              fontWeight: FontWeight.w700,
                              color: palette.textPrimary,
                            ),
                          ),
                          Text(
                            display != null
                                ? '${display.currentMode.resolution} ${display.currentMode.refreshText}'
                                : '请检查连接',
                            style: AppText.meta.copyWith(
                              color: palette.textSubtle,
                            ),
                          ),
                        ],
                      ),
                    ),
                    if (display != null && display.primary)
                      const StatusTag('主显示器'),
                  ],
                ),
                const SizedBox(height: 6),
                AppListRow(
                  icon: Icons.aspect_ratio_rounded,
                  title: '分辨率与刷新率',
                  showChevron: true,
                  onTap: () => _pickDisplayMode(display),
                ),
              ],
            ),
            const SizedBox(height: 12),
            SettingsGroup(
              title: '电源',
              children: [
                Row(
                  children: [
                    Expanded(
                      child: PowerActionButton(
                        icon: Icons.bedtime_rounded,
                        label: '睡眠',
                        onPressed: () =>
                            confirmPowerAction(context, ref, 'sleep'),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: PowerActionButton(
                        icon: Icons.restart_alt_rounded,
                        label: '重启',
                        onPressed: () =>
                            confirmPowerAction(context, ref, 'restart'),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: PowerActionButton(
                        icon: Icons.power_settings_new_rounded,
                        label: '关机',
                        danger: true,
                        onPressed: () =>
                            confirmPowerAction(context, ref, 'shutdown'),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _pickAudioOutput() async {
    final remote = ref.read(remoteControlProvider.notifier);
    if (ref.read(remoteControlProvider).audioOutputs.isEmpty) {
      await remote.getAudioOutputs();
      if (!mounted) return;
    }
    final current = ref.read(remoteControlProvider).audioOutputs;
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
      await remote.setAudioOutput(selectedId);
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
      await ref
          .read(remoteControlProvider.notifier)
          .setDisplayMode(display.id, mode);
      if (mounted) {
        setState(() {});
        context.showSuccessSnackBar('显示模式已切换');
      }
    } catch (error) {
      if (mounted) context.showErrorSnackBar('切换显示模式失败: $error');
    }
  }
}
