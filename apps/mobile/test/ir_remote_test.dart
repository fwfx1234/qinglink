import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:qinglink_mobile/src/models/ir_remote.dart';
import 'package:qinglink_mobile/src/providers/ir_remote_provider.dart';

void main() {
  test('IR commands and macros round-trip through JSON', () {
    const command = IrCommand(
      id: 'power',
      name: '电源切换',
      carrierHz: 38000,
      patternUs: [9000, 4500, 560, 560],
      repeatCount: 2,
    );
    const profile = IrRemoteProfile(
      id: 'tv',
      name: '客厅电视',
      commands: [command],
    );
    const macro = IrMacro(
      id: 'startup',
      name: '启动流程',
      profileId: 'tv',
      steps: [IrMacroStep(commandId: 'power', delayAfterMs: 800)],
    );

    expect(
      IrRemoteProfile.fromJson(profile.toJson()).toJson(),
      profile.toJson(),
    );
    expect(IrMacro.fromJson(macro.toJson()).toJson(), macro.toJson());
  });

  test(
    'command validation rejects malformed timings and accepts valid data',
    () async {
      SharedPreferences.setMockInitialValues({});
      final notifier = IrRemoteNotifier();
      addTearDown(notifier.dispose);

      expect(
        notifier.validateCommand(
          const IrCommand(
            id: 'bad',
            name: '坏按键',
            carrierHz: 38000,
            patternUs: [9000, 4500, 560],
          ),
        ),
        contains('偶数'),
      );
      expect(
        notifier.validateCommand(
          const IrCommand(
            id: 'good',
            name: '电源',
            carrierHz: 38000,
            patternUs: [9000, 4500, 560, 560],
          ),
        ),
        isNull,
      );
    },
  );
}
