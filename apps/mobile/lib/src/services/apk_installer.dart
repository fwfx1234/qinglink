/// 应用内安装 APK 的原生通道封装（`com.qinglink.mobile/install`）。
///
/// 替代原来的 open_filex：那个插件只回「打开了 / 没权限 / 失败」，系统真正的原因
/// （`INSTALL_PARSE_FAILED_NO_CERTIFICATES` 之类）全被丢掉，用户只能看到一句
/// 「安装包未包含任何证书」。这里除了发起安装，还能在读包信息后先确认
/// 「这包是哪个应用、什么版本、签名跟已装的一致吗」，把问题挡在安装之前。
library;

import 'dart:io';

import 'package:flutter/services.dart';

/// 本应用的包名，用于校验下载到的 APK 不是别的应用。
const qinglinkPackageName = 'com.qinglink.mobile';

/// 一个 APK（待安装或已安装）的身份信息。
class ApkIdentity {
  const ApkIdentity({
    required this.packageName,
    required this.versionCode,
    required this.versionName,
    required this.signerSha256,
  });

  final String packageName;
  final int versionCode;
  final String versionName;
  final List<String> signerSha256;

  /// 指纹前 8 位，界面上够用来对照，又不至于刷屏。
  String get shortSigner {
    if (signerSha256.isEmpty) return '未知';
    final first = signerSha256.first.toUpperCase();
    return first.length <= 8 ? first : first.substring(0, 8);
  }

  static ApkIdentity? fromMap(Object? raw) {
    if (raw is! Map) return null;
    final signers = (raw['signerSha256'] as List?)
            ?.map((item) => '$item')
            .where((item) => item.isNotEmpty)
            .toList() ??
        const <String>[];
    return ApkIdentity(
      packageName: '${raw['packageName'] ?? ''}',
      versionCode: (raw['versionCode'] as num?)?.toInt() ?? 0,
      versionName: '${raw['versionName'] ?? ''}',
      signerSha256: signers,
    );
  }
}

/// 安装结果分类，界面据此决定提示什么。
enum ApkInstallStatus {
  /// 已装成功。
  success,

  /// 系统还没允许本应用「安装未知应用」。
  permissionRequired,

  /// 用户在系统确认框里取消了。
  aborted,

  /// 其它失败，具体原因看 [ApkInstallResult.friendlyMessage]。
  failed,
}

/// 一次安装的结果。
class ApkInstallResult {
  const ApkInstallResult(this.status, {this.message = '', this.code});

  final ApkInstallStatus status;

  /// 系统给的原因（原始英文码，可能为空）。
  final String message;

  /// PackageInstaller 的状态码，排查时用。
  final int? code;

  /// 把系统原因翻成能照着做的一句话。
  String get friendlyMessage {
    final raw = message.trim();
    final lower = raw.toLowerCase();
    if (lower.contains('no_certificates') || lower.contains('failed to collect certificates')) {
      return '安装包不完整或已损坏（系统读不到证书），请重新下载完整文件';
    }
    if (lower.contains('update_incompatible') || lower.contains('signatures do not match')) {
      return '新安装包的签名与已装版本不一致，需先卸载旧版再装';
    }
    if (lower.contains('version_downgrade')) {
      return '版本号低于已安装的版本，系统拒绝降级';
    }
    if (lower.contains('insufficient_storage') || lower.contains('no space')) {
      return '手机存储空间不足';
    }
    if (lower.contains('no_matching_abis')) {
      return '安装包不包含本机 CPU 架构的原生库';
    }
    if (lower.contains('user_restricted') || lower.contains('not allowed')) {
      return '系统限制了本次安装，请检查「安装未知应用」权限';
    }
    if (raw.isEmpty) return '安装没有完成，请重试';
    return '安装失败：$raw';
  }

  /// 带状态码的完整说明，出错时写进错误卡，方便截图反馈。
  String get detailedMessage {
    final codeText = code == null ? '' : '（状态码 $code）';
    return '$friendlyMessage$codeText';
  }
}

/// 原生安装通道。
class ApkInstaller {
  const ApkInstaller();

  static const _channel = MethodChannel('com.qinglink.mobile/install');

  /// 读取一个尚未安装的 APK 的身份信息；文件损坏或不是 APK 时返回 null。
  Future<ApkIdentity?> inspect(String path) async {
    if (!Platform.isAndroid) return null;
    try {
      final raw = await _channel.invokeMethod<Object?>('inspect', {'path': path});
      return ApkIdentity.fromMap(raw);
    } on PlatformException {
      return null;
    }
  }

  /// 读取当前已安装版本的签名信息；未安装返回 null。
  Future<ApkIdentity?> current() async {
    if (!Platform.isAndroid) return null;
    try {
      final raw = await _channel.invokeMethod<Object?>('current');
      return ApkIdentity.fromMap(raw);
    } on PlatformException {
      return null;
    }
  }

  /// 发起安装。会等系统确认框走完才返回最终结果。
  Future<ApkInstallResult> install(String path) async {
    if (!Platform.isAndroid) {
      return const ApkInstallResult(ApkInstallStatus.failed, message: '当前平台不支持应用内安装');
    }
    try {
      final raw = await _channel.invokeMethod<Object?>('install', {'path': path});
      if (raw is! Map) {
        return const ApkInstallResult(ApkInstallStatus.failed, message: '安装器没有返回结果');
      }
      return switch ('${raw['status']}') {
        'success' => const ApkInstallResult(ApkInstallStatus.success),
        'permission' => const ApkInstallResult(ApkInstallStatus.permissionRequired),
        'failed' => ApkInstallResult(
            (raw['code'] as num?)?.toInt() == 3
                ? ApkInstallStatus.aborted
                : ApkInstallStatus.failed,
            message: '${raw['message'] ?? ''}',
            code: (raw['code'] as num?)?.toInt(),
          ),
        _ => const ApkInstallResult(ApkInstallStatus.failed, message: '安装器返回了未知状态'),
      };
    } on PlatformException catch (error) {
      return ApkInstallResult(ApkInstallStatus.failed, message: error.message ?? error.code);
    }
  }

  /// 跳到「安装未知应用」授权页。
  Future<void> openInstallSettings() async {
    if (!Platform.isAndroid) return;
    try {
      await _channel.invokeMethod<void>('openInstallSettings');
    } on PlatformException {
      // 跳不过去也不该让页面崩掉，用户可从系统设置自行进入。
    }
  }

  /// 两组签名是否有交集（多签名场景下不要求顺序一致）。
  static bool sameSigner(ApkIdentity a, ApkIdentity b) {
    if (a.signerSha256.isEmpty || b.signerSha256.isEmpty) return true;
    return a.signerSha256.any((item) => b.signerSha256.contains(item));
  }
}
