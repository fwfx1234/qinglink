import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/providers/remote_control_provider.dart';

void main() {
  test('preserves a server error code and message', () {
    final error = RemoteCommandException.fromResponse({
      'type': 'error',
      'data': {'code': 'DISPLAY_LIST_FAILED', 'message': '无法读取任何已连接显示器的当前显示模式'},
    });

    expect(error.code, 'DISPLAY_LIST_FAILED');
    expect(error.message, '无法读取任何已连接显示器的当前显示模式');
    expect(error.toString(), contains('DISPLAY_LIST_FAILED'));
  });

  test('tracks and clears the display-specific error state', () {
    const initial = RemoteControlState();
    final failed = initial.copyWith(
      displayError: 'DISPLAY_LIST_FAILED: failed',
    );
    final recovered = failed.copyWith(displayError: '');

    expect(failed.displayError, isNotEmpty);
    expect(recovered.displayError, isEmpty);
  });

  test('builds a confirmation unlock request without an implicit PIN', () {
    expect(autoUnlockRequestPayload(), {'type': 'auto_unlock'});
    expect(
      autoUnlockRequestPayload(lockPassword: '1234'),
      {'type': 'auto_unlock', 'pin': '1234'},
    );
  });
}
