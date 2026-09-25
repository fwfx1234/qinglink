import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/services/ws_event_adapter.dart';

void main() {
  test('parses audio output devices and default marker', () {
    final outputs = WsEventAdapter.parseAudioOutputs({
      'type': 'audio_outputs',
      'data': {
        'devices': [
          {
            'id': 'speaker-1',
            'name': '扬声器',
            'is_default': true,
          },
          {
            'id': 'headphones-1',
            'name': '蓝牙耳机',
            'is_default': false,
          },
        ],
      },
    });

    expect(outputs, hasLength(2));
    expect(outputs!.first.name, '扬声器');
    expect(outputs.first.isDefault, isTrue);
    expect(outputs.last.isDefault, isFalse);
  });

  test('rejects an invalid audio output response', () {
    expect(
      WsEventAdapter.parseAudioOutputs({'type': 'audio_outputs'}),
      isNull,
    );
  });
}
