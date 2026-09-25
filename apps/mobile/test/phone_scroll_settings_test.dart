import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/providers/settings_provider.dart';
import 'package:qinglink_mobile/src/screens/mouse_keyboard_screen.dart';

void main() {
  test('existing settings default new input options to disabled', () {
    final config = ConnectionConfig.fromJson(const <String, dynamic>{});

    expect(config.reversePhoneScrollBar, isFalse);
    expect(config.autoUnlockOnWake, isFalse);
  });

  test('new input options survive serialization and copyWith changes', () {
    final updated = const ConnectionConfig().copyWith(
      reversePhoneScrollBar: true,
      autoUnlockOnWake: true,
    );
    final restored = ConnectionConfig.fromJson(updated.toJson());

    expect(restored.reversePhoneScrollBar, isTrue);
    expect(restored.autoUnlockOnWake, isTrue);
    expect(
      updated.copyWith(reversePhoneScrollBar: false).reversePhoneScrollBar,
      isFalse,
    );
  });

  test('natural scrolling reverses only the phone scroll bar multiplier', () {
    expect(phoneScrollDirection(false), -1);
    expect(phoneScrollDirection(true), 1);
  });

  test('keyboard sheet closes only after the IME transitions to hidden', () {
    expect(
      shouldCloseKeyboardSheet(
        sheetOpen: true,
        imeWasVisible: false,
        imeVisible: false,
      ),
      isFalse,
    );
    expect(
      shouldCloseKeyboardSheet(
        sheetOpen: true,
        imeWasVisible: true,
        imeVisible: true,
      ),
      isFalse,
    );
    expect(
      shouldCloseKeyboardSheet(
        sheetOpen: true,
        imeWasVisible: true,
        imeVisible: false,
      ),
      isTrue,
    );
    expect(
      shouldCloseKeyboardSheet(
        sheetOpen: false,
        imeWasVisible: true,
        imeVisible: false,
      ),
      isFalse,
    );
  });

  test('empty keyboard draft forwards an IME Backspace through its guard', () {
    expect(
      shouldForwardEmptyImeBackspace(
        guardWasDeleted: true,
        previousDraft: '',
        committedDraft: '',
      ),
      isTrue,
    );
    expect(
      shouldForwardEmptyImeBackspace(
        guardWasDeleted: true,
        previousDraft: 'draft',
        committedDraft: '',
      ),
      isFalse,
    );
  });
}
