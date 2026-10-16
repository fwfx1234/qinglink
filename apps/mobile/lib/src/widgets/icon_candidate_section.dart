/// 「PC 上检测到的图标文件」区块：带缩略图的候选列表。
///
/// 候选来自服务端的 `list_scanner_app_icons`（条目附近的 exe / 快捷方式 / PNG、ICO），
/// 每一项都按路径取一次预览，用户不必凭文件名猜哪张图是对的。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';
import 'app_ui.dart';
import 'icon_preview_thumb.dart';

/// 候选图标列表。
class IconCandidateSection extends StatelessWidget {
  const IconCandidateSection({
    super.key,
    required this.candidates,
    required this.loading,
    required this.busy,
    required this.onUse,
    this.error,
  });

  final List<IconCandidate> candidates;
  final bool loading;

  /// 忙碌时禁用「使用」按钮。
  final bool busy;

  /// 选中某个候选文件的路径。
  final ValueChanged<String> onUse;

  final String? error;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    if (error != null) {
      return Text(error!, style: AppText.meta.copyWith(color: AppColors.red));
    }
    if (!loading && candidates.isEmpty) {
      return Text(
        'PC 上没有找到其他可用的图标文件，可以从别处浏览一张图片。',
        style: AppText.meta.copyWith(color: palette.textSubtle),
      );
    }
    return Column(
      children: [
        for (final candidate in candidates) ...[
          _CandidateCard(
            candidate: candidate,
            onUse: busy ? null : () => onUse(candidate.path),
          ),
          const SizedBox(height: 8),
        ],
      ],
    );
  }
}

/// 单条候选图标文件：缩略图、类型、路径与「使用」按钮。
class _CandidateCard extends StatelessWidget {
  const _CandidateCard({required this.candidate, required this.onUse});

  final IconCandidate candidate;

  /// 为 null 表示正在忙碌，此时禁用「使用」按钮。
  final VoidCallback? onUse;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return AppCard(
      minHeight: 54,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      child: Row(
        children: [
          IconPreviewThumb(
            path: candidate.path,
            size: 40,
            fallbackIcon: Icons.insert_drive_file_outlined,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  candidate.kindLabel,
                  style: AppText.body.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 2),
                Text(
                  candidate.path,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          const SizedBox(width: 6),
          Text(
            _formatBytes(candidate.sizeBytes),
            style: AppText.meta.copyWith(color: palette.textSubtle),
          ),
          const SizedBox(width: 6),
          SecondaryButton(label: '使用', height: 34, onPressed: onUse),
        ],
      ),
    );
  }

  /// 文件体积，帮用户判断"这张图是不是图标"。
  static String _formatBytes(int bytes) {
    if (bytes >= 1024 * 1024) {
      return '${(bytes / 1024 / 1024).toStringAsFixed(1)} MB';
    }
    if (bytes >= 1024) return '${(bytes / 1024).round()} KB';
    return '$bytes B';
  }
}
