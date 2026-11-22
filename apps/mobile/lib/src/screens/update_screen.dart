/// 检查更新页：版本对照、下载进度与安装入口。
///
/// Android 支持后台下载、SHA256 校验与拉起系统安装器；iOS 无法侧载，只提示
/// 前往发布页。版面沿用其它二级页的写法（自绘顶栏 + ListView + 卡片分组），
/// 不用 Material AppBar，避免与全站风格不一致；卡片组件在 [update_widgets.dart]。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';

import '../providers/update_provider.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/app_ui.dart';
import 'update_widgets.dart';

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
            UpdateTopBar(
              busy: state?.busy ?? false,
              onRefresh: () => ref.read(updateProvider.notifier).check(),
            ),
            const SizedBox(height: AppSpacing.md),
            if (async.hasError && state == null)
              UpdateErrorCard(
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
      UpdateVersionCard(state: state, palette: palette),
      if (state.phase == UpdatePhase.downloading) ...[
        const SizedBox(height: AppSpacing.md),
        UpdateProgressCard(state: state, palette: palette),
      ],
      if (state.error != null) ...[
        const SizedBox(height: AppSpacing.md),
        UpdateErrorCard(message: state.error!, palette: palette),
      ],
      if (release != null) ...[
        const SizedBox(height: AppSpacing.lg),
        const SectionHeading(title: '新版本'),
        UpdateReleaseCard(state: state, palette: palette),
        const SizedBox(height: AppSpacing.lg),
        const SectionHeading(title: '更新说明'),
        AppCard(
          radius: AppRadius.lg,
          padding: const EdgeInsets.all(AppMetrics.cardPadding),
          child: Text(
            release.notes.trim().isEmpty
                ? '这个版本没有填写更新说明。'
                : release.notes.trim(),
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
      UpdatePhase.available when state.canInstall => PrimaryButton(
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
