/// 连接诊断：把链路关键事实一次摆出来，出问题时不用在设置里来回翻。
///
/// 只读为主，动作只有两个：重新发现局域网电脑、按当前配置重连。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../providers/auth_provider.dart';
import '../providers/feature_facades.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';

class ConnectionDiagnoseScreen extends ConsumerStatefulWidget {
  const ConnectionDiagnoseScreen({super.key});

  @override
  ConsumerState<ConnectionDiagnoseScreen> createState() =>
      _ConnectionDiagnoseScreenState();
}

class _ConnectionDiagnoseScreenState
    extends ConsumerState<ConnectionDiagnoseScreen> {
  bool _busy = false;

  /// 协议版本取「发现列表里同一 IP 的记录」，拿不到就显示未上报。
  String _protocol(List<DiscoveredPc> devices, String? host) {
    for (final device in devices) {
      if (device.host == host) return 'v${device.version}';
    }
    return '—';
  }

  Future<void> _rediscover() async {
    setState(() => _busy = true);
    try {
      await ref.read(connectionProvider.notifier).refreshDiscovery();
      if (!mounted) return;
      final count = ref.read(connectionProvider).discoveredDevices.length;
      context.showSuccessSnackBar(
        count == 0 ? '没有发现局域网内的电脑' : '发现 $count 台电脑',
      );
    } catch (error) {
      if (mounted) context.showErrorSnackBar('发现失败: $error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _reconnect() async {
    setState(() => _busy = true);
    try {
      final ok = await ref.read(connectionProvider.notifier).connectOnly();
      if (!mounted) return;
      ok ? context.showSuccessSnackBar('已重新连接') : context.showErrorSnackBar('连接失败，请检查电脑端服务');
    } catch (error) {
      if (mounted) context.showErrorSnackBar('连接失败: $error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final settings = ref.watch(settingsProvider);
    final auth = ref.watch(connectionProvider);
    final ready = ref.watch(connectionReadyProvider);
    final protocol = _protocol(auth.discoveredDevices, settings.pcIpAddress);
    final address = settings.baseUrl == null
        ? '未配置'
        : '${settings.pcIpAddress ?? '—'}:${settings.pcPort}';

    return Scaffold(
      backgroundColor: palette.background,
      body: SafeArea(
        bottom: false,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(
            AppMetrics.pagePadding,
            0,
            AppMetrics.pagePadding,
            AppSpacing.xxl,
          ),
          children: [
            SizedBox(
              height: AppMetrics.navHeight,
              child: Row(
                children: [
                  IconBtn(
                    icon: Icons.chevron_left_rounded,
                    tooltip: '返回',
                    tonal: true,
                    iconSize: 22,
                    onPressed: () => Navigator.of(context).maybePop(),
                  ),
                  const SizedBox(width: AppSpacing.md),
                  Text(
                    '连接诊断',
                    style: AppText.sectionTitle.copyWith(
                      color: palette.textPrimary,
                    ),
                  ),
                  const Spacer(),
                  IconBtn(
                    icon: Icons.refresh_rounded,
                    tooltip: '重新发现',
                    tonal: true,
                    onPressed: _busy ? null : _rediscover,
                  ),
                ],
              ),
            ),
            const SizedBox(height: AppSpacing.md),
            _StatusBanner(ready: ready, auth: auth),
            const SizedBox(height: AppSpacing.lg),
            const _SectionLabel('链路'),
            const SizedBox(height: AppSpacing.sm),
            AppCard(
              radius: AppRadius.lg,
              padding: const EdgeInsets.symmetric(horizontal: AppSpacing.md),
              child: Column(
                children: [
                  _ValueRow(
                    label: '设备',
                    value: (settings.pcName?.isNotEmpty ?? false)
                        ? settings.pcName!
                        : '未配对',
                  ),
                  const _RowDivider(),
                  _ValueRow(label: '地址', value: address),
                  const _RowDivider(),
                  _ValueRow(label: '协议', value: protocol),
                  const _RowDivider(),
                  _ValueRow(
                    label: '通道',
                    value: ready ? 'HTTP + WS 已建立' : '未建立',
                  ),
                  const _RowDivider(),
                  _ValueRow(
                    label: '唤醒地址',
                    value: (settings.macAddress?.isNotEmpty ?? false)
                        ? settings.macAddress!
                        : '未获取',
                  ),
                ],
              ),
            ),
            const SizedBox(height: AppSpacing.lg),
            const _SectionLabel('最近一次错误'),
            const SizedBox(height: AppSpacing.sm),
            AppCard(
              radius: AppRadius.lg,
              padding: const EdgeInsets.all(AppSpacing.md),
              child: Text(
                auth.errorMessage?.isNotEmpty == true
                    ? auth.errorMessage!
                    : '没有记录到错误',
                style: AppText.body.copyWith(
                  color: auth.errorMessage?.isNotEmpty == true
                      ? palette.danger
                      : palette.textMuted,
                ),
              ),
            ),
            const SizedBox(height: AppSpacing.lg),
            Row(
              children: [
                Expanded(
                  child: SecondaryButton(
                    label: '重新发现',
                    icon: Icons.wifi_find_rounded,
                    onPressed: _busy ? null : _rediscover,
                  ),
                ),
                const SizedBox(width: AppSpacing.md),
                Expanded(
                  child: PrimaryButton(
                    label: '重新连接',
                    icon: Icons.link_rounded,
                    onPressed: _busy ? null : _reconnect,
                  ),
                ),
              ],
            ),
            const SizedBox(height: AppSpacing.md),
            Text(
              '诊断只读取本机已保存的连接信息，不会修改电脑端数据。'
              '若一直连不上，先确认电脑上的 QingLink 服务在运行，且两台设备在同一网段。',
              style: AppText.meta.copyWith(color: palette.textSubtle),
            ),
          ],
        ),
      ),
    );
  }
}

/// 顶部状态条：一眼看出「连没连上、卡在哪一步」。
class _StatusBanner extends StatelessWidget {
  const _StatusBanner({required this.ready, required this.auth});

  final bool ready;
  final AuthState auth;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final (String title, String subtitle, Color tone) = switch ((
      ready,
      auth.connection,
    )) {
      (true, _) => ('已连接', '可以开始远程控制', palette.brand),
      (_, AppConnectionState.connecting) => (
        '正在连接',
        '正在与电脑建立通道',
        palette.warn,
      ),
      _ => ('未连接', '手机与电脑之间没有可用通道', palette.textMuted),
    };
    return Container(
      padding: const EdgeInsets.all(AppSpacing.md),
      decoration: BoxDecoration(
        color: ready ? palette.brandTint : palette.surface,
        border: Border.all(
          color: ready
              ? palette.brand.withValues(alpha: 0.32)
              : palette.line,
        ),
        borderRadius: BorderRadius.circular(AppRadius.lg),
      ),
      child: Row(
        children: [
          Icon(
            ready ? Icons.check_circle_rounded : Icons.link_off_rounded,
            size: 20,
            color: tone,
          ),
          const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  style: AppText.rowTitle.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 2),
                Text(
                  subtitle,
                  style: AppText.meta.copyWith(color: palette.textMuted),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: AppText.label.copyWith(color: context.palette.textMuted),
    );
  }
}

class _ValueRow extends StatelessWidget {
  const _ValueRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: AppSpacing.md),
      child: Row(
        children: [
          // 标签定宽，值独占剩余宽度：设备名和 MAC 都是长串，
          // 用 Spacer 会把值挤成 "DESKTOP-C2VPK…"。
          SizedBox(
            width: 64,
            child: Text(
              label,
              style: AppText.body.copyWith(color: palette.textMuted),
            ),
          ),
          const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Text(
              value,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.right,
              style: AppText.body.copyWith(
                color: palette.textPrimary,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _RowDivider extends StatelessWidget {
  const _RowDivider();

  @override
  Widget build(BuildContext context) {
    return Divider(height: 1, thickness: 1, color: context.palette.line);
  }
}
