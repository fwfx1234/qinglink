import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/ir_remote.dart';
import '../../providers/ir_remote_provider.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import 'ir_command_screen.dart';
import 'ir_macro_screen.dart';
import 'ir_remote_manage_screen.dart';
import 'ir_widgets.dart';

/// 电视遥控主页面：只负责发送按键，配置入口统一放到子页面。
class TvRemoteScreen extends ConsumerWidget {
  const TvRemoteScreen({super.key});

  void _push(BuildContext context, Widget page) =>
      Navigator.of(context).push(MaterialPageRoute(builder: (_) => page));

  Future<void> _send(
    BuildContext context,
    WidgetRef ref,
    IrCommand command,
  ) async {
    try {
      await ref.read(irRemoteProvider.notifier).transmit(command);
      if (context.mounted) showIrMessage(context, '已发送“${command.name}”');
    } catch (error) {
      if (context.mounted) showIrMessage(context, error.toString());
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final state = ref.watch(irRemoteProvider);
    final profile = state.activeProfile;
    if (state.loading || profile == null) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    final extras = IrRemotePanel.extraCommands(profile);
    return Scaffold(
      appBar: AppBar(
        title: const Text('电视遥控'),
        actions: [
          IconButton(
            onPressed: () =>
                _push(context, IrCommandScreen(profileId: profile.id)),
            icon: const Icon(Icons.keyboard_alt_outlined),
            tooltip: '按键与信号',
          ),
          IconButton(
            onPressed: () =>
                _push(context, IrMacroScreen(profileId: profile.id)),
            icon: const Icon(Icons.playlist_play_rounded),
            tooltip: '控制流程',
          ),
          IconButton(
            onPressed: () => _push(context, const IrRemoteManageScreen()),
            icon: const Icon(Icons.tune_rounded),
            tooltip: '遥控器管理',
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          if (!state.capability.supported) ...[
            const IrCapabilityBanner(supported: false),
            const SizedBox(height: 14),
          ],
          IrRemotePanel(
            profile: profile,
            capability: state.capability.supported,
            onCommand: (command) => _send(context, ref, command),
          ),
          if (extras.isNotEmpty) ...[
            const SizedBox(height: 20),
            SectionHeading(
              title: '更多按键',
              trailing: Text(
                '${extras.length} 个',
                style: AppText.meta.copyWith(color: palette.textSubtle),
              ),
            ),
            GridView.builder(
              itemCount: extras.length,
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                crossAxisCount: 3,
                childAspectRatio: 2.5,
                mainAxisSpacing: 6,
                crossAxisSpacing: 6,
              ),
              itemBuilder: (context, index) {
                final command = extras[index];
                return IrRemoteKey(
                  label: command.name,
                  onPressed: () => _send(context, ref, command),
                );
              },
            ),
          ],
        ],
      ),
    );
  }
}
