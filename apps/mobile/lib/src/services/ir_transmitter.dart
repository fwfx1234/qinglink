import 'package:flutter/services.dart';

class IrCapability {
  const IrCapability({
    required this.supported,
    this.carrierFrequencies = const [],
    this.carrierRanges = const [],
  });

  final bool supported;

  /// Legacy/sample frequencies exposed by older Android implementations.
  final List<int> carrierFrequencies;

  /// Inclusive [minHz, maxHz] ranges reported by ConsumerIrManager.
  final List<List<int>> carrierRanges;

  bool supportsCarrier(int hz) {
    if (carrierRanges.isNotEmpty) {
      return carrierRanges.any(
        (range) => range.length >= 2 && hz >= range[0] && hz <= range[1],
      );
    }
    return carrierFrequencies.isEmpty ||
        carrierFrequencies.any((frequency) => (frequency - hz).abs() <= 1000);
  }
}

class IrTransmitterException implements Exception {
  const IrTransmitterException(this.code, this.message);
  final String code;
  final String message;
  @override
  String toString() => message;
}

class IrTransmitter {
  const IrTransmitter();

  static const _channel = MethodChannel('com.qinglink.mobile/ir');

  Future<IrCapability> capabilities() async {
    try {
      final result = await _channel.invokeMapMethod<String, dynamic>(
        'getCapabilities',
      );
      if (result == null) return const IrCapability(supported: false);
      return IrCapability(
        supported: result['supported'] as bool? ?? false,
        carrierFrequencies:
            (result['carrierFrequencies'] as List<dynamic>? ?? const [])
                .whereType<num>()
                .map((value) => value.toInt())
                .toList(growable: false),
        carrierRanges: (result['carrierRanges'] as List<dynamic>? ?? const [])
            .whereType<List<dynamic>>()
            .map(
              (range) => range
                  .whereType<num>()
                  .map((value) => value.toInt())
                  .toList(growable: false),
            )
            .where((range) => range.length >= 2)
            .toList(growable: false),
      );
    } on MissingPluginException {
      return const IrCapability(supported: false);
    } on PlatformException catch (error) {
      throw IrTransmitterException(error.code, error.message ?? '无法读取红外能力');
    }
  }

  Future<void> transmit({
    required int carrierHz,
    required List<int> patternUs,
    int repeatCount = 1,
  }) async {
    try {
      await _channel.invokeMethod<void>('transmit', {
        'carrierHz': carrierHz,
        'patternUs': patternUs,
        'repeatCount': repeatCount,
      });
    } on MissingPluginException {
      throw const IrTransmitterException('UNSUPPORTED', '当前平台不支持手机红外发射');
    } on PlatformException catch (error) {
      throw IrTransmitterException(error.code, error.message ?? '红外发送失败');
    }
  }
}
