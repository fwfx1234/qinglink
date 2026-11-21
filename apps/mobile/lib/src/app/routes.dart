import 'package:go_router/go_router.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter/foundation.dart';

import '../screens/control_center_screen.dart';
import '../screens/home_screen.dart';
import '../screens/mouse_keyboard_screen.dart';
import '../screens/connection_diagnose_screen.dart';
import '../screens/pairing_screen.dart';
import '../screens/processes_tab.dart';
import '../screens/scan_folder_screen.dart';
import '../screens/settings_tab.dart';
import '../screens/shortcuts_screen.dart';
import '../screens/ir/tv_remote_screen.dart';
import '../screens/ir/ir_command_screen.dart';
import '../screens/ir/ir_macro_screen.dart';
import '../screens/ir/ir_remote_manage_screen.dart';
import '../providers/auth_provider.dart';
import '../models/auth.dart';

/// Typed route names used by feature pages.
abstract final class AppRoutes {
  static const home = '/';
  static const controlCenter = '/control-center';
  static const mouse = '/mouse';
  static const pairing = '/pairing';
  static const diagnose = '/diagnose';
  static const processes = '/processes';
  static const scanFolder = '/scan-folder';
  static const settings = '/settings';
  static const shortcuts = '/shortcuts';
  static const tvRemote = '/tv-remote';
  static const irCommand = '/ir/command';
  static const irMacro = '/ir/macro';
  static const irManage = '/ir/manage';
}

GoRouter buildAppRouter(Ref ref) => GoRouter(
  refreshListenable: _ProviderRefreshListenable(ref),
  redirect: (context, state) {
    final authenticated = ref.read(connectionProvider).isAuthenticated;
    // 配对页本身就要在未认证状态下进入，不能重定向回首页。
    final publicRoute =
        state.uri.path == AppRoutes.home ||
        state.uri.path == AppRoutes.pairing;
    if (!publicRoute && !authenticated) return AppRoutes.home;
    return null;
  },
  initialLocation: AppRoutes.home,
  routes: [
    GoRoute(
      path: AppRoutes.home,
      name: 'home',
      builder: (_, _) => const HomeScreen(),
    ),
    GoRoute(
      path: AppRoutes.controlCenter,
      name: 'control-center',
      builder: (_, _) => const ControlCenterScreen(),
    ),
    GoRoute(
      path: AppRoutes.mouse,
      name: 'mouse',
      builder: (_, _) => const MouseKeyboardScreen(),
    ),
    GoRoute(
      path: AppRoutes.diagnose,
      name: 'diagnose',
      builder: (_, _) => const ConnectionDiagnoseScreen(),
    ),
    GoRoute(
      path: AppRoutes.pairing,
      name: 'pairing',
      builder: (_, _) => const PairingScreen(),
    ),
    GoRoute(
      path: AppRoutes.processes,
      name: 'processes',
      builder: (_, _) => const ProcessesTab(),
    ),
    GoRoute(
      path: AppRoutes.scanFolder,
      name: 'scan-folder',
      builder: (_, state) => ScanFolderScreen(
        // `game`（默认）或 `app`：决定扫描到的条目加入库后的分类。
        initialKind: state.uri.queryParameters['kind'] == 'app' ? 'app' : 'game',
      ),
    ),
    GoRoute(
      path: AppRoutes.settings,
      name: 'settings',
      builder: (_, _) => const SettingsTab(),
    ),
    GoRoute(
      path: AppRoutes.shortcuts,
      name: 'shortcuts',
      builder: (_, _) => const ShortcutsScreen(),
    ),
    GoRoute(
      path: AppRoutes.tvRemote,
      name: 'tv-remote',
      builder: (_, _) => const TvRemoteScreen(),
    ),
    GoRoute(
      path: AppRoutes.irCommand,
      name: 'ir-command',
      builder: (_, state) => IrCommandScreen(
        profileId: state.uri.queryParameters['profile'] ?? '',
      ),
    ),
    GoRoute(
      path: AppRoutes.irMacro,
      name: 'ir-macro',
      builder: (_, state) => IrMacroScreen(
        profileId: state.uri.queryParameters['profile'] ?? '',
      ),
    ),
    GoRoute(
      path: AppRoutes.irManage,
      name: 'ir-manage',
      builder: (_, _) => const IrRemoteManageScreen(),
    ),
  ],
);

final class _ProviderRefreshListenable extends ChangeNotifier {
  _ProviderRefreshListenable(Ref ref) {
    ref.listen<AuthState>(connectionProvider, (_, _) => notifyListeners());
  }
}
