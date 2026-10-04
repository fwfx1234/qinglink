import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/settings_provider.dart';
import '../../services/touchpad_input_controller.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';

/// 鼠标设置面板：鼠标速度滑杆 + 自然滚动开关。
///
/// 面板内部保存拖动过程中的临时值，滑杆拖动即时同步到触控板，
/// 松手时才持久化到设置。
class MouseSettingsSheet extends ConsumerStatefulWidget {
  const MouseSettingsSheet({
    super.key,
    required this.sensitivity,
    required this.reverseScroll,
    required this.onSensitivityChanged,
  });

  /// 打开面板时的初始鼠标速度。
  final double sensitivity;

  /// 打开面板时的初始自然滚动开关值。
  final bool reverseScroll;

  /// 滑杆拖动过程中回调，用于实时同步触控板灵敏度。
  final ValueChanged<double> onSensitivityChanged;

  @override
  ConsumerState<MouseSettingsSheet> createState() => _MouseSettingsSheetState();
}

class _MouseSettingsSheetState extends ConsumerState<MouseSettingsSheet> {
  late double _sensitivity = widget.sensitivity;
  late bool _reverseScroll = widget.reverseScroll;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 4, 20, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Text(
                  '鼠标速度',
                  style: AppText.body.copyWith(color: palette.textPrimary),
                ),
                const Spacer(),
                StatusTag('${_sensitivity.toStringAsFixed(1)}x'),
              ],
            ),
            Slider(
              value: _sensitivity,
              min: TouchpadInputController.minSensitivity,
              max: TouchpadInputController.maxSensitivity,
              divisions: 14,
              onChanged: (value) {
                setState(() => _sensitivity = value);
                widget.onSensitivityChanged(value);
              },
              onChangeEnd: _saveSensitivity,
            ),
            const SizedBox(height: 4),
            AppListRow(
              title: '自然滚动',
              onTap: null,
              showChevron: false,
              trailing: AppToggle(
                value: _reverseScroll,
                onChanged: (value) {
                  setState(() => _reverseScroll = value);
                  ref
                      .read(settingsProvider.notifier)
                      .setReversePhoneScrollBar(value);
                },
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _saveSensitivity(double value) {
    unawaited(ref.read(settingsProvider.notifier).saveMouseSensitivity(value));
  }
}
