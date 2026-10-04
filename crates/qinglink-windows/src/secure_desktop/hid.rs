//! 纯 USB HID 键盘用法码到 Set 1 扫描码的转换。
//!
//! HID 用法 ID 描述物理按键，而 `SendInput` 扫描码使用 Windows Set 1 的取值。
//! 字母表刻意做成表格而不是算术区间：QWERTY 扫描码并非按字母顺序排列。

/// 键盘 HID 用法 → (Set 1 扫描码, 扩展键标志) 的逐条映射表。
///
/// This table covers the keyboard usages emitted by the mobile shortcut
/// dock as well as the usual text/navigation keys. Keep entries explicit
/// where the HID sequence does not match the Set 1 scan-code sequence.
const USAGE_SCANCODES: &[(u8, u16, bool)] = &[
    // Letters A-Z.
    (0x04, 0x1e, false),
    (0x05, 0x30, false),
    (0x06, 0x2e, false),
    (0x07, 0x20, false),
    (0x08, 0x12, false),
    (0x09, 0x21, false),
    (0x0a, 0x22, false),
    (0x0b, 0x23, false),
    (0x0c, 0x17, false),
    (0x0d, 0x24, false),
    (0x0e, 0x25, false),
    (0x0f, 0x26, false),
    (0x10, 0x32, false),
    (0x11, 0x31, false),
    (0x12, 0x18, false),
    (0x13, 0x19, false),
    (0x14, 0x10, false),
    (0x15, 0x13, false),
    (0x16, 0x1f, false),
    (0x17, 0x14, false),
    (0x18, 0x16, false),
    (0x19, 0x2f, false),
    (0x1a, 0x11, false),
    (0x1b, 0x2d, false),
    (0x1c, 0x15, false),
    (0x1d, 0x2c, false),
    // Number row and punctuation.
    (0x1e, 0x02, false),
    (0x1f, 0x03, false),
    (0x20, 0x04, false),
    (0x21, 0x05, false),
    (0x22, 0x06, false),
    (0x23, 0x07, false),
    (0x24, 0x08, false),
    (0x25, 0x09, false),
    (0x26, 0x0a, false),
    (0x27, 0x0b, false),
    (0x28, 0x1c, false), // Enter
    (0x29, 0x01, false), // Escape
    (0x2a, 0x0e, false), // Backspace
    (0x2b, 0x0f, false), // Tab
    (0x2c, 0x39, false), // Space
    (0x2d, 0x0c, false),
    (0x2e, 0x0d, false),
    (0x2f, 0x1a, false),
    (0x30, 0x1b, false),
    (0x31, 0x2b, false),
    (0x33, 0x27, false),
    (0x34, 0x28, false),
    (0x35, 0x29, false),
    (0x36, 0x33, false),
    (0x37, 0x34, false),
    (0x38, 0x35, false),
    (0x39, 0x3a, false), // Caps Lock
    // Function and navigation keys.
    (0x3a, 0x3b, false),
    (0x3b, 0x3c, false),
    (0x3c, 0x3d, false),
    (0x3d, 0x3e, false),
    (0x3e, 0x3f, false),
    (0x3f, 0x40, false),
    (0x40, 0x41, false),
    (0x41, 0x42, false),
    (0x42, 0x43, false),
    (0x43, 0x44, false),
    (0x44, 0x57, false),
    (0x45, 0x58, false),
    (0x46, 0x37, true),  // Print Screen
    (0x47, 0x46, false), // Scroll Lock
    (0x48, 0x45, false), // Pause
    (0x49, 0x52, true),  // Insert
    (0x4a, 0x47, true),  // Home
    (0x4b, 0x49, true),  // Page Up
    (0x4c, 0x53, true),  // Delete
    (0x4d, 0x4f, true),  // End
    (0x4e, 0x51, true),  // Page Down
    (0x4f, 0x4d, true),  // Right
    (0x50, 0x4b, true),  // Left
    (0x51, 0x50, true),  // Down
    (0x52, 0x48, true),  // Up
    (0x53, 0x45, true),  // Num Lock
    // Keypad.
    (0x54, 0x35, true),
    (0x55, 0x37, false),
    (0x56, 0x4a, false),
    (0x57, 0x4e, false),
    (0x58, 0x1c, true),
    (0x59, 0x4f, false),
    (0x5a, 0x50, false),
    (0x5b, 0x51, false),
    (0x5c, 0x4b, false),
    (0x5d, 0x4c, false),
    (0x5e, 0x4d, false),
    (0x5f, 0x47, false),
    (0x60, 0x48, false),
    (0x61, 0x49, false),
    (0x62, 0x52, false),
    (0x63, 0x53, false),
    // Modifier usages when sent as individual HID keys.
    (0xe0, 0x1d, false), // Left Ctrl
    (0xe1, 0x2a, false), // Left Shift
    (0xe2, 0x38, false), // Left Alt
    (0xe3, 0x5b, true),  // Left GUI
    (0xe4, 0x1d, true),  // Right Ctrl
    (0xe5, 0x36, false), // Right Shift
    (0xe6, 0x38, true),  // Right Alt
    (0xe7, 0x5c, true),  // Right GUI
];

/// Return `(Set 1 scan code, extended-key flag)` for a keyboard HID usage.
pub fn usage_to_scancode(usage: u8) -> Option<(u16, bool)> {
    USAGE_SCANCODES
        .iter()
        .find_map(|&(key, scan, extended)| (key == usage).then_some((scan, extended)))
}

#[cfg(test)]
mod tests {
    use super::usage_to_scancode;

    #[test]
    fn maps_mobile_shortcut_usages_to_expected_scan_codes() {
        let cases = [
            (0x04, (0x1e, false)), // A
            (0x05, (0x30, false)), // B, not A+1
            (0x06, (0x2e, false)), // C
            (0x0e, (0x25, false)), // K
            (0x0f, (0x26, false)), // L
            (0x19, (0x2f, false)), // V
            (0x28, (0x1c, false)), // Enter
            (0x29, (0x01, false)), // Esc
            (0x2b, (0x0f, false)), // Tab
            (0x4f, (0x4d, true)),  // Right
            (0x50, (0x4b, true)),  // Left
            (0x51, (0x50, true)),  // Down
            (0x52, (0x48, true)),  // Up
        ];
        for (usage, expected) in cases {
            assert_eq!(
                usage_to_scancode(usage),
                Some(expected),
                "usage 0x{usage:02x}"
            );
        }
    }

    #[test]
    fn maps_common_keyboard_usage_table() {
        let cases = [
            (0x1e, (0x02, false)), // 1
            (0x27, (0x0b, false)), // 0
            (0x2a, (0x0e, false)), // Backspace
            (0x2c, (0x39, false)), // Space
            (0x39, (0x3a, false)), // Caps Lock
            (0x3a, (0x3b, false)), // F1
            (0x45, (0x58, false)), // F12
            (0x4c, (0x53, true)),  // Delete
            (0xe4, (0x1d, true)),  // Right Ctrl
        ];
        for (usage, expected) in cases {
            assert_eq!(
                usage_to_scancode(usage),
                Some(expected),
                "usage 0x{usage:02x}"
            );
        }
    }

    #[test]
    fn rejects_unknown_usage() {
        for usage in [0x00, 0x32, 0x64, 0xff] {
            assert_eq!(usage_to_scancode(usage), None, "usage 0x{usage:02x}");
        }
    }
}
