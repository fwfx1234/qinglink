/// 设置页「输入」分组：自动弹出键盘、自然滚动与鼠标速度。
///
/// 三项都只改本机偏好，统一落在 `settingsProvider`：自动弹出键盘控制是否响应
/// 服务端的 `focus_input` 事件，自然滚动控制触控板/滚动条的滑动方向，鼠标速度
/// 决定指尖位移到指针位移的倍率。控制页的触控板监听同一个 provider，所以在设置
/// 页改完回到控制页即刻生效，不再需要单独的面板。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import 'settings_widgets.dart';

/// 设置页的「输入」分组。
class SettingsInputSection extends ConsumerWidget {
  const SettingsInputSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(settingsProvider);
    final notifier = ref.read(settingsProvider.notifier);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SettingsSectionLabel('输入'),
        const SizedBox(height: AppSpacing.sm),
        AppCard(
          radius: AppRadius.lg,
          padding: EdgeInsets.zero,
          child: Column(
            children: [
              SettingsRow(
                title: '自动弹出键盘',
                // 副标题短一行：这一行右侧是开关，长文案会被省略号截断。
                subtitle: settings.autoOpenKeyboardOnFocus
                    ? '电脑聚焦输入框时自动打开'
                    : '仅手动点键盘按钮打开',
                // 开关自带点击响应，行本身不再有箭头/点击跳转。
                onTap: () => notifier.setAutoOpenKeyboardOnFocus(
                  !settings.autoOpenKeyboardOnFocus,
                ),
                trailing: AppToggle(
                  value: settings.autoOpenKeyboardOnFocus,
                  onChanged: notifier.setAutoOpenKeyboardOnFocus,
                ),
              ),
              const SettingsRowDivider(),
              SettingsRow(
                title: '自然滚动',
                subtitle: settings.reversePhoneScrollBar
                    ? '跟随 macOS 方向（内容随手指）'
                    : '标准方向（滚动条随手指）',
                onTap: () =>
                    notifier.setReversePhoneScrollBar(
                      !settings.reversePhoneScrollBar,
                    ),
                trailing: AppToggle(
                  value: settings.reversePhoneScrollBar,
                  onChanged: notifier.setReversePhoneScrollBar,
                ),
              ),
              const SettingsRowDivider(),
              _SensitivityRow(
                value: settings.mouseSensitivity,
                onChanged: notifier.saveMouseSensitivity,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

/// 鼠标速度滑杆：拖动时只改本地显示，松手才写进设置。
///
/// 拖动过程里逐帧落盘既浪费写入，也会让控制页的监听者被高频触发；
/// 这里用局部 `_dragging` 承接中间值，`onChangeEnd` 一次持久化。
class _SensitivityRow extends StatefulWidget {
  const _SensitivityRow({required this.value, required this.onChanged});

  final double value;
  final ValueChanged<double> onChanged;

  @override
  State<_SensitivityRow> createState() => _SensitivityRowState();
}

class _SensitivityRowState extends State<_SensitivityRow> {
  double? _dragging;

  double get _shown => _dragging ?? widget.value;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.fromLTRB(
        AppMetrics.cardPadding,
        AppSpacing.md,
        AppMetrics.cardPadding,
        AppSpacing.sm,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text(
                '鼠标速度',
                style: AppText.rowTitle.copyWith(color: palette.textPrimary),
              ),
              const Spacer(),
              StatusTag('${_shown.toStringAsFixed(1)}x'),
            ],
          ),
          Slider(
            value: _shown,
            min: ConnectionConfig.minMouseSensitivity,
            max: ConnectionConfig.maxMouseSensitivity,
            divisions: 14,
            onChanged: (value) => setState(() => _dragging = value),
            onChangeEnd: (value) {
              setState(() => _dragging = null);
              widget.onChanged(value);
            },
          ),
        ],
      ),
    );
  }
}
