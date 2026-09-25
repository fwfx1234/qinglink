import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:qinglink_mobile/main.dart';
import 'package:qinglink_mobile/src/providers/ir_remote_provider.dart';
import 'package:qinglink_mobile/src/providers/settings_provider.dart';
import 'package:qinglink_mobile/src/screens/home_tab.dart';
import 'package:qinglink_mobile/src/screens/mouse_keyboard_screen.dart';
import 'package:qinglink_mobile/src/services/ir_transmitter.dart';

class _SupportedIrTransmitter extends IrTransmitter {
  const _SupportedIrTransmitter();

  @override
  Future<IrCapability> capabilities() async =>
      const IrCapability(supported: true);
}

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const ProviderScope(child: QingLinkApp()));
    await tester.pump();
  });

  testWidgets('offline home keeps the full feature surface and TV remote', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          settingsProvider.overrideWith(
            (_) => SettingsNotifier(loadPersisted: false),
          ),
          irRemoteProvider.overrideWith(
            (_) =>
                const IrRemoteNotifier(transmitter: _SupportedIrTransmitter()),
          ),
        ],
        child: MaterialApp(home: HomeTab(onGoToMore: () {})),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('连接电脑'), findsOneWidget);
    expect(find.text('快捷操作'), findsOneWidget);
    expect(find.text('进程'), findsOneWidget);
    expect(find.text('休眠'), findsOneWidget);
    expect(find.text('关机'), findsOneWidget);
    expect(find.text('音量'), findsOneWidget);
    expect(find.text('电源'), findsOneWidget);
    expect(find.text('TV'), findsOneWidget);
    expect(find.text('控制'), findsOneWidget);

    await tester.tap(find.text('TV'));
    await tester.pumpAndSettle();
    expect(find.text('雷鸟/TCL 电视'), findsOneWidget);
    expect(find.text('遥控器'), findsOneWidget);
  });

  testWidgets('offline home offers wake when a MAC address is available', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          settingsProvider.overrideWith(
            (_) => SettingsNotifier(
              initialState: const ConnectionConfig(
                macAddress: 'AA:BB:CC:DD:EE:FF',
              ),
              loadPersisted: false,
            ),
          ),
        ],
        child: MaterialApp(home: HomeTab(onGoToMore: () {})),
      ),
    );
    await tester.pump();

    expect(find.text('唤醒电脑'), findsOneWidget);
    expect(find.text('快捷操作'), findsOneWidget);
  });

  testWidgets('mouse controls stay fixed and usable across phone layouts', (
    WidgetTester tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(320, 560));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await tester.pumpWidget(
      const ProviderScope(child: MaterialApp(home: MouseKeyboardScreen())),
    );
    await tester.pump();

    expect(find.text('正在连接'), findsOneWidget);
    expect(find.byType(Scrollable), findsNothing);
    // 竖屏主界面只保留输入文字；鼠标点击、右键和拖选由触控板手势完成。
    expect(find.byTooltip('右键'), findsNothing);
    expect(find.byTooltip('双击'), findsNothing);
    expect(find.byTooltip('输入文字'), findsOneWidget);
    expect(find.byTooltip('远端退格'), findsNothing);
    expect(find.byTooltip('鼠标设置'), findsOneWidget);
    expect(find.byType(Slider), findsNothing);
    expect(tester.takeException(), isNull);

    await tester.binding.setSurfaceSize(const Size(800, 360));
    await tester.pump();
    expect(find.byType(Scrollable), findsNothing);
    // 横屏：触控板仍占据主区域，滚轮作为内部区域处理。
    expect(find.byIcon(Icons.touch_app_rounded), findsOneWidget);
    expect(tester.takeException(), isNull);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump(const Duration(seconds: 13));
  });
}
