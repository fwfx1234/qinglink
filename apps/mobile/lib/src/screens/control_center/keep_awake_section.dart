import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/system_controller.dart';
import '../../theme/app_theme.dart';
import '../../utils/snackbar_extension.dart';
import '../../widgets/app_ui.dart';

/// 控制中心的「保持唤醒」分组。
///
/// 开关由 PC 上的服务进程持有（`SetThreadExecutionState` 是 per-thread 状态，
/// 服务用一条常驻线程串行执行开与关）。状态未读到时禁用开关，避免把「未知」
/// 当成「已关闭」而误发一条关闭命令。
class KeepAwakeSection extends ConsumerStatefulWidget {
  const KeepAwakeSection({super.key});

  @override
  ConsumerState<KeepAwakeSection> createState() => _KeepAwakeSectionState();
}

class _KeepAwakeSectionState extends ConsumerState<KeepAwakeSection> {
  bool _busy = false;

  Future<void> _toggle(bool value) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await ref.read(systemControllerProvider.notifier).setKeepAwake(value);
      if (mounted) {
        context.showSuccessSnackBar(value ? '已开启保持唤醒' : '已关闭保持唤醒');
      }
    } catch (error) {
      if (mounted) context.showErrorSnackBar('设置保持唤醒失败: $error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final enabled = ref.watch(systemControllerProvider).keepAwake;
    final subtitle = enabled == null
        ? '正在读取 PC 状态…'
        : enabled
            ? '已阻止系统自动休眠与息屏'
            : '允许系统按电源计划自动休眠';
    return SettingsGroup(
      title: '保持唤醒',
      trailing: _busy
          ? const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : null,
      children: [
        AppListRow(
          icon: Icons.brightness_high_rounded,
          iconTone: AppColors.primary,
          tile: true,
          title: '阻止休眠与息屏',
          subtitle: subtitle,
          minHeight: AppMetrics.listRow,
          padding: const EdgeInsets.symmetric(vertical: 8),
          trailing: AppToggle(
            value: enabled ?? false,
            onChanged: enabled == null || _busy ? null : _toggle,
          ),
        ),
        const SizedBox(height: 6),
        Text(
          '由 PC 上的 QingLink 服务统一持有并持久化，桌面端托盘/设置页与这里共用同一状态。',
          style: AppText.meta.copyWith(color: palette.textMuted),
        ),
      ],
    );
  }
}
