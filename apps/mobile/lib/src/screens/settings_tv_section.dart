/// 设置页「电视联动」分组：电视电源是否跟随电脑的唤醒与关机。
///
/// 这两项原先散落在「更多」页底部的开关组里，和设置页的「输入」分组各自一半；
/// 统一收进设置页后，「更多」只保留入口，不再承载开关状态。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import 'settings_widgets.dart';

/// 设置页的「电视联动」分组。
class SettingsTvSection extends ConsumerWidget {
  const SettingsTvSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(settingsProvider);
    final notifier = ref.read(settingsProvider.notifier);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SettingsSectionLabel('电视联动'),
        const SizedBox(height: AppSpacing.sm),
        AppCard(
          radius: AppRadius.lg,
          padding: EdgeInsets.zero,
          child: Column(
            children: [
              SettingsRow(
                title: '跟随电脑开电视',
                subtitle: '发送唤醒时同步按下遥控器电源键',
                onTap: () => notifier.setTvFollowPcPowerOn(
                  !settings.tvFollowPcPowerOn,
                ),
                trailing: AppToggle(
                  value: settings.tvFollowPcPowerOn,
                  onChanged: notifier.setTvFollowPcPowerOn,
                ),
              ),
              const SettingsRowDivider(),
              SettingsRow(
                title: '跟随电脑关电视',
                subtitle: '关机或睡眠时同步按下遥控器电源键',
                onTap: () => notifier.setTvFollowPcPowerOff(
                  !settings.tvFollowPcPowerOff,
                ),
                trailing: AppToggle(
                  value: settings.tvFollowPcPowerOff,
                  onChanged: notifier.setTvFollowPcPowerOff,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
