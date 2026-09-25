import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../providers/auth_provider.dart';
import '../providers/remote_control_provider.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../utils/snackbar_extension.dart';
import '../utils/tv_power_follow.dart';
import '../utils/wake_on_lan.dart';
import '../widgets/app_ui.dart';

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

  Future<void> _runRemoteAction(
    Future<void> Function() action,
    String successMessage,
  ) async {
    try {
      await action();
      if (!mounted) return;
      context.showSuccessSnackBar(successMessage);
    } catch (e) {
      if (!mounted) return;
      context.showErrorSnackBar('操作失败: $e');
    }
  }

  Future<void> _wakeOnLan() async {
    final config = ref.read(settingsProvider);
    if (config.macAddress == null || config.macAddress!.isEmpty) {
      context.showWarningSnackBar('请先完成配对以获取 MAC 地址');
      return;
    }
    setState(() => _isWakingUp = true);
    try {
      final prefix =
          ref.read(remoteControlProvider.notifier).subnetPrefix ?? 24;
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

  void _clearSettings() {
    showConfirmDialog(
      context,
      title: '清除设置',
      message: '将清除保存的连接信息和 Token。',
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
    final authState = ref.watch(authProvider);

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
              title: '配对与连接',
              trailing: IconBtn(
                icon: Icons.close_rounded,
                tooltip: '返回',
                onPressed: () => Navigator.of(context).maybePop(),
              ),
            ),
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
                        icon: Icons.lock_rounded,
                        label: '锁屏',
                        onPressed: () => _runRemoteAction(
                          () => ref.read(remoteControlProvider.notifier).lock(),
                          '锁屏指令已执行',
                        ),
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
            const SizedBox(height: 12),
            SettingsGroup(
              title: 'Windows 解锁',
              children: [
                AppListRow(
                  icon: Icons.password_rounded,
                  iconTone: AppColors.amber,
                  title: '登录密码',
                  subtitle: settings.lockPasswordConfigured
                      ? '已配置，解锁时自动输入'
                      : '未配置，将尝试确认解锁；需要 PIN 时请手动输入',
                  showChevron: true,
                  onTap: _editLockPassword,
                ),
                if (settings.lockPasswordConfigured)
                  SecondaryButton(
                    label: '清除已保存密码',
                    icon: Icons.delete_outline_rounded,
                    expand: true,
                    onPressed: _clearLockPassword,
                  ),
              ],
            ),
            const SizedBox(height: 12),
            SettingsGroup(
              title: '局域网电脑',
              children: [
                _DiscoveryList(
                  onSelect: (device) async {
                    final connected = await ref
                        .read(authProvider.notifier)
                        .selectDevice(device);
                    if (!context.mounted) return;
                    if (!connected &&
                        ref.read(authProvider).errorMessage != null) {
                      context.showErrorSnackBar(
                        ref.read(authProvider).errorMessage!,
                      );
                    } else if (connected) {
                      context.showSuccessSnackBar('已选择 ${device.name}');
                    }
                  },
                ),
              ],
            ),
            const SizedBox(height: 12),
            SettingsGroup(
              title: '远程开机',
              children: [
                AppListRow(
                  icon: Icons.wifi_tethering_rounded,
                  title: 'Wake-on-LAN',
                  subtitle: _isWakingUp ? '正在发送…' : '远程唤醒电脑',
                  showChevron: !_isWakingUp,
                  trailing: _isWakingUp
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: AppColors.primary,
                          ),
                        )
                      : null,
                  onTap: _isWakingUp ? null : _wakeOnLan,
                ),
              ],
            ),
            const SizedBox(height: 12),
            SettingsGroup(
              title: '连接信息',
              children: [
                _InfoRow(
                  label: '状态',
                  value: authState.isAuthenticated ? '已连接' : '未连接',
                  valueColor: authState.isAuthenticated
                      ? AppColors.success
                      : AppColors.warning,
                ),
                _InfoRow(label: 'PC 地址', value: settings.baseUrl ?? '自动发现中'),
                _InfoRow(label: 'PC 名称', value: settings.pcName ?? '未发现'),
                _InfoRow(label: 'MAC 地址', value: settings.macAddress ?? '未配置'),
                _InfoRow(
                  label: '配对状态',
                  value: settings.isPaired ? '已配对' : '未配对',
                  valueColor: settings.isPaired ? AppColors.success : null,
                  last: true,
                ),
              ],
            ),
            const SizedBox(height: 16),
            if (authState.isAuthenticated)
              SecondaryButton(
                label: '断开连接',
                expand: true,
                onPressed: () {
                  ref.read(authProvider.notifier).logout();
                  context.showInfoSnackBar('已断开连接');
                },
              ),
            const SizedBox(height: 8),
            TextButton(
              onPressed: _clearSettings,
              style: TextButton.styleFrom(foregroundColor: palette.textSubtle),
              child: const Text('清除所有设置'),
            ),
          ],
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.label,
    required this.value,
    this.valueColor,
    this.last = false,
  });

  final String label;
  final String value;
  final Color? valueColor;
  final bool last;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 9),
      decoration: BoxDecoration(
        border: Border(
          bottom: BorderSide(color: last ? Colors.transparent : palette.line),
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: AppText.body.copyWith(color: palette.textSubtle)),
          const SizedBox(width: 12),
          Flexible(
            child: Text(
              value,
              style: AppText.body.copyWith(
                fontWeight: FontWeight.w600,
                color: valueColor ?? palette.textPrimary,
              ),
              textAlign: TextAlign.end,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

class _DiscoveryList extends ConsumerWidget {
  final Future<void> Function(DiscoveredPc device) onSelect;

  const _DiscoveryList({required this.onSelect});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final devices = ref.watch(
      authProvider.select((state) => state.discoveredDevices),
    );
    if (devices.isEmpty) {
      return Row(
        children: [
          const Icon(Icons.wifi_find_rounded, color: AppColors.cyan, size: 18),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              '正在搜索局域网内的电脑…',
              style: AppText.body.copyWith(color: palette.textMuted),
            ),
          ),
        ],
      );
    }
    return Column(
      children: devices
          .map(
            (device) => AppListRow(
              icon: Icons.computer_rounded,
              title: device.name,
              subtitle: '${device.host}:${device.port}',
              showChevron: true,
              onTap: () => onSelect(device),
            ),
          )
          .toList(),
    );
  }
}
