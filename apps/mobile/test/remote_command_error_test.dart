import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/core/network/remote_command_exception.dart';
import 'package:qinglink_mobile/src/providers/system_controller.dart';

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

  test('builds unlock state through the system controller model', () {
    const initial = SystemState();
    expect(initial.unlock, isNull);
  });
}
