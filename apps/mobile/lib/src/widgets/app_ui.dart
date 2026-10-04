/// 共享 UI 组件库的桶文件。
///
/// 各组件已按职责拆分到 `lib/src/widgets/` 下的独立库；本文件只做 `export`，
/// 使既有的 `import '.../widgets/app_ui.dart';` 调用方无需任何改动。
library;

export 'app_buttons.dart';
export 'app_card.dart';
export 'app_controls.dart';
export 'app_layout.dart';
export 'app_list_row.dart';
export 'app_remote_controls.dart';
export 'app_status.dart';
export 'app_text_styles.dart';
