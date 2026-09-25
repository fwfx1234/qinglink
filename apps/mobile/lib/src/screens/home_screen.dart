import 'dart:async';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../models/system_status.dart';
import '../models/wake_state.dart';
import '../providers/auth_provider.dart';
import '../providers/remote_control_provider.dart';
import '../providers/settings_provider.dart';
import '../providers/theme_provider.dart';
import '../services/ws_client.dart';
import '../utils/presence_notifier.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../utils/tv_power_follow.dart';
import '../utils/wake_on_lan.dart';
import '../widgets/pairing_dialog.dart';
import 'home_tab.dart';
import 'library_tab.dart';
import 'more_tab.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen>
    with WidgetsBindingObserver {
  int _currentIndex = 0;
  WakeState _wakeState = WakeState.idle;
  late final PageController _pageController;
  bool _pairingDialogShown = false;
  DateTime? _lastPairingTime;
  bool _lastPcOnline = false;
  bool _autoWakePending = false;
  bool _autoWakeInFlight = false;
  bool _wakeFlowInFlight = false;
  bool _appForegrounded = true;
  DateTime? _lastWolSentAt;
  DateTime? _lastResumeAt;
  bool _pendingUnlockNotification = false;
  DateTime? _lastUnlockNotificationAt;
  final PresenceNotificationTracker _presenceTracker =
      PresenceNotificationTracker();
  StreamSubscription<void>? _offlineSubscription;
  Timer? _unlockStatusTimer;

  static const _wolCooldown = Duration(seconds: 60);

  /// 回前台后的重连宽限窗口：期间判定的离线不弹窗（自动唤醒仍正常触发）。
  static const _resumeNotificationSuppression = Duration(seconds: 30);

  List<Widget> _buildTabs() => [
    RepaintBoundary(
      child: HomeTab(
        wakeState: _wakeState,
        onGoToLibrary: () => _onTabTap(1),
        onGoToMore: () => _onTabTap(2),
        onWake: _sendWakeAndWait,
      ),
    ),
    const RepaintBoundary(child: LibraryTab()),
    const RepaintBoundary(child: MoreTab()),
  ];

  @override
  void initState() {
    super.initState();
    _pageController = PageController();
    WidgetsBinding.instance.addObserver(this);

    // 配对弹窗监听认证状态；上线/离线通知监听心跳活性在线状态与真实离线事件。
    ref.listenManual<AuthState>(authProvider, (previous, next) {
      _maybeShowPairingDialog(next);
    });
    ref.listenManual<bool>(
      remoteControlProvider.select((s) => s.pcOnline),
      (_, next) => _onPcOnlineChanged(next),
    );
    ref.listenManual<bool>(remoteControlProvider.select((s) => s.isConnected), (
      _,
      next,
    ) {
      // 会话恢复（ready）说明自动唤醒意图已达成，撤防待命中的 WOL，
      // 同时结束“回前台重连过渡期”，之后的真实离线恢复弹窗判定。
      if (next) {
        _autoWakePending = false;
        _lastResumeAt = null;
        // 连接成功后补齐 MAC（WoL 需要）并刷新锁屏状态，避免用旧状态判断解锁。
        unawaited(_fetchWakeNetworks());
        unawaited(ref.read(remoteControlProvider.notifier).getUnlockStatus());
      }
    });
    ref.listenManual<UnlockStatus?>(
      remoteControlProvider.select((s) => s.unlockStatus),
      (previous, next) {
        // Only report an actual locked -> unlocked transition. The initial
        // status query after connecting must stay silent.
        if (previous?.locked != true || next?.locked != false) return;
        if (!_appForegrounded) {
          _pendingUnlockNotification = true;
          return;
        }
        _notifyUnlockSuccess();
      },
    );
    _offlineSubscription = ref
        .read(remoteControlProvider.notifier)
        .offlineEvents
        .listen((_) => _onGenuineOffline());
    _unlockStatusTimer = Timer.periodic(const Duration(seconds: 5), (_) {
      if (!mounted || !ref.read(remoteControlProvider).isConnected) return;
      unawaited(ref.read(remoteControlProvider.notifier).getUnlockStatus());
    });

    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await ref.read(settingsProvider.notifier).loaded;
      if (mounted) {
        unawaited(_checkAutoWake());
      }
    });
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _appForegrounded = state == AppLifecycleState.resumed;
    if (state == AppLifecycleState.resumed && mounted) {
      // 后台→前台：清除未结算的离线/上线回合，回前台后的重连结果不再弹窗。
      // 若会话在后台未被挂起（仍连接中），则无需过渡期，直接按真实状态判定。
      _presenceTracker.resetEpisode();
      _lastResumeAt = ref.read(remoteControlProvider).isConnected
          ? null
          : DateTime.now();
      if (ref.read(remoteControlProvider).isConnected) {
        unawaited(ref.read(remoteControlProvider.notifier).refreshForeground());
      }
      unawaited(_checkAutoWake(fromResume: true));
      if (_pendingUnlockNotification) {
        _pendingUnlockNotification = false;
        _notifyUnlockSuccess();
      }
    }
  }

  /// 是否处于回前台后的宽限窗口（后台恢复过渡期）。
  bool _recentlyResumed() {
    final resumedAt = _lastResumeAt;
    if (resumedAt == null) return false;
    return DateTime.now().difference(resumedAt) <
        _resumeNotificationSuppression;
  }

  Future<void> _showPairingDialog() async {
    try {
      await ref.read(authProvider.notifier).beginPairing();
    } catch (error) {
      _pairingDialogShown = false;
      if (mounted) context.showErrorSnackBar(error.toString());
      return;
    }
    if (!mounted) return;
    final result = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (dialogContext) => PairingDialog(dialogContext: dialogContext),
    );
    _pairingDialogShown = false;
    if (result == true && mounted) {
      await _handleAuthenticatedConnection();
    }
  }

  @override
  void dispose() {
    _offlineSubscription?.cancel();
    _unlockStatusTimer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    _pageController.dispose();
    super.dispose();
  }

  /// 连接成功且未认证时弹出配对对话框（带 3 秒冷却，避免重复触发）。
  void _maybeShowPairingDialog(AuthState next) {
    if (_pairingDialogShown || !mounted) return;
    if (!next.isConnected || next.status != AuthStatus.unauthenticated) return;
    final now = DateTime.now();
    if (_lastPairingTime != null &&
        now.difference(_lastPairingTime!) < const Duration(seconds: 3)) {
      return;
    }
    _pairingDialogShown = true;
    _lastPairingTime = now;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _showPairingDialog();
    });
  }

  /// 真实离线事件（宽限期到期）→ 提示一次离线；回前台仍带自动唤醒意图则补发 WOL。
  void _onGenuineOffline() {
    if (!mounted) return;

    final autoWakeWanted =
        _autoWakePending &&
        _appForegrounded &&
        ref.read(settingsProvider).autoWakeOnLaunch &&
        (ref.read(settingsProvider).macAddress?.isNotEmpty ?? false);
    _autoWakePending = false;

    // 回前台后的重连若超过宽限期才判定离线，属于后台恢复过渡期，不弹窗；
    // 仅持续前台期间发生的真实在线→离线切换才提示。
    if (!_recentlyResumed() &&
        _presenceTracker.onOffline(
          foregrounded: _appForegrounded,
          authenticated: ref.read(authProvider).isAuthenticated,
        )) {
      _showNotification('PC已离线', AppColors.error);
    }

    if (autoWakeWanted) {
      unawaited(_maybeWakeAndWait());
    }
  }

  /// 心跳活性在线状态恢复 → 仅当本轮离线已提示过才补一条上线提示。
  void _onPcOnlineChanged(bool pcOnline) {
    if (!mounted) return;
    final previous = _lastPcOnline;
    _lastPcOnline = pcOnline;
    if (previous == pcOnline) return;
    if (pcOnline) {
      _autoWakePending = false;
      if (_presenceTracker.onOnline(foregrounded: _appForegrounded)) {
        _showNotification('PC已上线', AppColors.success);
      }
    }
  }

  Future<void> _checkAutoWake({bool fromResume = false}) async {
    if (_autoWakeInFlight) return;

    final config = ref.read(settingsProvider);
    final action = decideAutoWakeAction(
      enabled: config.autoWakeOnLaunch,
      paired: config.savedToken != null && config.savedToken!.isNotEmpty,
      macConfigured: config.macAddress != null && config.macAddress!.isNotEmpty,
      coldStart: !fromResume,
      pcOnline: ref.read(remoteControlProvider).pcOnline,
    );

    switch (action) {
      case AutoWakeAction.normalConnect:
        // 在线回前台：先静默重连；宽限期后仍未恢复则自动发 WOL。
        if (fromResume &&
            config.autoWakeOnLaunch &&
            (config.macAddress?.isNotEmpty ?? false)) {
          _autoWakePending = true;
        }
        _fallbackNormalConnect();
        return;
      case AutoWakeAction.hintMissingMac:
        if (!fromResume && mounted) {
          context.showWarningSnackBar('自动唤醒已开启，但需要先完成 PC 配对');
        }
        _fallbackNormalConnect();
        return;
      case AutoWakeAction.wakeAndWait:
        _autoWakeInFlight = true;
        try {
          if (fromResume) {
            // 已知离线回前台：立即重连并发送 WOL（60s 冷却）。
            _fallbackNormalConnect();
            await _maybeWakeAndWait();
          } else {
            // 冷启动：立即发唤醒包并与连接并行等待。
            if (mounted) setState(() => _wakeState = WakeState.waking);
            await _sendWakeAndWait();
          }
        } finally {
          _autoWakeInFlight = false;
        }
        return;
    }
  }

  void _fallbackNormalConnect() {
    final config = ref.read(settingsProvider);
    if (config.baseUrl != null) {
      ref.read(authProvider.notifier).connectOnly().then((_) {
        if (mounted && ref.read(authProvider).isAuthenticated) {
          _handleAuthenticatedConnection();
        }
      });
    }
  }

  Future<void> _sendWakeAndWait() async {
    if (_wakeFlowInFlight) return;
    _wakeFlowInFlight = true;
    try {
      // 第一轮唤醒
      if (mounted) setState(() => _wakeState = WakeState.waking);
      await _doWakeOnLan();

      // 跟随电脑开电视：发唤醒后等 5 秒，电脑已在线说明电视已开，不再切电源。
      unawaited(followTvPowerAfterWake(ref));

      var success = await _waitAndHandle(const Duration(minutes: 2));
      if (success) return;

      // 第二轮：重试一次
      if (mounted) setState(() => _wakeState = WakeState.waking);
      await _doWakeOnLan();

      success = await _waitAndHandle(const Duration(minutes: 2));
      if (!success && mounted) setState(() => _wakeState = WakeState.failed);
    } catch (error) {
      debugPrint('WoL failed: $error');
      if (mounted) setState(() => _wakeState = WakeState.failed);
    } finally {
      _wakeFlowInFlight = false;
    }
  }

  Future<bool> _waitAndHandle(Duration timeout) async {
    final success = await _waitForConnection(timeout);
    if (success) {
      await _handleAuthenticatedConnection();
    }
    return success;
  }

  /// 自动唤醒：60s 冷却内不重发 WOL，只继续等待连接。
  Future<void> _maybeWakeAndWait() async {
    final now = DateTime.now();
    if (_lastWolSentAt != null &&
        now.difference(_lastWolSentAt!) < _wolCooldown) {
      final success = await _waitForConnection(const Duration(seconds: 30));
      if (success) await _handleAuthenticatedConnection();
      return;
    }
    _lastWolSentAt = now;
    await _sendWakeAndWait();
  }

  Future<void> _doWakeOnLan() async {
    final remote = ref.read(remoteControlProvider.notifier);
    var config = ref.read(settingsProvider);
    var macAddress = config.macAddress;
    if ((macAddress == null || macAddress.isEmpty) &&
        config.wakeNetworks.isNotEmpty) {
      macAddress = config.wakeNetworks.first.macAddress;
      await ref.read(settingsProvider.notifier).updateMacAddress(macAddress);
      config = ref.read(settingsProvider);
    }
    if (macAddress == null || macAddress.isEmpty) {
      // Older pairings may not have persisted the MAC/network snapshot. If a
      // session is available, refresh it before giving up on the wake action.
      final fetched = await remote.getMacAddress();
      if (fetched != null && fetched.isNotEmpty) {
        await ref.read(settingsProvider.notifier).updateMacAddress(fetched);
        config = ref.read(settingsProvider);
        macAddress = config.macAddress;
      }
    }
    if (macAddress == null || macAddress.isEmpty) {
      throw StateError('MAC address is not configured');
    }
    final prefix = remote.subnetPrefix ?? 24;
    final targetAddress = getSubnetBroadcast(config.pcIpAddress, prefix);
    await WakeOnLan.wake(
      macAddress: macAddress,
      address: targetAddress,
      networks: config.wakeNetworks.isEmpty ? null : config.wakeNetworks,
    );
  }

  Future<bool> _waitForConnection(Duration timeout) async {
    final stopwatch = Stopwatch()..start();
    while (stopwatch.elapsed < timeout) {
      if (!mounted) return false;
      setState(() => _wakeState = WakeState.connecting);
      await ref.read(authProvider.notifier).connectOnly();
      if (ref.read(authProvider).isAuthenticated) {
        return true;
      }
      await Future.delayed(const Duration(seconds: 3));
    }
    return false;
  }

  Future<void> _handleAuthenticatedConnection() async {
    final remote = ref.read(remoteControlProvider.notifier);
    await remote.waitForWebSocket();
    var unlockStatus = await remote.getUnlockStatus();
    await remote.getLockInputStatus();

    // A saved PIN selects credentialed unlock; without one the server runs the
    // confirmation flow. Keep the client side single-shot so a transitional
    // login surface cannot receive duplicates.
    final autoUnlockEnabled = ref.read(settingsProvider).autoUnlockOnWake;
    if (autoUnlockEnabled && unlockStatus?.locked == true) {
      try {
        final password = await ref
            .read(settingsProvider.notifier)
            .readLockPassword();
        unlockStatus =
            await remote.unlockWithMode(lockPassword: password) ?? unlockStatus;
      } catch (error) {
        debugPrint('Automatic unlock failed: $error');
        unlockStatus = await remote.getUnlockStatus() ?? unlockStatus;
      }
    }

    if (!mounted) return;
    // A failed status query must not be treated as an unlocked PC. Keep the
    // conservative locked state so the user can retry unlock instead of
    // receiving a false "ready" indication.
    final locked = unlockStatus?.locked ?? true;
    setState(() {
      _wakeState = locked ? WakeState.locked : WakeState.connected;
    });

    _fetchMacAddress();
    _refreshAllData();
  }

  Future<void> _fetchMacAddress() async {
    final config = ref.read(settingsProvider);
    if (config.macAddress != null && config.macAddress!.isNotEmpty) return;

    final mac = await ref.read(remoteControlProvider.notifier).getMacAddress();
    if (mac != null && mac.isNotEmpty && mounted) {
      await ref.read(settingsProvider.notifier).updateMacAddress(mac);
    }
  }

  Future<void> _fetchWakeNetworks() async {
    final networks = await ref
        .read(remoteControlProvider.notifier)
        .getWakeNetworks();
    if (networks.isNotEmpty && mounted) {
      await ref.read(settingsProvider.notifier).updateWakeNetworks(networks);
      final first = networks.first;
      if ((ref.read(settingsProvider).macAddress?.isNotEmpty ?? false) ==
          false) {
        await ref
            .read(settingsProvider.notifier)
            .updateMacAddress(first.macAddress);
      }
    } else {
      await _fetchMacAddress();
    }
  }

  Future<void> _refreshAllData() async {
    if (!mounted) return;
    final remoteNotifier = ref.read(remoteControlProvider.notifier);
    await Future.wait([
      remoteNotifier.refreshSystemStatus(),
      remoteNotifier.refreshForeground(),
      remoteNotifier.refreshSubnetPrefix(),
      remoteNotifier.loadUnifiedLibrary(force: true),
      remoteNotifier.loadRecentUnifiedLibrary(),
    ]);
  }

  void _onTabTap(int index) {
    if (index != _currentIndex && index == 0) {
      // 切到首页时刷新前台窗口，避免显示过期的活动窗口。
      unawaited(ref.read(remoteControlProvider.notifier).refreshForeground());
    }
    if (index == _currentIndex) return;
    _pageController.animateToPage(
      index,
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
    );
  }

  @override
  Widget build(BuildContext context) {
    final authState = ref.watch(authProvider);
    final pcName = ref.watch(settingsProvider.select((s) => s.pcName));
    final remoteWsState = ref.watch(
      remoteControlProvider.select((s) => s.wsState),
    );
    final tabs = _buildTabs();

    return Scaffold(
      body: DecoratedBox(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [
              AppColors.primary.withValues(alpha: 0.05),
              Colors.transparent,
            ],
            stops: const [0, 0.28],
          ),
        ),
        child: Column(
          children: [
            _StatusBar(
              authState: authState,
              remoteWsState: remoteWsState,
              pcName: pcName,
            ),
            Expanded(
              child: PageView.builder(
                controller: _pageController,
                // 支持左右滑动手势切换 Tab。
                physics: const PageScrollPhysics(),
                itemCount: tabs.length,
                onPageChanged: (index) {
                  setState(() => _currentIndex = index);
                },
                itemBuilder: (context, index) => tabs[index],
              ),
            ),
          ],
        ),
      ),
      bottomNavigationBar: _BottomNav(
        currentIndex: _currentIndex,
        onTap: _onTabTap,
      ),
    );
  }

  void _showNotification(String message, Color color) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Icon(
              message.contains('上线') || message.contains('解锁')
                  ? Icons.check_circle_rounded
                  : Icons.error_rounded,
              color: Colors.white,
              size: 18,
            ),
            const SizedBox(width: 8),
            Text(message),
          ],
        ),
        behavior: SnackBarBehavior.floating,
        backgroundColor: color,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        margin: const EdgeInsets.all(16),
        duration: const Duration(seconds: 2),
      ),
    );
  }

  void _notifyUnlockSuccess() {
    final now = DateTime.now();
    if (_lastUnlockNotificationAt != null &&
        now.difference(_lastUnlockNotificationAt!) <
            const Duration(seconds: 2)) {
      return;
    }
    _lastUnlockNotificationAt = now;
    _showNotification('Windows已解锁', AppColors.success);
  }
}

