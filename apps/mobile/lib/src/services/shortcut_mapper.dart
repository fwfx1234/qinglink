/// 快捷键标签到 PC 端 `key_event` 的 HID 用法与修饰键映射。
library;

/// 一次按键：HID usage 加修饰键位。
class ShortcutStroke {
  const ShortcutStroke(this.usage, {this.modifiers = 0});

  /// HID 用法码，服务端只接受 0x04 到 0x73。
  final int usage;

  /// 修饰键位掩码。
  final int modifiers;
}

abstract final class ShortcutMapper {
  /// USB HID 修饰键位：左 Ctrl / 左 Shift / 左 Alt / 左 Win。
  static const int ctrl = 0x01;
  static const int shift = 0x02;
  static const int alt = 0x04;
  static const int meta = 0x08;

  /// 具名键与键盘符号的 HID 用法码。
  static const Map<String, int> _keys = <String, int>{
    'enter': 0x28,
    'return': 0x28,
    'esc': 0x29,
    'escape': 0x29,
    'backspace': 0x2a,
    'tab': 0x2b,
    'space': 0x2c,
    'insert': 0x49,
    'home': 0x4a,
    'pageup': 0x4b,
    'delete': 0x4c,
    'del': 0x4c,
    'end': 0x4d,
    'pagedown': 0x4e,
    'right': 0x4f,
    'left': 0x50,
    'down': 0x51,
    'up': 0x52,
    '→': 0x4f,
    '←': 0x50,
    '↓': 0x51,
    '↑': 0x52,
    'f1': 0x3a,
    'f2': 0x3b,
    'f3': 0x3c,
    'f4': 0x3d,
    'f5': 0x3e,
    'f6': 0x3f,
    'f7': 0x40,
    'f8': 0x41,
    'f9': 0x42,
    'f10': 0x43,
    'f11': 0x44,
    'f12': 0x45,
    '`': 0x35,
    'minus': 0x2d,
    '-': 0x2d,
    'equal': 0x2e,
    '=': 0x2e,
    'leftbracket': 0x2f,
    '[': 0x2f,
    'rightbracket': 0x30,
    ']': 0x30,
    r'\': 0x31,
    'semicolon': 0x33,
    ';': 0x33,
    'apostrophe': 0x34,
    "'": 0x34,
    ',': 0x36,
    'comma': 0x36,
    '.': 0x37,
    'period': 0x37,
    '/': 0x38,
    'slash': 0x38,
  };

  /// 需要 Shift 的符号，映射到对应的物理主键。
  static const Map<String, int> _shiftedKeys = <String, int>{
    '~': 0x35,
    '!': 0x1e,
    '@': 0x1f,
    '#': 0x20,
    r'$': 0x21,
    '%': 0x22,
    '^': 0x23,
    '&': 0x24,
    '*': 0x25,
    '(': 0x26,
    ')': 0x27,
    '_': 0x2d,
    'plus': 0x2e,
    '{': 0x2f,
    '}': 0x30,
    '|': 0x31,
    ':': 0x33,
    '"': 0x34,
    '<': 0x36,
    '>': 0x37,
    '?': 0x38,
  };

  /// 修饰键别名。
  static const Map<String, int> _modifiers = <String, int>{
    'ctrl': ctrl,
    'control': ctrl,
    'ctl': ctrl,
    'shift': shift,
    'alt': alt,
    'opt': alt,
    'option': alt,
    'win': meta,
    'cmd': meta,
    'command': meta,
    'super': meta,
    'meta': meta,
  };

  /// 软键盘编辑器里的修饰键。
  static const List<String> modifierLabels = <String>[
    'Ctrl',
    'Shift',
    'Alt',
    'Win',
  ];

