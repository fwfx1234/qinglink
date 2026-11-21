/// 设置页的「局域网电脑」发现结果弹层。
///
/// 只负责展示 `connectionProvider` 里的发现结果与重搜入口；打开它的动作、
/// 连接后的页面跳转仍由 `settings_tab.dart` 负责。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';

/// 局域网电脑发现结果弹层。
class SettingsDiscoverySheet extends ConsumerWidget {
  const SettingsDiscoverySheet({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final devices = ref.watch(
      connectionProvider.select((state) => state.discoveredDevices),
    );
    return Container(
      padding: EdgeInsets.fromLTRB(
        AppSpacing.xl,
        AppSpacing.md,
        AppSpacing.xl,
        MediaQuery.of(context).viewPadding.bottom + AppSpacing.lg,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Center(
            child: Container(
              width: 36,
              height: 4,
              decoration: BoxDecoration(
                color: palette.lineStrong,
                borderRadius: BorderRadius.circular(AppRadius.pill),
              ),
            ),
          ),
          const SizedBox(height: AppSpacing.lg),
          Text(
            '局域网电脑',
            style: AppText.cardTitle.copyWith(color: palette.textPrimary),
          ),
          const SizedBox(height: 2),
          Text(
            devices.isEmpty ? '没有发现可连接的电脑' : '点击选择要连接的电脑',
            style: AppText.meta.copyWith(color: palette.textMuted),
          ),
          const SizedBox(height: AppSpacing.md),
          if (devices.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: AppSpacing.lg),
              child: Text(
                '确认电脑与手机在同一 Wi-Fi，并已打开电脑端 QingLink。',
                style: AppText.meta.copyWith(color: palette.textMuted),
              ),
            )
          else
            for (final device in devices) ...[
              AppListRow(
                icon: Icons.computer_rounded,
                title: device.name,
                subtitle: '${device.host}:${device.port}',
                showChevron: true,
                onTap: () async {
                  final connected = await ref
                      .read(connectionProvider.notifier)
                      .selectDevice(device);
                  if (!context.mounted) return;
                  Navigator.of(context).maybePop();
                  if (!connected &&
                      ref.read(connectionProvider).errorMessage != null) {
                    context.showErrorSnackBar(
                      ref.read(connectionProvider).errorMessage!,
                    );
                  } else if (connected) {
                    context.showSuccessSnackBar('已选择 ${device.name}');
                  }
                },
              ),
              const SizedBox(height: AppSpacing.sm),
            ],
          const SizedBox(height: AppSpacing.sm),
          SecondaryButton(
            label: '重新搜索',
            icon: Icons.wifi_find_rounded,
            expand: true,
            onPressed: () =>
                ref.read(connectionProvider.notifier).refreshDiscovery(),
          ),
        ],
      ),
    );
  }
}
