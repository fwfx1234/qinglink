/// IR 页面共享组件的桶文件。
///
/// 各组件已按职责拆分到同目录下的独立库；本文件只做 `export`，
/// 使既有的 `import 'ir_widgets.dart';` 调用方无需任何改动。
library;

export 'ir_command_dialog.dart';
export 'ir_feedback.dart';
export 'ir_macro_dialog.dart';
export 'ir_remote_key.dart';
export 'ir_remote_panel.dart';
