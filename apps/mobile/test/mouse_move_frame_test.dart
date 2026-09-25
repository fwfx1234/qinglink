import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/services/mouse_move_frame.dart';

void main() {
  test('encodes a 12-byte little-endian frame matching the server protocol', () {
    final bytes = MouseMoveFrame(
      sequence: 0x1234,
      dx: -2,
      dy: 2,
      timestampMs: 0x12345678,
    ).encode();

    expect(bytes.length, MouseMoveFrame.frameLength);
    final data = ByteData.sublistView(Uint8List.fromList(bytes));
    expect(data.getUint8(0), 1); // protocol version
    expect(data.getUint8(1), 1); // event: mouse move
    expect(data.getUint16(2, Endian.little), 0x1234);
    expect(data.getInt16(4, Endian.little), -2);
    expect(data.getInt16(6, Endian.little), 2);
    expect(data.getUint32(8, Endian.little), 0x12345678);
  });

  test('clamps dx/dy to the ±500 wire limit', () {
    final bytes = MouseMoveFrame(
      sequence: 0,
      dx: 1000,
      dy: -1000,
      timestampMs: 0,
    ).encode();
    final data = ByteData.sublistView(Uint8List.fromList(bytes));
    expect(data.getInt16(4, Endian.little), MouseMoveFrame.maxDelta);
    expect(data.getInt16(6, Endian.little), -MouseMoveFrame.maxDelta);
  });

  test('wraps sequence and timestamp within their wire widths', () {
    final bytes = MouseMoveFrame(
      sequence: 0x1FFFF,
      dx: 1,
      dy: 1,
      timestampMs: 0x1_0000_1234,
    ).encode();
    final data = ByteData.sublistView(Uint8List.fromList(bytes));
    expect(data.getUint16(2, Endian.little), 0xFFFF);
    expect(data.getUint32(8, Endian.little), 0x1234);
  });
}
