/// 移动端主壳：底部导航与 Tab 装配、页面切换，以及生命周期到各编排器的接线。
///
/// 其余职责分别落在同目录的独立库里：顶部栏在 `home_top_bar.dart`，底部导航在
/// `home_bottom_nav.dart`，连接/配对与连接后刷新在 `home_connection.dart`，唤醒
/// 流程在 `home_wake_flow.dart`，提示投递在 `home_notifications.dart`。本文件只
/// 保留 `HomeScreen`、它必须持有的页面状态，以及把状态接到子组件上的动作。
library;

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/auth.dart';
import '../models/system_status.dart';
import '../models/wake_state.dart';
import '../providers/auth_provider.dart';
import '../providers/feature_facades.dart';
import '../providers/settings_provider.dart';
import '../providers/system_controller.dart';
import 'home_bottom_nav.dart';
import 'home_connection.dart';
import 'home_notifications.dart';
import 'home_tab.dart';
import 'home_top_bar.dart';
import 'home_wake_flow.dart';
import 'library_tab.dart';
import 'more_tab.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen>
    with WidgetsBindingObserver {
  /// 三个 Tab 的页面标题，顶栏只展示“我在哪”。
  static const _tabTitles = ['首页', '库', '更多'];

  int _currentIndex = 0;
  WakeState _wakeState = WakeState.idle;
  late final PageController _pageController;
  bool _appForegrounded = true;
  Timer? _unlockStatusTimer;
  late final HomeNotificationPresenter _notifications;
  late final HomeConnectionCoordinator _connection;
  late final HomeWakeFlow _wake;

  Widget _buildTab(int index) {
    return RepaintBoundary(
      child: switch (index) {
        0 => HomeTab(
          wakeState: _wakeState,
          onGoToLibrary: () => _onTabTap(1),
          onGoToMore: () => _onTabTap(2),
          onWake: _wake.sendWakeAndWait,
        ),
        1 => const LibraryTab(),
        _ => const MoreTab(),
      },
    );
  }

  @override
  void initState() {
    super.initState();
    _pageController = PageController();
    _notifications = HomeNotificationPresenter(contextOf: _contextOrNull);
    _connection = HomeConnectionCoordinator(
      ref,
      isMounted: () => mounted,
      isForegrounded: () => _appForegrounded,
      contextOf: _contextOrNull,
      onWakeStateChanged: _setWakeState,
      notifications: _notifications,
    );
    _wake = HomeWakeFlow(
      ref,
      isMounted: () => mounted,
      onWakeStateChanged: _setWakeState,
      onConnected: _connection.handleAuthenticatedConnection,
    );
    WidgetsBinding.instance.addObserver(this);

    _bindConnectionListeners();

    _unlockStatusTimer = Timer.periodic(const Duration(seconds: 5), (_) {
      if (!mounted || !ref.read(connectionReadyProvider)) return;
      unawaited(ref.read(systemControllerProvider.notifier).refreshUnlock());
    });

    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await ref.read(settingsProvider.notifier).loaded;
      if (mounted) {
        // App 启动只尝试连接，不再发送 Wake-on-LAN。唤醒电脑必须由用户
        // 点击首页状态卡或设置页的 Wake-on-LAN 操作显式触发。
        _connection.fallbackNormalConnect();
      }
    });
  }

  /// 订阅认证状态、在线状态与解锁状态：事件转交连接编排器与提示投递器。
  void _bindConnectionListeners() {
    // 配对弹窗监听认证状态；上线/离线通知监听心跳活性在线状态与真实离线事件。
    ref.listenManual<AuthState>(connectionProvider, (previous, next) {
      _connection.maybeShowPairingDialog(next);
    });
    ref.listenManual<bool>(
      connectionReadyProvider,
      (_, next) => _connection.onPcOnlineChanged(next),
    );
    ref.listenManual<bool>(connectionReadyProvider, (
      _,
      next,
    ) {
      // 连接成功后结束回前台重连过渡期。
      if (next) {
        // 连接成功后补齐 MAC（WoL 需要）并刷新锁屏状态，避免用旧状态判断解锁。
        unawaited(_connection.fetchWakeNetworks());
        unawaited(ref.read(systemControllerProvider.notifier).refreshUnlock());
      }
    });
    ref.listenManual<UnlockStatus?>(
      systemControllerProvider.select((s) => s.unlock),
      (previous, next) {
        _notifications.handleUnlockStatus(
          previous,
          next,
          foregrounded: _appForegrounded,
        );
      },
    );
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _appForegrounded = state == AppLifecycleState.resumed;
    if (state == AppLifecycleState.resumed && mounted) {
      // 后台→前台：清除未结算的离线/上线回合，回前台后的重连结果不再弹窗。
      // 若会话在后台未被挂起（仍连接中），则无需过渡期，直接按真实状态判定。
      _connection.resetPresenceEpisode();
      if (ref.read(connectionReadyProvider)) {
        unawaited(ref.read(systemControllerProvider.notifier).refreshForeground());
      }
      _connection.fallbackNormalConnect();
      // 后台期间的解锁提示在回前台后补发。
      _notifications.flushPendingUnlock();
    }
  }

  /// 宿主挂载时的 BuildContext；未挂载时为 null，供编排器与提示投递器安全取用。
  BuildContext? _contextOrNull() => mounted ? context : null;

  /// 唤醒状态落地：宿主已卸载时忽略，避免对已卸载的 State 调用 `setState`。
  void _setWakeState(WakeState state) {
    if (!mounted) return;
    setState(() => _wakeState = state);
  }

  @override
  void dispose() {
    _unlockStatusTimer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    _pageController.dispose();
    super.dispose();
  }

  void _onTabTap(int index) {
    if (index != _currentIndex && index == 0) {
      // 切到首页时刷新前台窗口，避免显示过期的活动窗口。
      unawaited(ref.read(systemControllerProvider.notifier).refreshForeground());
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
    return Scaffold(
      body: SafeArea(
        bottom: false,
        child: Column(
          children: [
            HomeTopBar(title: _tabTitles[_currentIndex]),
            Expanded(
              child: PageView.builder(
                controller: _pageController,
                // 支持左右滑动手势切换 Tab。
                physics: const PageScrollPhysics(),
                itemCount: 3,
                onPageChanged: (index) {
                  setState(() => _currentIndex = index);
                },
                itemBuilder: (context, index) => _buildTab(index),
              ),
            ),
          ],
        ),
      ),
      bottomNavigationBar: HomeBottomNav(
        currentIndex: _currentIndex,
        onTap: _onTabTap,
      ),
    );
  }
}
