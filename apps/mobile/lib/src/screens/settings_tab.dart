/// S8 设置（对应设计稿 `S08-settings-*.png`）。
///
/// 结构自上而下：顶栏（返回 + 标题）→ 电源四宫格 → 解锁与唤醒 → 输入 →
/// 电视联动 → 连接信息 → 断开当前连接 → 清除所有设置并退出配对。
///
/// **这里是全机唯一的设置界面**：所有偏好开关都在这一页，手机端不再有第二个
/// 设置入口（「更多」页只保留跳转，控制页顶栏的齿轮同样跳到这里）。
///
/// 页面只做装配与动作分发：通用行/卡片在 `settings_widgets.dart`，
/// 「输入」分组在 `settings_input_section.dart`，「电视联动」分组在
/// `settings_tv_section.dart`，发现电脑的弹层在 `settings_discovery_sheet.dart`。
library;

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../app/routes.dart';
import '../models/auth.dart';
import '../providers/app_version_provider.dart';
import '../providers/auth_provider.dart';
import '../providers/power_controller.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../utils/snackbar_extension.dart';
import '../utils/tv_power_follow.dart';
import '../utils/wake_on_lan.dart';
import '../widgets/app_ui.dart';
import '../widgets/app_sheet.dart';
import 'settings_discovery_sheet.dart';
import 'settings_input_section.dart';
import 'settings_tv_section.dart';
import 'settings_widgets.dart';

class SettingsTab extends ConsumerStatefulWidget {
  const SettingsTab({super.key});

  @override
  ConsumerState<SettingsTab> createState() => _SettingsTabState();
}

class _SettingsTabState extends ConsumerState<SettingsTab> {
  bool _isWakingUp = false;

