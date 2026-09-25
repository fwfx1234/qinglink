import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import '../providers/remote_control_provider.dart';
import '../utils/remote_actions.dart';
import 'app_ui.dart';

/// 首页与控制页共用的「当前窗口」模块。
///
/// 直接订阅 Provider 计算可用状态，避免两个页面因各自判断 `ready`/`isReady`
/// 出现状态不一致；前台窗口标题/进程名缺失时逐级回退。
class ActiveWindowSection extends ConsumerWidget {
  const ActiveWindowSection({super.key, this.sectionTitle});

  /// 非空时在卡片上方渲染区块标题。
  final String? sectionTitle;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final foreground = ref.watch(
      remoteControlProvider.select((s) => s.foregroundApp),
    );
    final connected = ref.watch(
      remoteControlProvider.select((s) => s.isConnected),
    );

    final card = ActiveWindowCard(
      title: _resolveTitle(foreground),
      meta: foreground == null ? '未检测到前台窗口' : '前台运行 · PID ${foreground.pid}',
      enabled: connected && foreground != null,
      onWindows: () => showWindowListSheet(context, ref),
      onMinimize: () => runWindowAction(context, ref, 'minimize'),
      onMaximize: () => runWindowAction(context, ref, 'maximize'),
      onClose: () => runWindowAction(context, ref, 'close'),
    );

    if (sectionTitle == null) return card;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeading(title: sectionTitle!),
        card,
      ],
    );
  }

  String _resolveTitle(ForegroundResponse? foreground) {
    if (foreground == null) return '暂无活动窗口';
    final title = foreground.title.trim();
    if (title.isNotEmpty) return title;
    final process = foreground.processName.trim();
    if (process.isNotEmpty) return process;
    return '当前窗口';
  }
}
