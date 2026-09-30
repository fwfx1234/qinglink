import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../providers/settings_provider.dart';
import '../utils/snackbar_extension.dart';
import '../utils/tv_power_follow.dart';
import '../utils/wake_on_lan.dart';
import 'app_ui.dart';

/// Shown whenever the app is not connected to the PC. Offers a
/// Wake-on-LAN action so the user can boot the PC without leaving the tab.
class UnconnectedState extends ConsumerStatefulWidget {
  final String message;

  const UnconnectedState({super.key, this.message = '请先发现并选择局域网内的 PC'});

  @override
  ConsumerState<UnconnectedState> createState() => _UnconnectedStateState();
}

class _UnconnectedStateState extends ConsumerState<UnconnectedState> {
  bool _busy = false;

  Future<void> _wake() async {
    if (_busy) return;
    final config = ref.read(settingsProvider);
    final mac = config.macAddress;
    if (mac == null || mac.isEmpty) {
      context.showWarningSnackBar('请先完成 PC 配对以获取 MAC 地址');
      return;
    }
    setState(() => _busy = true);
    try {
      const prefix = 24;
      final target = getSubnetBroadcast(config.pcIpAddress, prefix);
      await WakeOnLan.wake(
        macAddress: mac,
        address: target,
        networks: config.wakeNetworks.isEmpty ? null : config.wakeNetworks,
      );
      unawaited(followTvPowerAfterWake(ref));
      if (mounted) context.showInfoSnackBar('已发送唤醒信号，正在连接…');
      final stopwatch = Stopwatch()..start();
      while (mounted && stopwatch.elapsed < const Duration(seconds: 90)) {
        await ref.read(connectionProvider.notifier).connectOnly();
        if (ref.read(connectionProvider).isAuthenticated) break;
        await Future<void>.delayed(const Duration(seconds: 3));
      }
    } catch (error) {
      if (mounted) context.showErrorSnackBar('唤醒失败: $error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _retry() async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await ref.read(connectionProvider.notifier).connectOnly();
    } catch (error) {
      if (mounted) context.showErrorSnackBar('连接失败: $error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final mac = ref.watch(settingsProvider.select((s) => s.macAddress));
    final canWake = mac != null && mac.isNotEmpty;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.link_off_rounded,
              size: 64,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.3),
            ),
            const SizedBox(height: 16),
            Text(
              '未连接电脑',
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
              ),
            ),
            const SizedBox(height: 8),
            Text(
              widget.message,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurface.withValues(alpha: 0.3),
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 20),
            PrimaryButton(
              label: canWake ? '唤醒电脑' : '重试连接',
              icon: canWake
                  ? Icons.power_settings_new_rounded
                  : Icons.refresh_rounded,
              busy: _busy,
              onPressed: canWake ? _wake : _retry,
            ),
            if (canWake) ...[
              const SizedBox(height: 4),
              TextButton(
                onPressed: _busy ? null : _retry,
                child: const Text('仅重试连接'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
