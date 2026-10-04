/// 主壳顶部状态栏：应用标识、主题切换与设备状态药丸。
///
/// 由主壳 `home_screen.dart` 装配；底部导航在 `home_bottom_nav.dart`，
/// 连接与提示逻辑分别在 `home_connection.dart` / `home_notifications.dart`。
library;

import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../providers/theme_provider.dart';
import '../services/ws_client.dart';
import '../theme/app_theme.dart';

/// 主壳顶部栏：毛玻璃底 + 「轻联」标识 + 主题切换 + 设备状态药丸。
class HomeTopBar extends ConsumerWidget {
  final AuthState authState;
  final WsConnectionState remoteWsState;
  final String? pcName;

  const HomeTopBar({
    super.key,
    required this.authState,
    required this.remoteWsState,
    this.pcName,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 14, sigmaY: 14),
        child: Container(
          height: 56 + MediaQuery.of(context).padding.top,
          padding: EdgeInsets.only(
            top: MediaQuery.of(context).padding.top,
            left: 18,
            right: 18,
            bottom: 8,
          ),
          decoration: BoxDecoration(
            color: palette.topbarBackground,
            border: Border(
              bottom: BorderSide(color: palette.line.withValues(alpha: 0.5)),
            ),
          ),
          child: Row(
            children: [
              Container(
                width: 30,
                height: 30,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: AppColors.primary,
                  borderRadius: BorderRadius.circular(9),
                ),
                child: const Text(
                  'Q',
                  style: TextStyle(
                    color: Color(0xFF082219),
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  '轻联',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w700,
                    color: palette.textPrimary,
                  ),
                ),
              ),
              _TopIconButton(
                icon: ref.watch(themeModeProvider) == ThemeMode.dark
                    ? Icons.wb_sunny_outlined
                    : Icons.nightlight_round,
                tooltip: '切换主题',
                onPressed: () => ref.read(themeModeProvider.notifier).toggle(),
              ),
              const SizedBox(width: 4),
              _DevicePill(
                pcName: pcName?.isNotEmpty == true ? pcName! : '客厅电脑',
                connected: remoteWsState == WsConnectionState.connected,
                isAuthenticated: authState.isAuthenticated,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 顶部栏图标按钮 — 44x44, 10px radius, transparent bg
class _TopIconButton extends StatelessWidget {
  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  const _TopIconButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 44,
      height: 44,
      child: IconButton(
        tooltip: tooltip,
        onPressed: onPressed,
        icon: Icon(icon, size: 19),
        color: context.palette.textMuted,
        style: IconButton.styleFrom(
          backgroundColor: Colors.transparent,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(10),
          ),
        ),
      ),
    );
  }
}

/// 顶部设备名称药丸 — min-height 44, gap 8, radius 10
class _DevicePill extends StatelessWidget {
  final String pcName;
  final bool connected;
  final bool isAuthenticated;

  const _DevicePill({
    required this.pcName,
    required this.connected,
    required this.isAuthenticated,
  });

  @override
  Widget build(BuildContext context) {
    if (!isAuthenticated) return const SizedBox.shrink();
    final palette = context.palette;
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 118),
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: () {},
        child: Container(
          constraints: const BoxConstraints(minHeight: 44),
          padding: const EdgeInsets.only(left: 6, right: 4),
          decoration: BoxDecoration(
            color: Colors.transparent,
            borderRadius: BorderRadius.circular(10),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 7,
                height: 7,
                decoration: BoxDecoration(
                  color: connected ? AppColors.primary : palette.textSubtle,
                  shape: BoxShape.circle,
                  boxShadow: connected
                      ? [
                          BoxShadow(
                            color: AppColors.primary.withValues(alpha: 0.12),
                            blurRadius: 4,
                            spreadRadius: 2,
                          ),
                        ]
                      : null,
                ),
              ),
              const SizedBox(width: 5),
              Flexible(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      pcName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: palette.textPrimary,
                      ),
                    ),
                    Text(
                      connected ? '已连接' : '已关机',
                      style: TextStyle(fontSize: 10, color: palette.textMuted),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 2),
              Icon(
                Icons.keyboard_arrow_down_rounded,
                size: 14,
                color: palette.textSubtle,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
