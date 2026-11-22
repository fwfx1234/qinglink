/// 检查更新页：版本对照、下载进度与安装入口。
///
/// Android 支持后台下载、SHA256 校验与拉起系统安装器；iOS 无法侧载，只提示
/// 前往发布页。版面沿用其它二级页的写法（自绘顶栏 + ListView + 卡片分组），
/// 不用 Material AppBar，避免与全站风格不一致。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';

import '../providers/update_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';

class UpdateScreen extends ConsumerWidget {
  const UpdateScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final async = ref.watch(updateProvider);
    final state = async.valueOrNull;

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
            _TopBar(
              busy: state?.busy ?? false,
              onRefresh: () => ref.read(updateProvider.notifier).check(),
            ),
            const SizedBox(height: AppSpacing.md),
            if (async.hasError && state == null)
              _ErrorCard(
                message: '读取版本信息失败：${async.error}',
                palette: palette,
              )
            else if (state == null)
              const Padding(
                padding: EdgeInsets.only(top: AppSpacing.xxxl),
                child: Center(child: CircularProgressIndicator()),
              )
            else
              ..._content(context, ref, state, palette),
          ],
        ),
      ),
    );
  }

  List<Widget> _content(
    BuildContext context,
    WidgetRef ref,
    UpdateState state,
    AppPalette palette,
  ) {
    final release = state.release;
    final downloaded = state.phase == UpdatePhase.downloading ||
        state.phase == UpdatePhase.ready;

    return [
      _VersionCard(state: state, palette: palette),
      if (state.phase == UpdatePhase.downloading) ...[
        const SizedBox(height: AppSpacing.md),
        _ProgressCard(state: state, palette: palette),
      ],
      if (state.error != null) ...[
        const SizedBox(height: AppSpacing.md),
        _ErrorCard(message: state.error!, palette: palette),
      ],
      if (release != null) ...[
        const SizedBox(height: AppSpacing.lg),
        const SectionHeading(title: '新版本'),
        _ReleaseCard(state: state, palette: palette),
        const SizedBox(height: AppSpacing.lg),
        const SectionHeading(title: '更新说明'),
        AppCard(
          radius: AppRadius.lg,
          padding: const EdgeInsets.all(AppMetrics.cardPadding),
          child: Text(
            release.notes.trim().isEmpty ? '这个版本没有填写更新说明。' : release.notes.trim(),
            style: AppText.body.copyWith(color: palette.textMuted),
          ),
        ),
      ],
      const SizedBox(height: AppSpacing.xl),
      _primaryAction(context, ref, state),
      if (release != null && release.htmlUrl.isNotEmpty) ...[
        const SizedBox(height: AppSpacing.md),
        SecondaryButton(
          label: downloaded ? '在发布页查看' : '前往发布页',
          icon: Icons.open_in_new_rounded,
          expand: true,
          onPressed: () => _openUrl(release.htmlUrl, context),
        ),
      ],
      const SizedBox(height: AppSpacing.lg),
      Text(
        '更新包来自本项目的 GitHub Release。下载完成后会先核对校验清单，'
        '哈希不一致的安装包不会进入安装环节。',
        style: AppText.meta.copyWith(color: palette.textSubtle),
      ),
    ];
  }

  /// 主按钮随阶段变化：检查 → 下载 → 安装，避免页面上同时挂三个动作按钮。
  Widget _primaryAction(
    BuildContext context,
    WidgetRef ref,
    UpdateState state,
  ) {
    final notifier = ref.read(updateProvider.notifier);
    final canInstallApk = state.canInstall;

    return switch (state.phase) {
      UpdatePhase.checking => const PrimaryButton(
          label: '正在检查…',
          busy: true,
          expand: true,
        ),
      UpdatePhase.downloading => PrimaryButton(
          label: '正在下载 ${state.progress}%',
          icon: Icons.downloading_rounded,
          expand: true,
        ),
      UpdatePhase.ready => PrimaryButton(
          label: '安装更新',
          icon: Icons.install_mobile_rounded,
          expand: true,
          onPressed: () => _install(context, ref),
        ),
      UpdatePhase.available when canInstallApk => PrimaryButton(
          label: '下载更新',
          icon: Icons.download_rounded,
          expand: true,
          onPressed: notifier.download,
        ),
      UpdatePhase.available => PrimaryButton(
          label: '前往发布页',
          icon: Icons.open_in_new_rounded,
          expand: true,
          onPressed: () {
            final url = state.release?.htmlUrl ?? '';
            if (url.isNotEmpty) _openUrl(url, context);
          },
        ),
      _ => PrimaryButton(
          label: '检查更新',
          icon: Icons.refresh_rounded,
          expand: true,
          onPressed: notifier.check,
        ),
    };
  }

  Future<void> _install(BuildContext context, WidgetRef ref) async {
    final outcome = await ref.read(updateProvider.notifier).install();
    if (!context.mounted) return;
    switch (outcome) {
      case InstallOutcome.permissionDenied:
        context.showWarningSnackBar('请先允许「安装未知来源应用」');
      case InstallOutcome.failed:
        context.showErrorSnackBar('无法拉起系统安装器');
      case InstallOutcome.unsupported:
        context.showInfoSnackBar('当前平台不支持应用内安装，请前往发布页');
      case InstallOutcome.launched:
        break;
    }
  }

  Future<void> _openUrl(String url, BuildContext context) async {
    if (url.isEmpty) return;
    final uri = Uri.tryParse(url);
    if (uri == null ||
        !(await canLaunchUrl(uri)) ||
        !(await launchUrl(uri, mode: LaunchMode.externalApplication))) {
      if (context.mounted) context.showErrorSnackBar('无法打开发布页');
    }
  }
}

/// 自绘顶栏：返回 + 标题 + 重新检查。
class _TopBar extends StatelessWidget {
  const _TopBar({required this.busy, required this.onRefresh});

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
class _VersionCard extends StatelessWidget {
  const _VersionCard({required this.state, required this.palette});

  final UpdateState state;
  final AppPalette palette;

  @override
  Widget build(BuildContext context) {
    final (icon, color, text) = _status(state);
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
  (IconData, Color, String) _status(UpdateState state) => switch (state.phase) {
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
class _ProgressCard extends StatelessWidget {
  const _ProgressCard({required this.state, required this.palette});

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

/// 新版本摘要卡：版本号与可执行的动作提示。
class _ReleaseCard extends StatelessWidget {
  const _ReleaseCard({required this.state, required this.palette});

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
          _Row(label: '最新版本', value: release.version, palette: palette),
          Divider(height: 1, color: palette.line),
          _Row(
            label: '安装包',
            value: release.hasApk ? 'Android APK' : '请到发布页下载',
            palette: palette,
          ),
          Divider(height: 1, color: palette.line),
          _Row(
            label: '来源',
            value: release.tag,
            palette: palette,
          ),
        ],
      ),
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
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

/// 错误提示卡：浅红底 + 描边，替掉原来裸红字的写法。
class _ErrorCard extends StatelessWidget {
  const _ErrorCard({required this.message, required this.palette});

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
