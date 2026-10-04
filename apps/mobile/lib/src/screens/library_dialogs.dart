/// 「库」里的对话框：删除/移除确认与重命名、启动参数、添加命名的输入框。
library;

import 'package:flutter/material.dart';

import '../models/app_info.dart';
import '../theme/app_theme.dart';

/// 删除用户添加的程序前的二次确认。
///
/// 用户取消或点掉对话框时返回 `false`。
Future<bool> confirmLibraryItemDeletion(
  BuildContext context,
  LauncherItem item,
) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: const Text('删除程序'),
      content: Text('确定删除「${item.name}」吗？删除后将从库中移除。'),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx, false),
          child: const Text('取消'),
        ),
        FilledButton(
          style: FilledButton.styleFrom(backgroundColor: AppColors.red),
          onPressed: () => Navigator.pop(ctx, true),
          child: const Text('删除'),
        ),
      ],
    ),
  );
  return confirmed == true;
}

/// 从常用中移除前的二次确认。
///
/// 用户取消或点掉对话框时返回 `false`。
Future<bool> confirmLibraryFavoriteRemoval(
  BuildContext context,
  LauncherItem item,
) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: const Text('从库中移除'),
      content: Text('确定将「${item.name}」从常用中移除吗？'),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx, false),
          child: const Text('取消'),
        ),
        FilledButton(
          style: FilledButton.styleFrom(backgroundColor: AppColors.red),
          onPressed: () => Navigator.pop(ctx, true),
          child: const Text('移除'),
        ),
      ],
    ),
  );
  return confirmed == true;
}

/// 重命名对话框。
///
/// 返回去空格后的新名字；取消、留空或与 [current] 相同时返回 `null`。
Future<String?> promptLibraryRename(BuildContext context, String current) async {
  final controller = TextEditingController(text: current);
  final name = await showDialog<String>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: const Text('重命名'),
      content: TextField(
        controller: controller,
        autofocus: true,
        decoration: const InputDecoration(labelText: '显示名称'),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(ctx, controller.text),
          child: const Text('保存'),
        ),
      ],
    ),
  );
  controller.dispose();
  final trimmed = name?.trim();
  if (trimmed == null || trimmed.isEmpty || trimmed == current) return null;
  return trimmed;
}

/// 启动参数编辑对话框，返回原样的输入内容；取消时返回 `null`。
Future<String?> promptLibraryLaunchArgs(
  BuildContext context,
  String current,
) async {
  final controller = TextEditingController(text: current);
  final value = await showDialog<String>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: const Text('启动参数'),
      content: TextField(
        controller: controller,
        autofocus: true,
        minLines: 1,
        maxLines: 3,
        decoration: const InputDecoration(
          labelText: '参数（空格分隔，可用引号）',
          hintText: '例如 -start steam://open/bigpicture',
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(ctx, controller.text),
          child: const Text('保存'),
        ),
      ],
    ),
  );
  controller.dispose();
  return value;
}

/// 「添加到库」的显示名称输入框，返回原样的输入内容；取消时返回 `null`。
Future<String?> promptLibraryItemName(
  BuildContext context,
  String suggested,
) async {
  final controller = TextEditingController(text: suggested);
  final name = await showDialog<String>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: const Text('添加到库'),
      content: TextField(
        controller: controller,
        autofocus: true,
        decoration: const InputDecoration(labelText: '显示名称'),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(ctx, controller.text),
          child: const Text('添加'),
        ),
      ],
    ),
  );
  controller.dispose();
  return name;
}
