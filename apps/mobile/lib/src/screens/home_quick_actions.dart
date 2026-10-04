/// 首页「快捷操作」区：系统动作、电视动作与指针控制入口。
///
/// 网格与瓦片样式属于本库内部实现；页面装配在 `home_tab.dart`。
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../app/routes.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../utils/remote_actions.dart';
import '../widgets/app_ui.dart';
import 'ir/tv_quick_panel.dart';

/// 快捷操作网格：按红外可用性决定列数，每个瓦片直接调用对应入口。
class HomeQuickActionsSection extends ConsumerWidget {
  const HomeQuickActionsSection({
    super.key,
    required this.online,
    required this.ready,
    required this.irReady,
    required this.onUnlock,
    required this.onTvPower,
  });

  final bool online;
  final bool ready;
  final bool irReady;
  final VoidCallback onUnlock;
  final VoidCallback onTvPower;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return _QuickGrid(
      crossAxisCount: irReady ? 4 : 3,
      items: [
        ..._systemTiles(context, ref),
        if (irReady) ..._tvTiles(context),
        _pointerTile(context),
      ],
    );
  }

  /// 系统动作：解锁、进程、休眠、关机、音量。
  List<_QuickTile> _systemTiles(BuildContext context, WidgetRef ref) {
    return [
      _QuickTile(
        icon: Icons.lock_open_rounded,
        label: '解锁',
        tone: AppColors.amber,
        // Keep the entry visible while preventing a remote
        // command from being sent before the PC session exists.
        enabled: online,
        onTap: onUnlock,
      ),
      _QuickTile(
        icon: Icons.dynamic_feed_rounded,
        label: '进程',
        tone: AppColors.violet,
        enabled: ready,
        onTap: () => context.push(AppRoutes.processes),
      ),
      _QuickTile(
        icon: Icons.bedtime_rounded,
        label: '休眠',
        tone: AppColors.slate,
        enabled: ready,
        onTap: () => confirmPowerAction(context, ref, 'sleep'),
      ),
      _QuickTile(
        icon: Icons.power_settings_new_rounded,
        label: '关机',
        tone: AppColors.red,
        enabled: ready,
        onTap: () => confirmPowerAction(context, ref, 'shutdown'),
      ),
      _QuickTile(
        icon: Icons.volume_up_rounded,
        label: '音量',
        tone: AppColors.amber,
        enabled: ready,
        onTap: () => showVolumeSheet(context, ref),
      ),
    ];
  }

  /// 电视动作：红外电源与遥控器面板（仅在红外可用时出现）。
  List<_QuickTile> _tvTiles(BuildContext context) {
    return [
      _QuickTile(
        icon: Icons.power_settings_new_rounded,
        label: '电源',
        tone: AppColors.slate,
        enabled: true,
        onTap: onTvPower,
      ),
      _QuickTile(
        icon: Icons.settings_remote_rounded,
        label: 'TV',
        tone: AppColors.cyan,
        enabled: true,
        onTap: () => showTvQuickPanel(context),
      ),
    ];
  }

  /// 指针控制入口。
  _QuickTile _pointerTile(BuildContext context) {
    return _QuickTile(
      icon: Icons.mouse_rounded,
      label: '控制',
      tone: AppColors.cyan,
      enabled: ready,
      onTap: () => context.push(AppRoutes.mouse),
    );
  }
}

/// 单个快捷操作瓦片的纯数据。
class _QuickTile {
  const _QuickTile({
    required this.icon,
    required this.label,
    required this.tone,
    required this.enabled,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final Color tone;
  final bool enabled;
  final VoidCallback onTap;
}

class _QuickGrid extends StatelessWidget {
  const _QuickGrid({required this.items, this.crossAxisCount = 3});
  final List<_QuickTile> items;
  final int crossAxisCount;

  @override
  Widget build(BuildContext context) {
    return GridView.builder(
      shrinkWrap: true,
      padding: EdgeInsets.zero,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: items.length,
      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: crossAxisCount,
        mainAxisSpacing: 8,
        crossAxisSpacing: 8,
        mainAxisExtent: 48,
      ),
      itemBuilder: (context, index) => _QuickTileView(item: items[index]),
    );
  }
}

class _QuickTileView extends StatelessWidget {
  const _QuickTileView({required this.item});
  final _QuickTile item;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Opacity(
      opacity: item.enabled ? 1 : 0.4,
      child: PressableScale(
        enabled: item.enabled,
        child: AppCard(
          radius: AppRadius.md,
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 8),
          onTap: item.enabled ? item.onTap : null,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              TileIcon(
                icon: item.icon,
                color: item.tone,
                size: 26,
                radius: 8,
                iconSize: 15,
                alpha: 0.16,
              ),
              const SizedBox(width: 7),
              Flexible(
                child: Text(
                  item.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.label.copyWith(
                    fontWeight: FontWeight.w700,
                    color: palette.textPrimary,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
