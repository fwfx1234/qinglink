/// 检查更新页：显示当前/最新版本、更新说明、下载进度与安装入口。
///
/// iOS 没有可侧载的 APK：检测到更新后只提供「前往发布页」按钮；Android 则完整支持
/// 后台下载、SHA256 校验与拉起系统安装器。
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

    return Scaffold(
      backgroundColor: palette.background,
      appBar: AppBar(
        title: const Text('检查更新'),
        backgroundColor: palette.background,
        foregroundColor: palette.textPrimary,
        elevation: 0,
      ),
      body: async.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(AppSpacing.lg),
            child: Text(
              '加载失败：$error',
              style: AppText.body.copyWith(color: palette.textMuted),
              textAlign: TextAlign.center,
            ),
          ),
        ),
        data: (state) => _body(context, ref, state, palette),
      ),
    );
  }

  Widget _body(
    BuildContext context,
    WidgetRef ref,
    UpdateState state,
    AppPalette palette,
  ) {
    return ListView(
      padding: const EdgeInsets.fromLTRB(
        AppMetrics.pagePadding,
        AppSpacing.lg,
        AppMetrics.pagePadding,
        AppSpacing.xxl,
      ),
      children: [
        AppCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _infoRow('当前版本', state.currentVersion, palette),
              const SizedBox(height: AppSpacing.sm),
              _infoRow(
                '最新版本',
                state.release?.version ?? '—',
                palette,
              ),
              const SizedBox(height: AppSpacing.md),
              _phaseHint(state, palette),
            ],
          ),
        ),
        const SizedBox(height: AppSpacing.lg),
        SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: state.busy
                ? null
                : () => ref.read(updateProvider.notifier).check(),
            icon: const Icon(Icons.refresh_rounded),
            label: Text(state.busy ? '请稍候…' : '检查更新'),
          ),
        ),
        if (state.phase == UpdatePhase.downloading) ...[
          const SizedBox(height: AppSpacing.lg),
          LinearProgressIndicator(value: state.progress / 100.0),
          const SizedBox(height: AppSpacing.sm),
          Text(
            state.totalBytes > 0
                ? '${state.progress}%  ·  ${_mb(state.downloadedBytes)} / ${_mb(state.totalBytes)} MB'
                : '正在下载…',
            style: AppText.label.copyWith(color: palette.textMuted),
          ),
        ],
        if (state.error != null) ...[
          const SizedBox(height: AppSpacing.md),
          Text(
            state.error!,
            style: AppText.label.copyWith(color: AppColors.error),
          ),
        ],
        if (state.release != null) ..._releaseActions(context, ref, state),
      ],
    );
  }

  List<Widget> _releaseActions(
    BuildContext context,
    WidgetRef ref,
    UpdateState state,
  ) {
    final release = state.release!;
    final buttons = <Widget>[
      if (state.phase == UpdatePhase.available && release.hasApk)
        FilledButton.icon(
          onPressed: () => ref.read(updateProvider.notifier).download(),
          icon: const Icon(Icons.download_rounded),
          label: const Text('下载更新'),
        ),
      if (state.phase == UpdatePhase.ready)
        FilledButton.icon(
          onPressed: () async {
            final outcome =
                await ref.read(updateProvider.notifier).install();
            if (!context.mounted) return;
            switch (outcome) {
              case InstallOutcome.permissionDenied:
                context.showWarningSnackBar('请允许安装未知来源应用');
              case InstallOutcome.failed:
                context.showErrorSnackBar('无法拉起安装器');
              case InstallOutcome.unsupported:
                context.showInfoSnackBar('请前往发布页手动安装');
              case InstallOutcome.launched:
                break;
            }
          },
          icon: const Icon(Icons.install_mobile_rounded),
          label: const Text('安装更新'),
        ),
      OutlinedButton.icon(
        onPressed: () => _openUrl(release.htmlUrl, context),
        icon: const Icon(Icons.open_in_new_rounded),
        label: const Text('前往发布页'),
      ),
    ];

    return [
      const SizedBox(height: AppSpacing.xl),
      Text('更新说明', style: AppText.sectionTitle),
      const SizedBox(height: AppSpacing.sm),
      AppCard(
        child: Text(
          release.notes.trim().isEmpty ? '暂无更新说明' : release.notes.trim(),
          style: AppText.body.copyWith(color: context.palette.textMuted),
        ),
      ),
      const SizedBox(height: AppSpacing.lg),
      ...buttons.map(
        (button) => Padding(
          padding: const EdgeInsets.only(bottom: AppSpacing.sm),
          child: SizedBox(width: double.infinity, child: button),
        ),
      ),
    ];
  }

  Widget _infoRow(String label, String value, AppPalette palette) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: AppText.body),
        Text(
          value,
          style: AppText.body.copyWith(color: palette.textMuted),
        ),
      ],
    );
  }

  Widget _phaseHint(UpdateState state, AppPalette palette) {
    final (text, color) = switch (state.phase) {
      UpdatePhase.idle => ('尚未检查更新', palette.textMuted),
      UpdatePhase.checking => ('正在检查…', AppColors.warning),
      UpdatePhase.upToDate => ('已是最新版本', AppColors.success),
      UpdatePhase.available => ('发现新版本，可下载', AppColors.warning),
      UpdatePhase.downloading => ('正在后台下载', AppColors.warning),
      UpdatePhase.ready => ('已就绪，可安装', AppColors.success),
      UpdatePhase.error => ('检查或下载失败', AppColors.error),
    };
    return Row(
      children: [
        Icon(Icons.circle, size: 8, color: color),
        const SizedBox(width: AppSpacing.sm),
        Text(text, style: AppText.label.copyWith(color: color)),
      ],
    );
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

  String _mb(int bytes) => (bytes / 1024 / 1024).toStringAsFixed(1);
}
