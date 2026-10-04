import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/core/errors/pairing_failures.dart';

void main() {
  test('maps every documented pairing error code to Chinese copy', () {
    expect(
      pairingFailureText('PAIRING_REQUIRES_DESKTOP'),
      contains('解锁'),
    );
    expect(pairingFailureText('PAIRING_BUSY'), contains('过多'));
    expect(pairingFailureText('INVALID_REQUEST'), contains('设备信息'));
    expect(pairingFailureText('PAIRING_EXPIRED'), contains('重新配对'));
    expect(pairingFailureText('PAIRING_NOT_FOUND'), contains('重新配对'));
    expect(pairingFailureText('INVALID_PIN'), contains('配对码'));
    expect(pairingFailureText('RATE_LIMITED'), contains('尝试次数'));
    expect(pairingFailureText('DEVICE_MISMATCH'), contains('设备标识'));
    expect(pairingFailureText('PAIRING_NETWORK_ERROR'), contains('网络'));
  });

  test('never leaks the raw code and prefers the server text otherwise', () {
    for (final code in const [
      'PAIRING_REQUIRES_DESKTOP',
      'PAIRING_BUSY',
      'INVALID_REQUEST',
    ]) {
      expect(pairingFailureText(code), isNot(contains(code)));
    }
    expect(
      pairingFailureText('SOMETHING_UNKNOWN', '电脑不太对劲'),
      '电脑不太对劲',
    );
    expect(pairingFailureText('SOMETHING_UNKNOWN'), '配对失败，请重试');
    expect(pairingFailureText('SOMETHING_UNKNOWN', ''), '配对失败，请重试');
  });
}
