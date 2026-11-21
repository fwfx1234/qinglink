/// 首页状态卡：在线时是一行紧凑卡片，中断态是整卡 + 主按钮。
///
/// 状态数据与判定逻辑在 `home_status.dart`。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';
import 'home_status.dart';

/// 首页顶部的电脑状态卡，主按钮行为由 [HomeStatusView.action] 决定。
class HomeStatusCard extends StatelessWidget {
  const HomeStatusCard({super.key, required this.status, this.onPrimary});

  final HomeStatusView status;
  final VoidCallback? onPrimary;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            palette.statusGradientStart,
            palette.statusGradientEnd,
          ],
        ),
        border: Border.all(color: palette.statusBorder),
        borderRadius: BorderRadius.circular(AppRadius.lg),
      ),
      // 设计稿卡片高 56：图标 40 + 上下各 8，横向留 12 让药丸更贴近右边。
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.md,
        vertical: AppSpacing.sm,
      ),
      child: status.kind == HomeStatusKind.ready
          ? _ReadyRow(status: status)
          : _InterruptPanel(status: status, onPrimary: onPrimary),
    );
  }
}

/// 在线态：图标 + 设备名/说明 + 在线药丸，不再占用主按钮位置。
class _ReadyRow extends StatelessWidget {
  const _ReadyRow({required this.status});

  final HomeStatusView status;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      children: [
        Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: palette.brand.withValues(alpha: 0.14),
            borderRadius: BorderRadius.circular(AppRadius.md),
          ),
          child: Icon(
            Icons.desktop_windows_rounded,
            size: 20,
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
                status.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: AppText.cardTitle.copyWith(color: palette.textPrimary),
              ),
              if (status.subtitle != null) ...[
                const SizedBox(height: 2),
                Text(
                  status.subtitle!,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textMuted),
                ),
              ],
            ],
          ),
        ),
        const SizedBox(width: AppSpacing.sm),
        _StatusPill(label: status.label, color: status.dot),
      ],
    );
  }
}

/// 状态药丸：品牌浅底 + 品牌文字。
class _StatusPill extends StatelessWidget {
  const _StatusPill({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 26,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(AppRadius.pill),
      ),
      child: Text(
        label,
        style: AppText.meta.copyWith(
          color: color,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

/// 中断态（离线 / 锁定 / UAC / 唤醒中）：整卡表达 + 主按钮。
class _InterruptPanel extends StatelessWidget {
  const _InterruptPanel({required this.status, this.onPrimary});

  final HomeStatusView status;
  final VoidCallback? onPrimary;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            StatusDot(color: status.dot, glow: true),
            const SizedBox(width: 8),
            Text(
              status.label,
              style: AppText.meta.copyWith(
                color: status.dot,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.md),
        Text(
          status.title,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          style: AppText.cardTitle.copyWith(color: palette.textPrimary),
        ),
        if (status.subtitle != null) ...[
          const SizedBox(height: 4),
          Text(
            status.subtitle!,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: AppText.meta.copyWith(color: palette.textMuted),
          ),
        ],
        if (status.busy) ...[
          const SizedBox(height: AppSpacing.md),
          const AppProgressBar(),
          const SizedBox(height: AppSpacing.sm),
          Text(
            status.progressLabel ?? '',
            style: AppText.meta.copyWith(color: palette.textMuted),
          ),
        ],
        if (status.primaryLabel.isNotEmpty) ...[
          const SizedBox(height: AppSpacing.lg),
          PrimaryButton(
            label: status.primaryLabel,
            icon: status.primaryIcon,
            expand: true,
            busy: status.busy,
            onPressed: status.busy ? null : onPrimary,
          ),
        ],
      ],
    );
  }
}
