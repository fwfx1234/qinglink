import 'dart:typed_data';

/// 鼠标移动二进制帧协议（与服务端 `MouseMoveFrame` 对应）。
///
/// 12 字节小端布局：
/// ```text
/// [0]     protocol version（当前为 1）
/// [1]     event type（1 = mouse move）
/// [2..3]  sequence（u16，用于丢帧检测）
/// [4..5]  dx（i16）
/// [6..7]  dy（i16）
/// [8..11] timestamp_ms（u32，客户端时间戳）
/// ```
class MouseMoveFrame {
  static const int protocolVersion = 1;
  static const int eventMouseMove = 1;
  static const int frameLength = 12;
  static const int maxDelta = 500;

  final int sequence;
  final int dx;
  final int dy;
  final int timestampMs;

  const MouseMoveFrame({
    required this.sequence,
    required this.dx,
    required this.dy,
    required this.timestampMs,
  });

  /// 编码为 12 字节小端帧；dx/dy 会被限制在 [-500, 500]。
  ///
  /// 直接返回 [Uint8List]，避免额外拷贝，适合触控板高频（~60Hz）路径。
  Uint8List encode() {
    final bytes = Uint8List(frameLength);
    final data = ByteData.sublistView(bytes);
    data.setUint8(0, protocolVersion);
    data.setUint8(1, eventMouseMove);
    data.setUint16(2, sequence & 0xffff, Endian.little);
    data.setInt16(4, dx.clamp(-maxDelta, maxDelta), Endian.little);
    data.setInt16(6, dy.clamp(-maxDelta, maxDelta), Endian.little);
    data.setUint32(8, timestampMs & 0xffffffff, Endian.little);
    return bytes;
  }
}
