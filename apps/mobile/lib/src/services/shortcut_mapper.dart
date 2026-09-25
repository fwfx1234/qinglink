/// Maps the labels shown in the mobile shortcut dock to HID usages.
class ShortcutStroke {
  const ShortcutStroke(this.usage, {this.modifiers = 0});

  final int usage;
  final int modifiers;
}

abstract final class ShortcutMapper {
  static const int ctrl = 0x01;
  static const int shift = 0x02;
  static const int alt = 0x04;

  static final Map<String, ShortcutStroke> _strokes = {
    'Tab': const ShortcutStroke(0x2b),
    'Esc': const ShortcutStroke(0x29),
    'Enter': const ShortcutStroke(0x28),
    '↑': const ShortcutStroke(0x52),
    '↓': const ShortcutStroke(0x51),
    '←': const ShortcutStroke(0x50),
    '→': const ShortcutStroke(0x4f),
    'Ctrl + C': const ShortcutStroke(0x06, modifiers: ctrl),
    'Ctrl + V': const ShortcutStroke(0x19, modifiers: ctrl),
    'Ctrl + K': const ShortcutStroke(0x0e, modifiers: ctrl),
    'Ctrl + L': const ShortcutStroke(0x0f, modifiers: ctrl),
    'Ctrl + Enter': const ShortcutStroke(0x28, modifiers: ctrl),
    'Alt + Enter': const ShortcutStroke(0x28, modifiers: alt),
  };

  static ShortcutStroke? fromLabel(String label) => _strokes[label.trim()];
}
