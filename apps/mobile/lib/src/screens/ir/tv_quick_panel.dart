import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../utils/tv_power_follow.dart';
import '../../widgets/app_ui.dart';
import 'ir_widgets.dart';
import 'tv_remote_screen.dart';

/// 首页快捷面板：常用电视按键 + 完整遥控器入口。
Future<void> showTvQuickPanel(BuildContext context) {
  return showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    builder: (_) => const TvQuickPanel(),
  );
}

class TvQuickPanel extends ConsumerWidget {
  const TvQuickPanel({super.key});

  Future<void> _send(
    BuildContext context,
    WidgetRef ref,
    String id, {
    List<String> nameKeywords = const [],
  }) async {
    final command = findIrCommand(
      ref.read(irRemoteProvider).activeProfile,
      id,
      nameKeywords: nameKeywords,
    );
    if (command == null) {
      showIrMessage(context, '当前遥控器没有该按键', error: true);
      return;
    }
    try {
      await ref.read(irRemoteProvider.notifier).transmit(command);
      if (context.mounted) showIrMessage(context, '已发送“${command.name}”');
    } catch (error) {
      if (context.mounted) showIrMessage(context, error.toString(), error: true);
    }
  }

  void _openRemote(BuildContext context) {
    Navigator.of(context).pop();
    Navigator.of(context).push(
      MaterialPageRoute(builder: (_) => const TvRemoteScreen()),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final profile = ref.watch(irRemoteProvider).activeProfile;
    if (profile == null) return const SizedBox.shrink();
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 0, 20, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                TileIcon(
                  icon: Icons.tv_rounded,
                  color: AppColors.cyan,
                  size: 34,
                  radius: 9,
                  iconSize: 18,
                ),
                const SizedBox(width: 9),
                Expanded(
                  child: Text(
                    profile.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AppText.cardTitle.copyWith(
                      color: palette.textPrimary,
                    ),
                  ),
                ),
                TextButton.icon(
                  onPressed: () => _openRemote(context),
                  icon: const Icon(Icons.open_in_new_rounded, size: 16),
                  label: const Text('遥控器'),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Row(
              children: [
                Expanded(
                  child: IrRemoteKey(
                    label: '电源',
                    icon: Icons.power_settings_new_rounded,
                    accent: true,
                    onPressed: () => _send(
                      context,
                      ref,
                      'power',
                      nameKeywords: const ['电源', '开关'],
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: IrRemoteKey(
                    label: '信号源',
                    icon: Icons.input_rounded,
                    onPressed: () => _send(
                      context,
                      ref,
                      'input',
                      nameKeywords: const ['信号源'],
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: IrRemoteKey(
                    label: '上',
                    icon: Icons.keyboard_arrow_up_rounded,
                    onPressed: () => _send(context, ref, 'up'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: IrRemoteKey(
                    label: '下',
                    icon: Icons.keyboard_arrow_down_rounded,
                    onPressed: () => _send(context, ref, 'down'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: IrRemoteKey(
                    label: '确认',
                    accent: true,
                    onPressed: () => _send(context, ref, 'ok'),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
