import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/utils/online_grace_tracker.dart';

void main() {
  test('markOnline 后断线在宽限期内恢复保持在线、不触发离线', () async {
    var offline = 0;
    final tracker = OnlineGraceTracker(
      grace: const Duration(milliseconds: 50),
      onOffline: () => offline++,
    );
    addTearDown(tracker.dispose);

    expect(tracker.isOnline, isFalse);
    tracker.markOnline();
    expect(tracker.isOnline, isTrue);

    tracker.onDisconnected(explicit: false);
    await Future<void>.delayed(const Duration(milliseconds: 20));
    tracker.markOnline(); // 宽限期内恢复
    await Future<void>.delayed(const Duration(milliseconds: 80));

    expect(tracker.isOnline, isTrue);
    expect(offline, 0);
  });

  test('宽限到期未恢复触发一次离线', () async {
    var offline = 0;
    final tracker = OnlineGraceTracker(
      grace: const Duration(milliseconds: 40),
      onOffline: () => offline++,
    );
    addTearDown(tracker.dispose);

    tracker.markOnline();
    tracker.onDisconnected(explicit: false);
    await Future<void>.delayed(const Duration(milliseconds: 100));

    expect(tracker.isOnline, isFalse);
    expect(offline, 1);

    // 已离线后再断开不会重复触发。
    tracker.onDisconnected(explicit: false);
    await Future<void>.delayed(const Duration(milliseconds: 100));
    expect(offline, 1);
  });

  test('显式断开立即离线且不触发 onOffline', () async {
    var offline = 0;
    final tracker = OnlineGraceTracker(
      grace: const Duration(milliseconds: 40),
      onOffline: () => offline++,
    );
    addTearDown(tracker.dispose);

    tracker.markOnline();
    tracker.onDisconnected(explicit: true);

    expect(tracker.isOnline, isFalse);
    await Future<void>.delayed(const Duration(milliseconds: 80));
    expect(offline, 0);
  });

  test('后台暂停不判离线，恢复后重新进入宽限期', () async {
    var offline = 0;
    final tracker = OnlineGraceTracker(
      grace: const Duration(milliseconds: 40),
      onOffline: () => offline++,
    );
    addTearDown(tracker.dispose);

    tracker.markOnline();
    tracker.pause();
    tracker.onDisconnected(explicit: false);
    await Future<void>.delayed(const Duration(milliseconds: 80));

    expect(tracker.isOnline, isTrue); // 后台不做离线判定
    expect(offline, 0);

    tracker.resume();
    tracker.onDisconnected(explicit: false); // 回前台重连失败 → 重新进入宽限
    await Future<void>.delayed(const Duration(milliseconds: 80));

    expect(tracker.isOnline, isFalse);
    expect(offline, 1);
  });
}