class _StatusBar extends ConsumerWidget {
  final AuthState authState;
  final WsConnectionState remoteWsState;
  final String? pcName;

  const _StatusBar({
    required this.authState,
    required this.remoteWsState,
    this.pcName,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 14, sigmaY: 14),
        child: Container(
          height: 56 + MediaQuery.of(context).padding.top,
          padding: EdgeInsets.only(
            top: MediaQuery.of(context).padding.top,
            left: 18,
            right: 18,
            bottom: 8,
          ),
          decoration: BoxDecoration(
            color: palette.topbarBackground,
            border: Border(
              bottom: BorderSide(color: palette.line.withValues(alpha: 0.5)),
            ),
          ),
          child: Row(
            children: [
              Container(
                width: 30,
                height: 30,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: AppColors.primary,
                  borderRadius: BorderRadius.circular(9),
                ),
                child: const Text(
                  'Q',
                  style: TextStyle(
                    color: Color(0xFF082219),
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  '轻联',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w700,
                    color: palette.textPrimary,
                  ),
                ),
              ),
              _TopIconButton(
                icon: ref.watch(themeModeProvider) == ThemeMode.dark
                    ? Icons.wb_sunny_outlined
                    : Icons.nightlight_round,
                tooltip: '切换主题',
                onPressed: () => ref.read(themeModeProvider.notifier).toggle(),
              ),
              const SizedBox(width: 4),
              _DevicePill(
                pcName: pcName?.isNotEmpty == true ? pcName! : '客厅电脑',
                connected: remoteWsState == WsConnectionState.connected,
                isAuthenticated: authState.isAuthenticated,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _BottomNav extends StatelessWidget {
  final int currentIndex;
  final ValueChanged<int> onTap;

  const _BottomNav({required this.currentIndex, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 16, sigmaY: 16),
        child: Container(
          decoration: BoxDecoration(
            color: palette.navBackground,
            border: Border(
              top: BorderSide(color: palette.line.withValues(alpha: 0.6)),
            ),
          ),
          child: SafeArea(
            top: false,
            child: NavigationBar(
              selectedIndex: currentIndex,
              onDestinationSelected: onTap,
              destinations: const [
                NavigationDestination(
                  icon: Icon(Icons.home_outlined),
                  selectedIcon: Icon(Icons.home_rounded),
                  label: '首页',
                ),
                NavigationDestination(
                  icon: Icon(Icons.grid_view_outlined),
                  selectedIcon: Icon(Icons.grid_view_rounded),
                  label: '库',
                ),
                NavigationDestination(
                  icon: Icon(Icons.more_horiz_rounded),
                  label: '更多',
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// 顶部栏图标按钮 — 44x44, 10px radius, transparent bg
class _TopIconButton extends StatelessWidget {
  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  const _TopIconButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 44,
      height: 44,
      child: IconButton(
        tooltip: tooltip,
        onPressed: onPressed,
        icon: Icon(icon, size: 19),
        color: context.palette.textMuted,
        style: IconButton.styleFrom(
          backgroundColor: Colors.transparent,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(10),
          ),
        ),
      ),
    );
  }
}

/// 顶部设备名称药丸 — min-height 44, gap 8, radius 10
class _DevicePill extends StatelessWidget {
  final String pcName;
  final bool connected;
  final bool isAuthenticated;

  const _DevicePill({
    required this.pcName,
    required this.connected,
    required this.isAuthenticated,
  });

  @override
  Widget build(BuildContext context) {
    if (!isAuthenticated) return const SizedBox.shrink();
    final palette = context.palette;
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 118),
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: () {},
        child: Container(
          constraints: const BoxConstraints(minHeight: 44),
          padding: const EdgeInsets.only(left: 6, right: 4),
          decoration: BoxDecoration(
            color: Colors.transparent,
            borderRadius: BorderRadius.circular(10),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 7,
                height: 7,
                decoration: BoxDecoration(
                  color: connected ? AppColors.primary : palette.textSubtle,
                  shape: BoxShape.circle,
                  boxShadow: connected
                      ? [
                          BoxShadow(
                            color: AppColors.primary.withValues(alpha: 0.12),
                            blurRadius: 4,
                            spreadRadius: 2,
                          ),
                        ]
                      : null,
                ),
              ),
              const SizedBox(width: 5),
              Flexible(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      pcName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: palette.textPrimary,
                      ),
                    ),
                    Text(
                      connected ? '已连接' : '已关机',
                      style: TextStyle(fontSize: 10, color: palette.textMuted),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 2),
              Icon(
                Icons.keyboard_arrow_down_rounded,
                size: 14,
                color: palette.textSubtle,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
