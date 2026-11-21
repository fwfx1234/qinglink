import 'package:flutter/material.dart';

import '../../models/display_info.dart';
import '../../theme/app_theme.dart';
import '../../widgets/app_ui.dart';
import '../mouse_keyboard/connection_status_bar.dart';

/// 控制中心的「显示器」分组。
///
/// 支持多显示器：每台显示器一行，点一下切换成当前操作对象；下面是这台显示器的
/// 分辨率入口。加载、空列表、失败三种状态都有明确呈现与重试入口——旧实现读失败时
/// 直接静默，页面会永久停在"未检测到显示器"。
class DisplaySection extends StatelessWidget {
  const DisplaySection({
    super.key,
    required this.displays,
    required this.activeDisplay,
    required this.loading,
    required this.error,
    required this.onRetry,
    required this.onSelect,
    required this.onPickMode,
  });

  final List<DisplayInfo> displays;

  /// 当前选中（或主）显示器。
  final DisplayInfo? activeDisplay;

  final bool loading;
  final String? error;
  final VoidCallback onRetry;
  final ValueChanged<String> onSelect;
  final VoidCallback onPickMode;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final active = activeDisplay;
    return SettingsGroup(
      title: '显示器',
      trailing: loading
          ? const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : IconBtn(
              icon: Icons.refresh_rounded,
              tooltip: '重新读取显示器',
              onPressed: onRetry,
            ),
      children: [
        if (error != null) ...[
          ErrorTextLine(message: error!),
          const SizedBox(height: 4),
        ],
        if (displays.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 6),
            child: Row(
              children: [
                TileIcon(
                  icon: Icons.monitor_rounded,
                  color: AppColors.cyan,
                  size: 32,
                  radius: 9,
                  iconSize: 17,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        loading ? '正在读取显示器…' : '未检测到显示器',
                        style: AppText.body.copyWith(
                          fontWeight: FontWeight.w700,
                          color: palette.textPrimary,
                        ),
                      ),
                      Text(
                        loading ? '请稍候' : '确认 PC 已连接显示器，然后点右上角重试',
                        style: AppText.meta.copyWith(color: palette.textSubtle),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          )
        else
          for (final display in displays)
            _DisplayRow(
              display: display,
              selected: display.id == active?.id,
              selectable: displays.length > 1,
              onTap: () => onSelect(display.id),
            ),
        const SizedBox(height: 6),
        AppListRow(
          icon: Icons.aspect_ratio_rounded,
          title: '分辨率与刷新率',
          subtitle: active == null
              ? '先读取显示器'
              : '${active.currentMode.resolution} ${active.currentMode.refreshText}'
                    .trim(),
          showChevron: true,
          onTap: active == null ? null : onPickMode,
        ),
      ],
    );
  }
}

/// 单台显示器一行：名称、当前模式、主显示器标记、选中态。
class _DisplayRow extends StatelessWidget {
  const _DisplayRow({
    required this.display,
    required this.selected,
    required this.selectable,
    required this.onTap,
  });

  final DisplayInfo display;
  final bool selected;
  final bool selectable;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final mode = display.currentMode;
    final current = '${mode.resolution} ${mode.refreshText}'.trim();
    return InkWell(
      borderRadius: BorderRadius.circular(10),
      onTap: selectable ? onTap : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Row(
          children: [
            TileIcon(
              icon: Icons.monitor_rounded,
              color: selected ? AppColors.primary : AppColors.cyan,
              size: 32,
              radius: 9,
              iconSize: 17,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    display.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AppText.body.copyWith(
                      fontWeight: FontWeight.w700,
                      color: palette.textPrimary,
                    ),
                  ),
                  // 「主显示器」标记放到第二行：显示器型号往往很长，
                  // 和标记抢同一行会把型号截成「NVIDIA GeForce RTX 3070…」。
                  Row(
                    children: [
                      Flexible(
                        child: Text(
                          current.isEmpty ? display.id : current,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: AppText.meta.copyWith(
                            color: palette.textSubtle,
                          ),
                        ),
                      ),
                      if (display.primary) ...[
                        const SizedBox(width: AppSpacing.sm),
                        const StatusTag('主显示器'),
                      ],
                    ],
                  ),
                ],
              ),
            ),
            if (selectable && selected) ...[
              const SizedBox(width: 6),
              Icon(
                Icons.check_circle_rounded,
                size: 18,
                color: palette.brand,
              ),
            ],
          ],
        ),
      ),
    );
  }
}
