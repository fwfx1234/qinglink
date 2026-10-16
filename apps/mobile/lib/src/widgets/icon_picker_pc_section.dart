/// 「浏览 PC 上的图片」区块。
///
/// 手机看不到 PC 的文件系统，换自定义图标只能让用户浏览 PC 上的图片文件；这里负责
/// 入口、选中图片的预览，以及"使用 / 重选"两个动作，浏览器与落库由宿主完成。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import 'app_ui.dart';
import 'icon_preview_thumb.dart';

/// PC 图片选择区块。
class IconPickerPcSection extends StatelessWidget {
  const IconPickerPcSection({
    super.key,
    required this.pickedPath,
    required this.busy,
    required this.onBrowse,
    required this.onApply,
  });

  /// 已选中但还没应用的 PC 文件路径；为空时只显示入口按钮。
  final String? pickedPath;

  /// 宿主的忙碌标志：写入期间禁用按钮。
  final bool busy;

  /// 打开 PC 文件浏览器（重选也走它）。
  final VoidCallback onBrowse;

  /// 用选中的图片文件作为图标。
  final VoidCallback onApply;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final path = pickedPath;
    if (path == null) {
      return SecondaryButton(
        label: '浏览 PC 上的图片',
        icon: Icons.folder_open_rounded,
        expand: true,
        onPressed: busy ? null : onBrowse,
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppCard(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              IconPreviewThumb(path: path, size: 56, radius: 12),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      fileNameOf(path),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.cardTitle.copyWith(
                        color: palette.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      path,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.meta.copyWith(color: palette.textSubtle),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(
              child: SecondaryButton(
                label: '重新选择',
                icon: Icons.refresh_rounded,
                onPressed: busy ? null : onBrowse,
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: PrimaryButton(
                label: '使用这张图片',
                icon: Icons.check_rounded,
                busy: busy,
                onPressed: busy ? null : onApply,
              ),
            ),
          ],
        ),
      ],
    );
  }
}

/// 取路径最后一段作为文件名。
String fileNameOf(String path) {
  final parts = path.split(RegExp(r'[\\/]'));
  return parts.isEmpty ? path : parts.last;
}
