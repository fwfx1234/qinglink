import 'package:flutter/material.dart';

import '../../models/display_info.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import '../../widgets/app_sheet.dart';

/// 分辨率与刷新率选择弹层。
///
/// 按分辨率分组，同一分辨率下的每个刷新率都是一个可点的胶囊：服务端返回的是 1080p 及以上
/// 的可选模式（同一分辨率最多 4 个刷新率），当前模式不在列表里时也会附在最后。
class DisplayModeSheet extends StatelessWidget {
  const DisplayModeSheet({super.key, required this.display});

  final DisplayInfo display;

  /// 打开弹层并返回用户选中的模式；取消返回 `null`。
  static Future<DisplayModeInfo?> show(
    BuildContext context,
    DisplayInfo display,
  ) {
    return showAppSheet<DisplayModeInfo>(
      context,
      isScrollControlled: true,
      builder: (_) => DisplayModeSheet(display: display),
    );
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final groups = _groupByResolution(display.modes);
    return SafeArea(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: MediaQuery.sizeOf(context).height * 0.7,
        ),
        child: ListView(
          shrinkWrap: true,
          padding: const EdgeInsets.fromLTRB(
            AppSpacing.xl,
            0,
            AppSpacing.xl,
            AppSpacing.lg,
          ),
          children: [
            SheetHeader(
              title: '分辨率与刷新率',
              description:
                  '${display.name} · 共 ${display.modes.length} 种模式',
            ),
            if (groups.isEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 20),
                child: Text(
                  '这台显示器没有返回可用模式，请先刷新显示器列表',
                  style: AppText.body.copyWith(color: palette.textMuted),
                ),
              ),
            for (final group in groups) ...[
              Padding(
                padding: const EdgeInsets.only(top: 12, bottom: 6),
                child: Row(
                  children: [
                    Text(
                      group.resolution,
                      style: AppText.body.copyWith(
                        fontWeight: FontWeight.w700,
                        color: palette.textPrimary,
                      ),
                    ),
                    const SizedBox(width: 8),
                    if (display.isCurrent(group.modes.first))
                      const StatusTag('当前'),
                  ],
                ),
              ),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  for (final mode in group.modes)
                    _RefreshChip(
                      mode: mode,
                      selected: display.isCurrent(mode),
                      onTap: () => Navigator.of(context).pop(mode),
                    ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// 按分辨率聚合，保持服务端给出的顺序（面积降序、刷新率降序）。
  List<_ModeGroup> _groupByResolution(List<DisplayModeInfo> modes) {
    final groups = <String, List<DisplayModeInfo>>{};
    for (final mode in modes) {
      groups.putIfAbsent(mode.resolution, () => <DisplayModeInfo>[]).add(mode);
    }
    return [
      for (final entry in groups.entries)
        _ModeGroup(resolution: entry.key, modes: entry.value),
    ];
  }
}

class _ModeGroup {
  const _ModeGroup({required this.resolution, required this.modes});

  final String resolution;
  final List<DisplayModeInfo> modes;
}

class _RefreshChip extends StatelessWidget {
  const _RefreshChip({
    required this.mode,
    required this.selected,
    required this.onTap,
  });

  final DisplayModeInfo mode;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final label = mode.refreshText.isEmpty ? mode.resolution : mode.refreshText;
    return ChoiceChip(
      label: Text(label),
      selected: selected,
      onSelected: (_) => onTap(),
      showCheckmark: false,
      selectedColor: AppColors.primary,
      labelStyle: AppText.label.copyWith(
        color: selected ? AppColors.onPrimary : null,
      ),
    );
  }
}
