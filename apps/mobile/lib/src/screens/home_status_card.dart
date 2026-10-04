/// 首页状态卡：指示灯、标题、进度提示与主按钮。
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
        borderRadius: BorderRadius.circular(AppRadius.lg),
        boxShadow: [
          BoxShadow(
            color: palette.shadow,
            blurRadius: 24,
            offset: const Offset(0, 12),
            spreadRadius: -14,
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(AppRadius.lg),
        child: Stack(
          children: [
            _StatusCardHalo(
              right: -46,
              top: -34,
              size: 156,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                border: Border.all(
                  color: AppColors.primary.withValues(alpha: 0.13),
                ),
              ),
            ),
            _StatusCardHalo(
              right: 34,
              bottom: -64,
              size: 132,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: AppColors.primary.withValues(alpha: 0.05),
              ),
            ),
            _StatusCardPanel(status: status, onPrimary: onPrimary),
          ],
        ),
      ),
    );
  }
}

/// 状态卡角落的装饰圆环或色块，由 [Positioned] 定位在卡片边缘之外。
class _StatusCardHalo extends StatelessWidget {
  const _StatusCardHalo({
    required this.size,
    required this.decoration,
    this.right,
    this.top,
    this.bottom,
  });

  final double size;
  final BoxDecoration decoration;
  final double? right;
  final double? top;
  final double? bottom;

  @override
  Widget build(BuildContext context) {
    return Positioned(
      right: right,
      top: top,
      bottom: bottom,
      child: Container(width: size, height: size, decoration: decoration),
    );
  }
}

/// 状态卡的渐变面板：状态行、标题、可选的进度提示与主按钮。
class _StatusCardPanel extends StatelessWidget {
  const _StatusCardPanel({required this.status, this.onPrimary});

  final HomeStatusView status;
  final VoidCallback? onPrimary;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      padding: const EdgeInsets.all(16),
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
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _StatusCardHeader(status: status),
          const SizedBox(height: 10),
          Text(
            status.title,
            style: AppText.cardTitle.copyWith(
              fontSize: 15,
              color: palette.statusTitle,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          if (status.busy) ...[
            const SizedBox(height: 12),
            const AppProgressBar(),
            const SizedBox(height: 8),
            Text(
              status.progressLabel ?? '',
              style: AppText.meta.copyWith(
                color: AppColors.progressText,
              ),
            ),
          ],
          const SizedBox(height: 12),
          PrimaryButton(
            label: status.primaryLabel,
            icon: status.primaryIcon,
            height: 42,
            expand: true,
            busy: status.busy,
            onPressed: status.busy ? null : onPrimary,
          ),
        ],
      ),
    );
  }
}

/// 状态行：指示灯 + 状态文案。
class _StatusCardHeader extends StatelessWidget {
  const _StatusCardHeader({required this.status});

  final HomeStatusView status;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        StatusDot(color: status.dot, glow: true),
        const SizedBox(width: 7),
        Text(
          status.label,
          style: AppText.label.copyWith(
            color: status.dot,
            fontWeight: FontWeight.w600,
          ),
        ),
      ],
    );
  }
}
