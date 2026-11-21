import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../models/display_info.dart';
import '../providers/system_controller.dart';
import '../widgets/app_sheet.dart';

class LaunchModeSelection {
  const LaunchModeSelection({this.mode, this.clear = false});

  final DisplayModeInfo? mode;
  final bool clear;
}

Future<LaunchModeSelection?> showLibraryLaunchModeSheet(
  BuildContext context, {
  required LauncherItem item,
  required SystemController controller,
}) async {
  final primary = await controller.primaryDisplay();
  if (!context.mounted) return null;
  if (primary == null) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('读取 PC 主显示器失败')),
    );
    return null;
  }
  return showAppSheet<LaunchModeSelection>(
    context,
    isScrollControlled: true,
    builder: (sheetContext) => SafeArea(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: MediaQuery.sizeOf(sheetContext).height * 0.75,
        ),
        child: ListView(
          shrinkWrap: true,
          children: [
            ListTile(
              title: Text(item.name),
              subtitle: Text('主显示器 · 当前 ${primary.currentMode.resolution}'),
            ),
            for (final mode in primary.modes)
              ListTile(
                leading: Icon(
                  _sameMode(item.launchMode, mode)
                      ? Icons.radio_button_checked
                      : Icons.radio_button_unchecked,
                ),
                title: Text(mode.resolution),
                trailing: Text(mode.refreshText),
                onTap: () => Navigator.pop(
                  sheetContext,
                  LaunchModeSelection(mode: mode),
                ),
              ),
            if (item.launchMode != null)
              ListTile(
                leading: const Icon(Icons.clear),
                title: const Text('清除启动分辨率'),
                onTap: () => Navigator.pop(
                  sheetContext,
                  const LaunchModeSelection(clear: true),
                ),
              ),
          ],
        ),
      ),
    ),
  );
}

bool _sameMode(DisplayModeInfo? left, DisplayModeInfo right) =>
    left?.width == right.width &&
    left?.height == right.height &&
    left?.refreshMillihz == right.refreshMillihz;
