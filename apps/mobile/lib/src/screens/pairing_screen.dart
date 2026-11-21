/// S7 配对与连接（对应设计稿 `S07-pairing-*.png`）。
///
/// 结构自上而下：顶栏（返回 + 标题）→ 待配对设备卡 → 三步进度条 →
/// 配对码输入卡（6 位分格 + 剩余时间 + 主按钮 + 重发）→ 排障说明卡 → 取消配对。
library;

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../providers/settings_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';

/// 配对码有效期，与电脑端弹窗一致（60 秒）。
const int _pinTtlSeconds = 60;

class PairingScreen extends ConsumerStatefulWidget {
  const PairingScreen({super.key});

  @override
  ConsumerState<PairingScreen> createState() => _PairingScreenState();
}

class _PairingScreenState extends ConsumerState<PairingScreen> {
  final _pinController = TextEditingController();
  final _focusNode = FocusNode();
  Timer? _ticker;
  int _remaining = _pinTtlSeconds;
  bool _popScheduled = false;
  bool _resending = false;

  @override
  void initState() {
    super.initState();
    _startCountdown();
    _pinController.addListener(_onPinChanged);
  }

  @override
  void dispose() {
    _ticker?.cancel();
    _pinController
      ..removeListener(_onPinChanged)
      ..dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _onPinChanged() => setState(() {});

  void _startCountdown() {
    _ticker?.cancel();
    _remaining = _pinTtlSeconds;
    _ticker = Timer.periodic(const Duration(seconds: 1), (timer) {
      if (!mounted) {
        timer.cancel();
        return;
      }
      setState(() => _remaining = _remaining > 0 ? _remaining - 1 : 0);
      if (_remaining == 0) timer.cancel();
    });
  }

  String get _pin => _pinController.text.trim();

  bool get _pinValid => RegExp(r'^\d{6}$').hasMatch(_pin);

  void _pop(bool result) {
    if (_popScheduled) return;
    _popScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      FocusManager.instance.primaryFocus?.unfocus();
      Navigator.of(context).pop(result);
    });
  }

  Future<void> _resend() async {
    if (_resending) return;
    setState(() => _resending = true);
    final notifier = ref.read(connectionProvider.notifier);
    try {
      await notifier.cancelPairing();
      await notifier.beginPairing();
      if (mounted) {
        _pinController.clear();
        _startCountdown();
      }
    } catch (_) {
      // 失败原因已写入 authState.errorMessage，由下方错误行呈现。
    } finally {
      if (mounted) setState(() => _resending = false);
    }
  }

  Future<void> _cancel() async {
    await ref.read(connectionProvider.notifier).cancelPairing();
    _pop(false);
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final authState = ref.watch(connectionProvider);
    if (authState.isAuthenticated) _pop(true);

    final ip = ref.watch(settingsProvider.select((s) => s.pcIpAddress));
    final name = ref.watch(settingsProvider.select((s) => s.pcName));
    final pairing = authState.isAuthenticating;

    return Scaffold(
      backgroundColor: palette.background,
      body: SafeArea(
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () => _focusNode.unfocus(),
          child: ListView(
            padding: const EdgeInsets.fromLTRB(
              AppMetrics.pagePadding,
              0,
              AppMetrics.pagePadding,
              AppSpacing.xxl,
            ),
            children: [
              _PairingNavBar(onBack: _cancel),
              _DeviceCard(
                name: (name == null || name.isEmpty) ? 'QingLink 电脑' : name,
                address: (ip == null || ip.isEmpty) ? '局域网发现' : ip,
              ),
              const SizedBox(height: AppSpacing.md),
              const _StepStrip(activeStep: 2),
              const SizedBox(height: AppSpacing.lg),
              _PinCard(
                controller: _pinController,
                focusNode: _focusNode,
                remaining: _remaining,
                busy: pairing,
                error: pairing ? null : authState.errorMessage,
                onSubmit: _pinValid && !pairing
                    ? () => ref
                          .read(connectionProvider.notifier)
                          .pair(_pinController.text.trim())
                    : null,
                onResend: _resending || pairing ? null : _resend,
              ),
              const SizedBox(height: AppSpacing.lg),
              const _TroubleshootCard(),
              const SizedBox(height: AppSpacing.sm),
              Center(
                child: TextButton(
                  onPressed: pairing ? null : _cancel,
                  child: Text(
                    '取消配对',
                    style: AppText.body.copyWith(color: palette.textMuted),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 顶栏：返回键 + 左对齐大标题。
class _PairingNavBar extends StatelessWidget {
  const _PairingNavBar({required this.onBack});

  final VoidCallback onBack;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: AppMetrics.navHeight,
      child: Row(
        children: [
          IconBtn(
            icon: Icons.chevron_left_rounded,
            tooltip: '返回',
            tonal: true,
            iconSize: 22,
            onPressed: onBack,
          ),
          const SizedBox(width: AppSpacing.md),
          Text(
            '配对与连接',
            style: AppText.pageTitle.copyWith(color: palette.textPrimary),
          ),
        ],
      ),
    );
  }
}

/// 待配对设备：图标 + 名称 + 地址 + 状态药丸。
class _DeviceCard extends StatelessWidget {
  const _DeviceCard({required this.name, required this.address});

  final String name;
  final String address;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(AppMetrics.cardPadding),
      child: Row(
        children: [
          Container(
            width: 48,
            height: 48,
            decoration: BoxDecoration(
              color: palette.brandTint,
              borderRadius: BorderRadius.circular(AppRadius.sm),
            ),
            child: Icon(
              Icons.desktop_windows_rounded,
              size: 24,
              color: palette.brand,
            ),
          ),
          const SizedBox(width: AppSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.sectionTitle.copyWith(
                    color: palette.textPrimary,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  '$address · 局域网发现',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textMuted),
                ),
              ],
            ),
          ),
          const SizedBox(width: AppSpacing.sm),
          _Pill(
            label: '待配对',
            color: palette.warn,
            background: palette.warn.withValues(alpha: 0.14),
          ),
        ],
      ),
    );
  }
}

class _Pill extends StatelessWidget {
  const _Pill({
    required this.label,
    required this.color,
    required this.background,
  });

  final String label;
  final Color color;
  final Color background;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 26,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(AppRadius.pill),
      ),
      child: Text(
        label,
        style: AppText.label.copyWith(
          color: color,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

/// 三步进度条：数字 + 步骤名，当前步用品牌色描边与浅底。
class _StepStrip extends StatelessWidget {
  const _StepStrip({required this.activeStep});

  /// 1 点配对 / 2 电脑读数 / 3 输入配对码。
  final int activeStep;

  static const _steps = ['点配对', '电脑读数', '输入配对码'];

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      children: [
        for (var i = 0; i < _steps.length; i++) ...[
          if (i > 0) const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Container(
              height: AppMetrics.minTap,
              alignment: Alignment.center,
              decoration: BoxDecoration(
                color: i == activeStep
                    ? palette.brandTint
                    : palette.surface2,
                border: Border.all(
                  color: i == activeStep ? palette.brand : palette.line,
                ),
                borderRadius: BorderRadius.circular(AppRadius.md),
              ),
              child: Text.rich(
                TextSpan(
                  children: [
                    TextSpan(
                      text: '${i + 1} ',
                      style: TextStyle(
                        color: i == activeStep
                            ? palette.brand
                            : palette.textMuted,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    TextSpan(text: _steps[i]),
                  ],
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: AppText.label.copyWith(
                  color: i == activeStep ? palette.brand : palette.textMuted,
                ),
              ),
            ),
          ),
        ],
      ],
    );
  }
}

/// 配对码输入卡：标题 + 说明 + 6 位分格 + 剩余时间 + 主按钮 + 重发链接。
class _PinCard extends StatelessWidget {
  const _PinCard({
    required this.controller,
    required this.focusNode,
    required this.remaining,
    required this.busy,
    required this.error,
    required this.onSubmit,
    required this.onResend,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final int remaining;
  final bool busy;
  final String? error;
  final VoidCallback? onSubmit;
  final VoidCallback? onResend;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final expired = remaining <= 0;
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(AppMetrics.cardPadding),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '输入电脑上显示的 6 位配对码',
            style: AppText.sectionTitle.copyWith(color: palette.textPrimary),
          ),
          const SizedBox(height: AppSpacing.sm),
          Text(
            '电脑端 QingLink 会弹出配对窗口，里面就是这 6 位数',
            style: AppText.meta.copyWith(color: palette.textMuted),
          ),
          const SizedBox(height: AppSpacing.lg),
          _PinBoxes(
            controller: controller,
            focusNode: focusNode,
            enabled: !busy && !expired,
          ),
          if (error != null) ...[
            const SizedBox(height: AppSpacing.md),
            Row(
              children: [
                Icon(
                  Icons.error_outline_rounded,
                  size: 16,
                  color: palette.danger,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    error!,
                    style: AppText.meta.copyWith(color: palette.danger),
                  ),
                ),
              ],
            ),
          ],
          const SizedBox(height: AppSpacing.md),
          Row(
            children: [
              Text(
                '配对码剩余',
                style: AppText.meta.copyWith(color: palette.textMuted),
              ),
              const Spacer(),
              Text(
                expired ? '已过期' : '$remaining 秒',
                style: AppText.label.copyWith(
                  color: expired ? palette.danger : palette.warn,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.sm),
          ClipRRect(
            borderRadius: BorderRadius.circular(AppRadius.pill),
            child: LinearProgressIndicator(
              value: remaining / _pinTtlSeconds,
              minHeight: 6,
              color: expired ? palette.danger : palette.brand,
              backgroundColor: palette.surface3,
            ),
          ),
          const SizedBox(height: AppSpacing.lg),
          PrimaryButton(
            label: '配对连接',
            expand: true,
            busy: busy,
            onPressed: onSubmit,
          ),
          const SizedBox(height: AppSpacing.sm),
          Center(
            child: TextButton(
              onPressed: onResend,
              child: Text(
                '没收到弹窗？重新发送配对请求',
                style: AppText.label.copyWith(
                  color: onResend == null ? palette.textSubtle : palette.brand,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// 6 位分格输入：真实输入由一个透明 `TextField` 承载，分格只做视觉。
class _PinBoxes extends StatelessWidget {
  const _PinBoxes({
    required this.controller,
    required this.focusNode,
    required this.enabled,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final text = controller.text;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: enabled ? focusNode.requestFocus : null,
      child: Stack(
        children: [
          Row(
            children: [
              for (var i = 0; i < 6; i++) ...[
                if (i > 0) const SizedBox(width: AppSpacing.sm),
                Expanded(
                  child: _PinBox(
                    char: i < text.length ? text[i] : '',
                    // 输入焦点所在格：已输入位数（最多停在末格）。
                    active:
                        enabled &&
                        focusNode.hasFocus &&
                        i == text.length.clamp(0, 5),
                  ),
                ),
              ],
            ],
          ),
          Positioned.fill(
            child: Opacity(
              opacity: 0,
              child: TextField(
                controller: controller,
                focusNode: focusNode,
                enabled: enabled,
                autofocus: true,
                keyboardType: TextInputType.number,
                inputFormatters: [
                  FilteringTextInputFormatter.digitsOnly,
                  LengthLimitingTextInputFormatter(6),
                ],
                showCursor: false,
                decoration: const InputDecoration(
                  counterText: '',
                  border: InputBorder.none,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _PinBox extends StatelessWidget {
  const _PinBox({required this.char, required this.active});

  final String char;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final filled = char.isNotEmpty;
    return Container(
      height: 64,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: filled || active ? palette.brandTint : palette.surface2,
        border: Border.all(
          color: active
              ? palette.brand
              : filled
              ? palette.brand.withValues(alpha: 0.45)
              : palette.line,
          width: active ? 1.6 : 1,
        ),
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: active && !filled
          ? Container(width: 2, height: 26, color: palette.brand)
          : Text(
              char,
              style: TextStyle(
                fontSize: 26,
                height: 1,
                fontWeight: FontWeight.w600,
                color: palette.textPrimary,
              ),
            ),
    );
  }
}

/// 排障说明卡：标题 + 三条「现象 → 处理」。
class _TroubleshootCard extends StatelessWidget {
  const _TroubleshootCard();

  static const _items = [
    ('QingLink 没有在电脑上运行', '先打开电脑端 QingLink'),
    ('电脑锁屏或服务已退出', '解锁电脑后重新发送请求'),
    ('电脑上已有待确认的弹窗', '关掉旧弹窗再来一次'),
  ];

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(AppMetrics.cardPadding),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '电脑端没反应？',
            style: AppText.cardTitle.copyWith(color: palette.textPrimary),
          ),
          for (var i = 0; i < _items.length; i++) ...[
            if (i == 0)
              const SizedBox(height: AppSpacing.md)
            else
              Padding(
                padding: const EdgeInsets.symmetric(vertical: AppSpacing.md),
                child: Divider(height: 1, color: palette.line),
              ),
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsets.only(top: 6),
                  child: Container(
                    width: 4,
                    height: 4,
                    decoration: BoxDecoration(
                      color: palette.textSubtle,
                      shape: BoxShape.circle,
                    ),
                  ),
                ),
                const SizedBox(width: AppSpacing.sm),
                Expanded(
                  child: Text.rich(
                    TextSpan(
                      children: [
                        TextSpan(
                          text: _items[i].$1,
                          style: TextStyle(color: palette.textMuted),
                        ),
                        TextSpan(
                          text: ' → ',
                          style: TextStyle(color: palette.textSubtle),
                        ),
                        TextSpan(
                          text: _items[i].$2,
                          style: TextStyle(color: palette.textPrimary),
                        ),
                      ],
                    ),
                    style: AppText.meta,
                  ),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}
