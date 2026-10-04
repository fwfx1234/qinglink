/// 应用 UI 组件共用的排版字号体系。
library;

import 'package:flutter/material.dart';

/// 统一文本样式 —— 对齐原型 `styles.css` 的字号/字重。
///
/// 颜色由调用方通过 `.copyWith(color: ...)` 注入当前调色板。
abstract final class AppText {
  static const TextStyle eyebrow = TextStyle(
    fontSize: 10,
    letterSpacing: 1.1,
    height: 1.2,
  );
  static const TextStyle pageTitle = TextStyle(
    fontSize: 22,
    height: 1.15,
    letterSpacing: -0.4,
    fontWeight: FontWeight.w700,
  );
  static const TextStyle sectionTitle = TextStyle(
    fontSize: 14,
    fontWeight: FontWeight.w600,
  );
  static const TextStyle cardTitle = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w700,
  );
  static const TextStyle rowTitle = TextStyle(
    fontSize: 12,
    fontWeight: FontWeight.w700,
  );
  static const TextStyle body = TextStyle(fontSize: 12);
  static const TextStyle label = TextStyle(fontSize: 11);
  static const TextStyle meta = TextStyle(fontSize: 10);
  static const TextStyle tiny = TextStyle(fontSize: 9);
}
