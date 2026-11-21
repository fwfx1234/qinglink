/// 应用 UI 组件共用的排版字号体系。
///
/// 字号 / 行高 / 字重取自设计稿第 3 节（H1 26·H2 18·Body 14·Meta 12·Caption 10）。
/// 设计稿画布 390dp，真机 360dp 逻辑宽度，因此正文一级统一按 0.92 收缩一档
/// （Body 14→13、H2 18→16、H1 26→24），保持「字宽 / 屏宽」与设计稿一致。
library;

import 'package:flutter/material.dart';

/// 统一文本样式。
///
/// 颜色由调用方通过 `.copyWith(color: ...)` 注入当前调色板。
abstract final class AppText {
  /// 大标题上方的短标签（如「轻联 QingLink」）。
  static const TextStyle eyebrow = TextStyle(
    fontSize: 12,
    height: 18 / 12,
    letterSpacing: 0,
    fontWeight: FontWeight.w400,
  );

  /// 页面标题 H1。
  static const TextStyle pageTitle = TextStyle(
    fontSize: 24,
    height: 30 / 24,
    letterSpacing: -0.5,
    fontWeight: FontWeight.w600,
  );

  /// 分组标题 H2。
  static const TextStyle sectionTitle = TextStyle(
    fontSize: 16,
    height: 22 / 16,
    letterSpacing: 0,
    fontWeight: FontWeight.w600,
  );

  /// 卡片标题。
  static const TextStyle cardTitle = TextStyle(
    fontSize: 13,
    height: 19 / 13,
    fontWeight: FontWeight.w600,
  );

  /// 列表行标题。
  static const TextStyle rowTitle = TextStyle(
    fontSize: 13,
    height: 20 / 13,
    fontWeight: FontWeight.w600,
  );

  /// 正文。
  static const TextStyle body = TextStyle(
    fontSize: 13,
    height: 20 / 13,
    fontWeight: FontWeight.w500,
  );

  /// 正文的常规字重变体（说明文字、选项）。
  static const TextStyle label = TextStyle(
    fontSize: 11,
    height: 16 / 11,
    fontWeight: FontWeight.w500,
  );

  /// 辅助说明。
  static const TextStyle meta = TextStyle(
    fontSize: 11,
    height: 16 / 11,
    fontWeight: FontWeight.w400,
  );

  /// 角标、标签。
  static const TextStyle tiny = TextStyle(
    fontSize: 10,
    height: 14 / 10,
    fontWeight: FontWeight.w400,
  );

  /// 角标、标签（Caption 的语义别名）。
  static const TextStyle caption = tiny;
}