  Future<void> _editLockPassword() async {
    final controller = TextEditingController();
    final password = await showDialog<String>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Windows 登录密码'),
        content: TextField(
          controller: controller,
          obscureText: true,
          autofocus: true,
          keyboardType: TextInputType.number,
          decoration: const InputDecoration(
            hintText: '输入 4-32 位 Windows 登录 PIN',
          ),
          onSubmitted: (value) => Navigator.of(dialogContext).pop(value.trim()),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () =>
                Navigator.of(dialogContext).pop(controller.text.trim()),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (password == null || password.isEmpty || !mounted) return;
    try {
      await ref.read(settingsProvider.notifier).saveLockPassword(password);
      if (mounted) context.showSuccessSnackBar('登录密码已保存');
    } catch (error) {
      if (mounted) context.showErrorSnackBar('保存失败: $error');
    }
  }

  Future<void> _clearLockPassword() async {
    await ref.read(settingsProvider.notifier).clearLockPassword();
    if (mounted) context.showSuccessSnackBar('登录密码已清除');
  }

  Future<void> _wakeOnLan() async {
    final config = ref.read(settingsProvider);
    if (config.macAddress == null || config.macAddress!.isEmpty) {
      context.showWarningSnackBar('请先完成配对以获取 MAC 地址');
      return;
    }
    setState(() => _isWakingUp = true);
    try {
      const prefix = 24;
      final targetAddress = getSubnetBroadcast(config.pcIpAddress, prefix);
      await WakeOnLan.wake(
        macAddress: config.macAddress!,
        address: targetAddress,
        networks: config.wakeNetworks.isEmpty ? null : config.wakeNetworks,
      );
      unawaited(followTvPowerAfterWake(ref));
      if (!mounted) return;
      context.showSuccessSnackBar('已发送唤醒信号');
    } catch (e) {
      if (!mounted) return;
      context.showErrorSnackBar('唤醒失败: $e');
    } finally {
      if (mounted) setState(() => _isWakingUp = false);
    }
  }

  /// 重新发现局域网电脑：先刷新，再用底部弹层承接发现结果。
  Future<void> _rediscover() async {
    final notifier = ref.read(connectionProvider.notifier);
    await notifier.refreshDiscovery();
    if (!mounted) return;
    await showAppSheet<void>(
      context,
      builder: (_) => const SettingsDiscoverySheet(),
    );
  }

  void _disconnect() {
    showConfirmDialog(
      context,
      title: '断开当前连接',
      message: '断开后需要重新配对才能继续控制这台电脑。',
      confirmLabel: '断开',
      danger: true,
    ).then((confirmed) {
      if (confirmed && mounted) {
        ref.read(connectionProvider.notifier).logout();
        context.showInfoSnackBar('已断开连接');
      }
    });
  }

  void _clearSettings() {
    showConfirmDialog(
      context,
      title: '清除所有设置',
      message: '将清除保存的连接信息、配对令牌与安全凭据。',
      confirmLabel: '清除',
      danger: true,
    ).then((confirmed) {
      if (confirmed && mounted) {
        ref.read(settingsProvider.notifier).clearAll();
        context.showSuccessSnackBar('已清除所有设置');
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final settings = ref.watch(settingsProvider);
    final authState = ref.watch(connectionProvider);
    final protocol = _protocolVersion(authState.discoveredDevices, settings);
    final versionAsync = ref.watch(appVersionProvider);
    final versionLabel =
        versionAsync.valueOrNull?.displayVersion ?? '未知';

    return Scaffold(
      backgroundColor: palette.background,
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(
            AppMetrics.pagePadding,
            0,
            AppMetrics.pagePadding,
            AppSpacing.xxl,
          ),
          children: [
            SettingsNavBar(onBack: () => Navigator.of(context).maybePop()),
            const SettingsSectionLabel('电源'),
            const SizedBox(height: AppSpacing.sm),
            SettingsPowerTiles(
              onSleep: () => confirmPowerAction(context, ref, 'sleep'),
              onLock: () => _runRemoteAction(
                context,
                () => ref.read(powerControllerProvider).lock(),
                '锁屏指令已执行',
              ),
              onRestart: () => confirmPowerAction(context, ref, 'restart'),
              onShutdown: () => confirmPowerAction(context, ref, 'shutdown'),
            ),
            const SizedBox(height: AppSpacing.xl),
            const SettingsSectionLabel('解锁与唤醒'),
            const SizedBox(height: AppSpacing.sm),
            AppCard(
              radius: AppRadius.lg,
              padding: EdgeInsets.zero,
              child: Column(
                children: [
                  SettingsRow(
                    title: '发送解锁 PIN',
                    subtitle: settings.lockPasswordConfigured
                        ? '已保存，解锁时自动输入'
                        : '用于解锁 Windows 登录界面',
                    onTap: _editLockPassword,
                    trailing: settings.lockPasswordConfigured
                        ? TextButton(
                            onPressed: _clearLockPassword,
                            child: Text(
                              '清除',
                              style: AppText.label.copyWith(
                                color: palette.textMuted,
                              ),
                            ),
                          )
                        : null,
                  ),
                  const SettingsRowDivider(),
                  SettingsRow(
                    title: '唤醒后自动解锁',
                    subtitle: settings.autoUnlockOnWake
                        ? '发送唤醒后自动输入已保存的 PIN'
                        : '唤醒后停在 Windows 登录界面',
                    onTap: () => ref
                        .read(settingsProvider.notifier)
                        .setAutoUnlockOnWake(!settings.autoUnlockOnWake),
                    trailing: AppToggle(
                      value: settings.autoUnlockOnWake,
                      onChanged: ref
                          .read(settingsProvider.notifier)
                          .setAutoUnlockOnWake,
                    ),
                  ),
                  const SettingsRowDivider(),
                  SettingsRow(
                    title: '唤醒电脑',
                    subtitle: settings.macAddress?.isNotEmpty == true
                        ? 'Wake-on-LAN · ${settings.macAddress}'
                        : 'Wake-on-LAN · 未获取到 MAC',
                    busy: _isWakingUp,
                    onTap: _isWakingUp ? null : _wakeOnLan,
                  ),
                ],
              ),
            ),
            const SizedBox(height: AppSpacing.xl),
            const SettingsInputSection(),
            const SizedBox(height: AppSpacing.xl),
            const SettingsTvSection(),
            const SizedBox(height: AppSpacing.xl),
            const SettingsSectionLabel('连接信息'),
            const SizedBox(height: AppSpacing.sm),
            AppCard(
              radius: AppRadius.lg,
              padding: const EdgeInsets.symmetric(
                horizontal: AppMetrics.cardPadding,
                vertical: AppSpacing.xs,
              ),
              child: Column(
                children: [
                  SettingsRow(
                    title: '重新发现局域网电脑',
                    subtitle: settings.baseUrl == null
                        ? '当前未连接'
                        : '当前 ${settings.pcIpAddress ?? '—'} · '
                              '${settings.pcPort} UDP',
                    onTap: _rediscover,
                  ),
                  const SettingsRowDivider(),
                  SettingsValueRow(
                    label: '设备',
                    value: _deviceLabel(settings),
                  ),
                  const SettingsRowDivider(),
                  SettingsValueRow(
                    label: '地址',
                    value: settings.baseUrl == null
                        ? '未连接'
                        : '${settings.pcIpAddress ?? '—'}:${settings.pcPort}',
                  ),
                  const SettingsRowDivider(),
                  SettingsRow(
                    title: '检查更新',
                    subtitle: '获取当前版本 $versionLabel',
                    onTap: () => context.push(AppRoutes.update),
                  ),
                  const SettingsRowDivider(),
                  SettingsValueRow(
                    label: '版本',
                    value: 'QingLink $versionLabel · 协议 $protocol',
                  ),
                ],
              ),
            ),
            const SizedBox(height: AppSpacing.lg),
            if (authState.isAuthenticated)
              SettingsDangerButton(
                label: '断开当前连接',
                onPressed: _disconnect,
              ),
            const SizedBox(height: AppSpacing.md),
            Center(
              child: TextButton(
                onPressed: _clearSettings,
                child: Text(
                  '清除所有设置并退出配对',
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// 设备行文案：名称 + 系统（设计稿为「客厅主机 · Windows 11」）。
  String _deviceLabel(ConnectionConfig settings) {
    final name = settings.pcName;
    if (name == null || name.isEmpty) return '未连接';
    return '$name · Windows';
  }

  /// 协议版本取自最近一次局域网发现结果，取不到时按 3 展示。
  int _protocolVersion(List<DiscoveredPc> devices, ConnectionConfig settings) {
    for (final device in devices) {
      if (device.host == settings.pcIpAddress) return device.version;
    }
    return devices.isEmpty ? 3 : devices.first.version;
  }
}

/// 执行远程动作并提示结果：成功提示 [successMessage]，失败提示错误原因。
Future<void> _runRemoteAction(
  BuildContext context,
  Future<void> Function() action,
  String successMessage,
) async {
  try {
    await action();
    if (!context.mounted) return;
    context.showSuccessSnackBar(successMessage);
  } catch (e) {
    if (!context.mounted) return;
    context.showErrorSnackBar('操作失败: $e');
  }
}
