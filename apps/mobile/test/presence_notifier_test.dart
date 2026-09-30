import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/utils/presence_notifier.dart';

void main() {
  group('PresenceNotificationTracker', () {
    final now = DateTime(2026, 8, 17, 12, 0, 0);

    test('WS 重连成功（无离线切换）不提示', () {
      final tracker = PresenceNotificationTracker();
      // 断线后在宽限期内恢复：从未产生真实离线事件，恢复上线也不提示。
      expect(tracker.onOnline(foregrounded: true), isFalse);
      expect(tracker.offlineEpisodeActive, isFalse);
    });

    test('首次连接成功不提示上线', () {
      final tracker = PresenceNotificationTracker();
      expect(tracker.onOnline(foregrounded: true), isFalse);
    });

    test('前台真实离线提示一次', () {
      final tracker = PresenceNotificationTracker();
      expect(tracker.onOffline(foregrounded: true, authenticated: true, now: now), isTrue);
      expect(tracker.offlineEpisodeActive, isTrue);
    });

    test('离线发生在后台不提示', () {
      final tracker = PresenceNotificationTracker();
      expect(tracker.onOffline(foregrounded: false, authenticated: true, now: now), isFalse);
      expect(tracker.offlineEpisodeActive, isFalse);
    });

    test('登出/未认证后的离线不提示', () {
      final tracker = PresenceNotificationTracker();
      expect(tracker.onOffline(foregrounded: true, authenticated: false, now: now), isFalse);
    });

    test('30s 冷却抑制重复离线提示', () {
      final tracker = PresenceNotificationTracker();
      expect(tracker.onOffline(foregrounded: true, authenticated: true, now: now), isTrue);
      expect(tracker.onOffline(foregrounded: true, authenticated: true, now: now), isFalse);
    });

    test('离线已提示过，恢复上线才补一条上线提示', () {
      final tracker = PresenceNotificationTracker();
      tracker.onOffline(foregrounded: true, authenticated: true, now: now);
      expect(tracker.onOnline(foregrounded: true), isTrue);
      expect(tracker.offlineEpisodeActive, isFalse);
    });

    test('后台恢复上线不提示且复位本轮标记', () {
      final tracker = PresenceNotificationTracker();
      tracker.onOffline(foregrounded: true, authenticated: true, now: now);
      expect(tracker.onOnline(foregrounded: false), isFalse);
      expect(tracker.offlineEpisodeActive, isFalse);
    });

    test('回前台 resetEpisode 清除未结算回合，恢复上线不再补提示', () {
      final tracker = PresenceNotificationTracker();
      tracker.onOffline(foregrounded: true, authenticated: true, now: now);
      expect(tracker.offlineEpisodeActive, isTrue);
      tracker.resetEpisode();
      expect(tracker.offlineEpisodeActive, isFalse);
      expect(tracker.onOnline(foregrounded: true), isFalse);
    });

    test('未提示过离线时恢复上线不提示', () {
      final tracker = PresenceNotificationTracker();
      expect(tracker.onOnline(foregrounded: true), isFalse);
    });
  });

}