  /// 软键盘编辑器里的主键，修饰键由编辑器单独组合。
  static const Map<String, List<String>> keyGroups = <String, List<String>>{
    '字母': <String>[
      'A',
      'B',
      'C',
      'D',
      'E',
      'F',
      'G',
      'H',
      'I',
      'J',
      'K',
      'L',
      'M',
      'N',
      'O',
      'P',
      'Q',
      'R',
      'S',
      'T',
      'U',
      'V',
      'W',
      'X',
      'Y',
      'Z',
    ],
    '数字': <String>['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
    '符号': <String>[
      '`',
      '-',
      '=',
      '[',
      ']',
      r'\',
      ';',
      "'",
      ',',
      '.',
      '/',
      '~',
      '!',
      '@',
      '#',
      r'$',
      '%',
      '^',
      '&',
      '*',
      '(',
      ')',
      '_',
      'Plus',
      '{',
      '}',
      '|',
      ':',
      '"',
      '<',
      '>',
      '?',
    ],
    '常用': <String>['Tab', 'Esc', 'Enter', 'Space', 'Backspace', 'Delete'],
    '方向': <String>['↑', '↓', '←', '→'],
    '功能键': <String>[
      'Home',
      'End',
      'PageUp',
      'PageDown',
      'F1',
      'F2',
      'F3',
      'F4',
      'F5',
      'F6',
      'F7',
      'F8',
      'F9',
      'F10',
      'F11',
      'F12',
    ],
    '导航': <String>['Home', 'End', 'PageUp', 'PageDown', 'Insert'],
  };

  /// 根据软键盘的修饰键和主键生成规范化标签。
  static String compose(Iterable<String> modifiers, String key) {
    final selected = <String>{
      for (final modifier in modifiers)
        if (_modifierBit(modifier) != null) _canonicalModifier(modifier),
    };
    final ordered = modifierLabels.where(selected.contains).toList();
    return [...ordered, _canonicalKey(key)].join(' + ');
  }

  /// 解析一个快捷键标签；支持 `Ctrl + C`、`Alt + Enter`、`↑`、`F5` 等写法。
  ///
  /// 返回 `null` 表示标签无法翻译成 HID 事件（未知修饰键或未知主键）。
  static ShortcutStroke? fromLabel(String label) {
    final parts = label
        .split('+')
        .map((part) => part.trim())
        .where((part) => part.isNotEmpty)
        .toList(growable: false);
    if (parts.isEmpty) return null;

    var modifiers = 0;
    for (final part in parts.take(parts.length - 1)) {
      final bit = _modifiers[part.toLowerCase()];
      if (bit == null) return null;
      modifiers |= bit;
    }

    final key = parts.last;
    final shiftedUsage = _shiftedKeys[key.toLowerCase()];
    final usage = shiftedUsage ?? _usageOf(key);
    if (usage == null) return null;
    if (shiftedUsage != null) modifiers |= shift;
    return ShortcutStroke(usage, modifiers: modifiers);
  }

  /// 主键用法码：先查具名表，再按字母与数字的连续区间推导。
  static int? _usageOf(String raw) {
    final lower = raw.toLowerCase();
    final named = _keys[lower];
    if (named != null) return named;
    if (lower.length != 1) return null;

    final unit = lower.codeUnitAt(0);
    // a-z 对应 0x04 起；数字 1-9 对应 0x1e 起，0 单独在 0x27。
    if (unit >= 0x61 && unit <= 0x7a) return 0x04 + (unit - 0x61);
    if (unit >= 0x31 && unit <= 0x39) return 0x1e + (unit - 0x31);
    if (unit == 0x30) return 0x27;
    return null;
  }

  static int? _modifierBit(String label) => _modifiers[label.toLowerCase()];

  static String _canonicalModifier(String label) {
    return switch (label.toLowerCase()) {
      'control' || 'ctl' => 'Ctrl',
      'shift' => 'Shift',
      'alt' || 'opt' || 'option' => 'Alt',
      'win' || 'cmd' || 'command' || 'super' || 'meta' => 'Win',
      _ => label,
    };
  }

  static String _canonicalKey(String key) {
    return switch (key.toLowerCase()) {
      'return' => 'Enter',
      'escape' => 'Esc',
      'del' => 'Delete',
      'right' => '→',
      'left' => '←',
      'up' => '↑',
      'down' => '↓',
      _ when key.length == 1 && key.toLowerCase() == key => key.toUpperCase(),
      _ => key,
    };
  }
}
