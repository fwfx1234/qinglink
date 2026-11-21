import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import '../providers/feature_facades.dart';
import '../providers/system_controller.dart';
import '../theme/app_theme.dart';
import '../utils/remote_actions.dart';
import 'app_layout.dart';
import 'process_icon.dart';
import 'window_controls.dart';

/// 首页与控制页共用的「当前窗口」模块。
///
/// 直接订阅 Provider 计算可用状态，避免两个页面因各自判断 `ready`/`isReady`
/// 出现状态不一致；前台窗口标题/进程名缺失时逐级回退。
/// 控制页用卡片形态（[ActiveWindowSection]），首页用底部常驻 Dock 条
/// （[WindowDockSection]）——窗口控制要落在拇指区。
class ActiveWindowSection extends ConsumerWidget {
  const ActiveWindowSection({super.key, this.sectionTitle});

  /// 非空时在卡片上方渲染区块标题。
  final String? sectionTitle;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final foreground = ref.watch(
      systemControllerProvider.select((s) => s.foreground),
    );
    final connected = ref.watch(connectionReadyProvider);
    final enabled = connected && foreground != null;

    final card = ActiveWindowCard(
      title: resolveWindowTitle(foreground),
      meta: resolveWindowMeta(foreground),
      icon: _WindowIcon(foreground: foreground, size: 44, radius: 12),
      enabled: enabled,
      onWindows: () => showWindowListSheet(context, ref),
      onMinimize: () => runWindowAction(context, ref, 'minimize'),
      onMaximize: () => runWindowAction(context, ref, 'maximize'),
      onClose: () => runWindowAction(context, ref, 'close'),
    );

    if (sectionTitle == null) return card;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeading(title: sectionTitle!, bottom: AppSpacing.sm),
        card,
      ],
    );
  }
}

/// 首页底部常驻的窗口 Dock 条；按钮语义与卡片形态完全一致。
class WindowDockSection extends ConsumerWidget {
  const WindowDockSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final foreground = ref.watch(
      systemControllerProvider.select((s) => s.foreground),
    );
    final connected = ref.watch(connectionReadyProvider);
    return WindowDock(
      title: resolveWindowTitle(foreground),
      active: connected && foreground != null,
      enabled: connected && foreground != null,
      onWindows: () => showWindowListSheet(context, ref),
      onMinimize: () => runWindowAction(context, ref, 'minimize'),
      onMaximize: () => runWindowAction(context, ref, 'maximize'),
      onClose: () => runWindowAction(context, ref, 'close'),
    );
  }
}

/// 当前前台窗口的标题；标题为空时逐级回退到进程名。
String resolveWindowTitle(ForegroundResponse? foreground) {
  if (foreground == null) return '暂无活动窗口';
  final title = foreground.title.trim();
  if (title.isNotEmpty) return title;
  final process = foreground.processName.trim();
  if (process.isNotEmpty) return process;
  return '当前窗口';
}

/// 当前前台窗口的说明行。
String resolveWindowMeta(ForegroundResponse? foreground) {
  if (foreground == null) return '未检测到前台窗口';
  // 窗口卡右侧要放 4 个 40 宽的按钮，留给副标题的宽度只有 80dp 左右：
  // 只保留 PID，别再拼「前台运行」，否则一定会被省略号截断。
  return 'PID ${foreground.pid}';
}

/// 窗口卡左侧的真实程序图标，取不到时退回通用窗口图标。
class _WindowIcon extends StatelessWidget {
  const _WindowIcon({
    required this.foreground,
    required this.size,
    required this.radius,
  });

  final ForegroundResponse? foreground;
  final double size;
  final double radius;

  @override
  Widget build(BuildContext context) {
    if (foreground == null) {
      return ProcessIcon(
        path: null,
        pid: 0,
        size: size,
        radius: radius,
        color: context.palette.violet,
        fallback: Icons.window_rounded,
      );
    }
    return ProcessIcon(
      path: foreground!.path,
      pid: foreground!.pid,
      size: size,
      radius: radius,
      color: context.palette.violet,
      fallback: Icons.window_rounded,
    );
  }
}
