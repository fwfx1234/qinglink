/// 设置（`ConnectionConfig`）provider 的桶文件。
///
/// 原单文件已按职责拆分到 `lib/src/providers/` 下的独立库：
/// - `settings_models.dart`：设置模型与序列化（`QuickApp` / `ShortcutPreset` /
///   `ConnectionConfig`）；
/// - `settings_keys.dart`：本地存储键名；
/// - `settings_persistence.dart`：本地持久化读写与旧数据解析；
/// - `settings_notifier.dart`：`SettingsNotifier` 与 `settingsProvider`。
///
/// 本文件只做 `export`，既有 `import '.../providers/settings_provider.dart';`
/// 调用方无需任何改动。
library;

export 'settings_models.dart';
export 'settings_notifier.dart';
