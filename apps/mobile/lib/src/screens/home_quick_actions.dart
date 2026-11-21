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

/// 快捷操作网格：4 列图标瓦片，每个瓦片直接调用对应入口。
class HomeQuickActionsSection extends ConsumerWidget {
  const HomeQuickActionsSection({
    super.key,
    required this.online,
    required this.ready,
    required this.irReady,
    required this.shortcuts,
    required this.onUnlock,
    required this.onTvPower,
    required this.onShortcut,
  });

  final bool online;
  final bool ready;
  final bool irReady;
  final List<String> shortcuts;
  final VoidCallback onUnlock;
  final VoidCallback onTvPower;
  final ValueChanged<String> onShortcut;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return _QuickGrid(
      items: [
        ..._systemTiles(context, ref),
        if (irReady) ..._tvTiles(context),
        ...shortcuts.map(
          (label) => _QuickTile(
            icon: Icons.keyboard_command_key_rounded,
            label: label,
            tone: context.palette.violet,
            enabled: ready,
            onTap: () => onShortcut(label),
          ),
        ),
        _pointerTile(context),
      ],
    );
  }

  /// 系统动作：解锁、音量、睡眠、关机。
  List<_QuickTile> _systemTiles(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    return [
      _QuickTile(
        icon: Icons.lock_open_rounded,
        label: '解锁',
        tone: palette.warn,
        // Keep the entry visible while preventing a remote
        // command from being sent before the PC session exists.
        enabled: online,
        onTap: onUnlock,
      ),
      _QuickTile(
        icon: Icons.volume_up_rounded,
        label: '音量',
        tone: palette.info,
        enabled: ready,
        onTap: () => showVolumeSheet(context, ref),
      ),
      _QuickTile(
        icon: Icons.bedtime_rounded,
        label: '睡眠',
        tone: palette.violet,
        enabled: ready,
        onTap: () => confirmPowerAction(context, ref, 'sleep'),
      ),
      _QuickTile(
        icon: Icons.power_settings_new_rounded,
        label: '关机',
        tone: palette.danger,
        enabled: ready,
        onTap: () => confirmPowerAction(context, ref, 'shutdown'),
      ),
      _QuickTile(
        icon: Icons.dynamic_feed_rounded,
        label: '进程',
        tone: palette.info,
        enabled: ready,
        onTap: () => context.push(AppRoutes.processes),
      ),
    ];
  }

  /// 电视动作：红外电源与遥控器面板（仅在红外可用时出现）。
  List<_QuickTile> _tvTiles(BuildContext context) {
    final palette = context.palette;
    return [
      _QuickTile(
        icon: Icons.tv_rounded,
        label: '电视',
        tone: palette.info,
        enabled: true,
        onTap: onTvPower,
      ),
      _QuickTile(
        icon: Icons.settings_remote_rounded,
        label: '遥控',
        tone: palette.warn,
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
      tone: context.palette.violet,
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
  const _QuickGrid({required this.items});
  final List<_QuickTile> items;

  @override
  Widget build(BuildContext context) {
    return GridView.builder(
      shrinkWrap: true,
      padding: EdgeInsets.zero,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: items.length,
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 4,
        mainAxisSpacing: 6,
        crossAxisSpacing: 10,
        // 44 图标 + 8 间隙 + 16 文字行高 = 68，行距合计 76（设计稿 74）。
        mainAxisExtent: 70,
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
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: item.enabled ? item.onTap : null,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 44,
                height: 44,
                decoration: BoxDecoration(
                  color: item.tone.withValues(alpha: 0.14),
                  borderRadius: BorderRadius.circular(AppRadius.md),
                ),
                child: Icon(item.icon, size: 22, color: item.tone),
              ),
              const SizedBox(height: 8),
              Text(
                item.label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: AppText.meta.copyWith(color: palette.textMuted),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
