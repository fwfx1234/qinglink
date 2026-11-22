/// 检查更新页的展示组件：顶栏、版本卡、进度卡、版本摘要、错误卡。
///
/// 只负责画，不持有状态；动作全部由 `update_screen.dart` 通过回调传入。
library;

import 'package:flutter/material.dart';

import '../providers/update_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';

/// 自绘顶栏：返回 + 标题 + 重新检查，与其它二级页保持一致。
class UpdateTopBar extends StatelessWidget {
  const UpdateTopBar({super.key, required this.busy, required this.onRefresh});

  final bool busy;
  final VoidCallback onRefresh;

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
            onPressed: () => Navigator.of(context).maybePop(),
          ),
          const SizedBox(width: AppSpacing.md),
          Text(
            '检查更新',
            style: AppText.sectionTitle.copyWith(color: palette.textPrimary),
          ),
          const Spacer(),
          IconBtn(
            icon: Icons.refresh_rounded,
            tooltip: '重新检查',
            tonal: true,
            onPressed: busy ? null : onRefresh,
          ),
        ],
      ),
    );
  }
}

/// 版本与状态卡：左侧图标随阶段变色，右侧只放一个结论标签。
class UpdateVersionCard extends StatelessWidget {
  const UpdateVersionCard({
    super.key,
    required this.state,
    required this.palette,
  });

  final UpdateState state;
  final AppPalette palette;

  @override
  Widget build(BuildContext context) {
    final (icon, color, text) = _status();
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(AppMetrics.cardPadding),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TileIcon(
            icon: icon,
            color: color,
            size: 44,
            iconSize: 22,
            radius: AppRadius.md,
          ),
          const SizedBox(width: AppSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '当前版本',
                  style: AppText.meta.copyWith(color: palette.textMuted),
                ),
                const SizedBox(height: 2),
                Text(
                  state.currentVersion.isEmpty ? '未知' : state.currentVersion,
                  style: AppText.sectionTitle.copyWith(
                    color: palette.textPrimary,
                  ),
                ),
                const SizedBox(height: AppSpacing.sm),
                Row(
                  children: [
                    StatusDot(color: color),
                    const SizedBox(width: 6),
                    Flexible(
                      child: Text(
                        text,
                        style: AppText.label.copyWith(color: color),
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          if (state.hasUpdate)
            StatusTag(
              state.phase == UpdatePhase.ready ? '已下载' : '可更新',
              color: state.phase == UpdatePhase.ready
                  ? AppColors.primary
                  : AppColors.warning,
            ),
        ],
      ),
    );
  }

  /// 阶段 → 图标、主色与一句人话。
  (IconData, Color, String) _status() => switch (state.phase) {
        UpdatePhase.idle => (
            Icons.system_update_rounded,
            palette.brand,
            '尚未检查，点下方按钮获取最新版本',
          ),
        UpdatePhase.checking => (
            Icons.sync_rounded,
            AppColors.warning,
            '正在向发布页查询…',
          ),
        UpdatePhase.upToDate => (
            Icons.check_circle_rounded,
            AppColors.success,
            '已经是最新版本',
          ),
        UpdatePhase.available => (
            Icons.new_releases_rounded,
            AppColors.warning,
            '发现新版本 ${state.release?.version ?? ''}',
          ),
        UpdatePhase.downloading => (
            Icons.downloading_rounded,
            AppColors.warning,
            '正在后台下载 ${state.progress}%',
          ),
        UpdatePhase.ready => (
            Icons.verified_rounded,
            AppColors.success,
            '安装包已下载并通过校验',
          ),
        UpdatePhase.error => (
            Icons.error_outline_rounded,
            AppColors.error,
            '本次操作没有完成',
          ),
      };
}

/// 下载进度卡：百分比 + 进度条 + 已下载/总量。
class UpdateProgressCard extends StatelessWidget {
  const UpdateProgressCard({
    super.key,
    required this.state,
    required this.palette,
  });

  final UpdateState state;
  final AppPalette palette;

  @override
  Widget build(BuildContext context) {
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(AppMetrics.cardPadding),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                '下载中',
                style: AppText.cardTitle.copyWith(color: palette.textPrimary),
              ),
              const Spacer(),
              Text(
                '${state.progress}%',
                style: AppText.label.copyWith(color: palette.textMuted),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.sm),
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: LinearProgressIndicator(
              minHeight: 6,
              value: state.totalBytes > 0 ? state.progress / 100.0 : null,
              color: AppColors.primary,
              backgroundColor: palette.textPrimary.withValues(alpha: 0.08),
            ),
          ),
          const SizedBox(height: AppSpacing.sm),
          Text(
            state.totalBytes > 0
                ? '${_mb(state.downloadedBytes)} / ${_mb(state.totalBytes)} MB'
                : '正在连接下载地址…',
            style: AppText.meta.copyWith(color: palette.textSubtle),
          ),
        ],
      ),
    );
  }

  String _mb(int bytes) => (bytes / 1024 / 1024).toStringAsFixed(1);
}

/// 新版本摘要卡：版本号、安装包类型与来源 tag。
class UpdateReleaseCard extends StatelessWidget {
  const UpdateReleaseCard({
    super.key,
    required this.state,
    required this.palette,
  });

  final UpdateState state;
  final AppPalette palette;

  @override
  Widget build(BuildContext context) {
    final release = state.release!;
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.symmetric(horizontal: AppMetrics.cardPadding),
      child: Column(
        children: [
          UpdateInfoRow(
            label: '最新版本',
            value: release.version,
            palette: palette,
          ),
          Divider(height: 1, color: palette.line),
          UpdateInfoRow(
            label: '安装包',
            value: release.hasApk ? 'Android APK' : '请到发布页下载',
            palette: palette,
          ),
          Divider(height: 1, color: palette.line),
          UpdateInfoRow(label: '来源', value: release.tag, palette: palette),
        ],
      ),
    );
  }
}

/// 摘要卡的标签 + 值一行。
class UpdateInfoRow extends StatelessWidget {
  const UpdateInfoRow({
    super.key,
    required this.label,
    required this.value,
    required this.palette,
  });

  final String label;
  final String value;
  final AppPalette palette;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 13),
      child: Row(
        children: [
          Text(label, style: AppText.body.copyWith(color: palette.textMuted)),
          const Spacer(),
          const SizedBox(width: AppSpacing.md),
          Flexible(
            child: Text(
              value,
              textAlign: TextAlign.right,
              style: AppText.body.copyWith(color: palette.textPrimary),
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

/// 错误提示卡：浅红底 + 描边，替掉裸红字的写法。
class UpdateErrorCard extends StatelessWidget {
  const UpdateErrorCard({
    super.key,
    required this.message,
    required this.palette,
  });

  final String message;
  final AppPalette palette;

  @override
  Widget build(BuildContext context) {
    return AppCard(
      radius: AppRadius.lg,
      padding: const EdgeInsets.all(AppMetrics.cardPadding),
      color: palette.dangerTint,
      borderColor: palette.danger.withValues(alpha: 0.28),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.error_outline_rounded, size: 18, color: AppColors.error),
          const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Text(
              message,
              style: AppText.label.copyWith(color: AppColors.error),
            ),
          ),
        ],
      ),
    );
  }
}
