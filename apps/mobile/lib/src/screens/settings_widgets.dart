/// 设置页的通用积木：顶栏、分组标题、电源宫格、设置行、分隔线与只读键值行。
///
/// 这些部件与具体设置项无关，抽出来既让 `settings_tab.dart` 只负责装配，
/// 也让「输入」分组（`settings_input_section.dart`）能复用同一套行样式。
library;

import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../widgets/app_ui.dart';

/// 顶栏：返回键 + 左对齐大标题。
class SettingsNavBar extends StatelessWidget {
  const SettingsNavBar({super.key, required this.onBack});

  final VoidCallback onBack;

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
            onPressed: onBack,
          ),
          const SizedBox(width: AppSpacing.md),
          Text(
            '设置',
            style: AppText.pageTitle.copyWith(color: palette.textPrimary),
          ),
        ],
      ),
    );
  }
}

/// 分组小标题（12 号次级文字）。
class SettingsSectionLabel extends StatelessWidget {
  const SettingsSectionLabel(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: AppText.label.copyWith(color: context.palette.textMuted),
    );
  }
}

/// 电源四宫格：睡眠 / 锁屏 / 重启 / 关机。
class SettingsPowerTiles extends StatelessWidget {
  const SettingsPowerTiles({
    super.key,
    required this.onSleep,
    required this.onLock,
    required this.onRestart,
    required this.onShutdown,
  });

  final VoidCallback onSleep;
  final VoidCallback onLock;
  final VoidCallback onRestart;
  final VoidCallback onShutdown;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final tiles = <(IconData, String, VoidCallback)>[
      (Icons.bedtime_rounded, '睡眠', onSleep),
      (Icons.lock_outline_rounded, '锁屏', onLock),
      (Icons.restart_alt_rounded, '重启', onRestart),
      (Icons.power_settings_new_rounded, '关机', onShutdown),
    ];
    return Row(
      children: [
        for (var i = 0; i < tiles.length; i++) ...[
          if (i > 0) const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Builder(
              builder: (context) {
                // 关机是危险项：浅红底 + 红描边 + 红字，与主操作区分。
                final danger = i == tiles.length - 1;
                return AppCard(
                  minHeight: 96,
                  radius: AppRadius.md,
                  padding: EdgeInsets.zero,
                  color: danger ? palette.dangerTint : null,
                  borderColor: danger
                      ? palette.danger.withValues(alpha: 0.32)
                      : null,
                  onTap: tiles[i].$3,
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        tiles[i].$1,
                        size: 22,
                        color: danger ? palette.danger : palette.textPrimary,
                      ),
                      const SizedBox(height: AppSpacing.md),
                      Text(
                        tiles[i].$2,
                        style: AppText.label.copyWith(
                          color: danger
                              ? palette.danger
                              : palette.textPrimary,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),
          ),
        ],
      ],
    );
  }
}

/// 设置分组内的一行：标题 + 副标题 + 右侧箭头（或自定义尾部）。
class SettingsRow extends StatelessWidget {
  const SettingsRow({
    super.key,
    required this.title,
    this.subtitle,
    this.onTap,
    this.trailing,
    this.busy = false,
  });

  final String title;
  final String? subtitle;
  final VoidCallback? onTap;
  final Widget? trailing;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(AppRadius.lg),
      child: Container(
        constraints: const BoxConstraints(minHeight: AppMetrics.listRow),
        padding: const EdgeInsets.symmetric(
          horizontal: AppMetrics.cardPadding,
          vertical: AppSpacing.md,
        ),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    title,
                    style: AppText.rowTitle.copyWith(
                      color: onTap == null
                          ? palette.textMuted
                          : palette.textPrimary,
                    ),
                  ),
                  if (subtitle != null) ...[
                    const SizedBox(height: 2),
                    Text(
                      subtitle!,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppText.meta.copyWith(color: palette.textMuted),
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(width: AppSpacing.sm),
            if (busy)
              SizedBox(
                width: 18,
                height: 18,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: palette.brand,
                ),
              )
            else
              trailing ??
                  Icon(
                    Icons.chevron_right_rounded,
                    size: 18,
                    color: palette.textSubtle,
                  ),
          ],
        ),
      ),
    );
  }
}

/// 分组行分隔线，与卡片同宽。
class SettingsRowDivider extends StatelessWidget {
  const SettingsRowDivider({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: AppMetrics.cardPadding),
      child: Divider(height: 1, color: context.palette.line),
    );
  }
}

/// 连接信息的只读键值行。
class SettingsValueRow extends StatelessWidget {
  const SettingsValueRow({
    super.key,
    required this.label,
    required this.value,
  });

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 15),
      child: Row(
        children: [
          Text(label, style: AppText.body.copyWith(color: palette.textMuted)),
          const SizedBox(width: AppSpacing.md),
          Expanded(
            child: Text(
              value,
              textAlign: TextAlign.end,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: AppText.body.copyWith(
                color: palette.textPrimary,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// 危险操作的描边按钮：与主按钮不同色、不同排（设计规则 4）。
class SettingsDangerButton extends StatelessWidget {
  const SettingsDangerButton({
    super.key,
    required this.label,
    required this.onPressed,
  });

  final String label;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: 48,
      child: OutlinedButton(
        onPressed: onPressed,
        style: OutlinedButton.styleFrom(
          foregroundColor: palette.danger,
          side: BorderSide(color: palette.danger.withValues(alpha: 0.45)),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.md),
          ),
        ),
        child: Text(
          label,
          style: AppText.body.copyWith(
            color: palette.danger,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}
