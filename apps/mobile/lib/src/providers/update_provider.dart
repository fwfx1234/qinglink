/// 更新流程状态管理：检查 → 下载 → 安装。
///
/// 用 Riverpod 的 `AsyncNotifier` 承载状态；版本号来自 [appVersionProvider]，避免写死。
/// 下载在后台 async 进行，进度由 repository 回调写回状态，界面 watch 本 provider 即可。
library;

import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import '../data/repositories/update_repository.dart';
import '../models/update_info.dart';
import '../services/apk_installer.dart';
import 'app_version_provider.dart';

/// 更新流程阶段。
enum UpdatePhase {
  idle,
  checking,
  upToDate,
  available,
  downloading,
  ready,
  error,
}

/// 更新状态快照。
class UpdateState {
  const UpdateState({
    this.phase = UpdatePhase.idle,
    this.currentVersion = '',
    this.release,
    this.progress = 0,
    this.downloadedBytes = 0,
    this.totalBytes = 0,
    this.error,
    this.apkPath,
    this.apkIdentity,
    this.apkBytes = 0,
  });

  final UpdatePhase phase;
  final String currentVersion;
  final ReleaseInfo? release;
  final int progress;
  final int downloadedBytes;
  final int totalBytes;
  final String? error;
  final String? apkPath;

  /// 已下载安装包的身份（包名 / 版本号 / 签名者），由原生侧读出。
  /// 界面用它把「装的是哪一包」显示出来，避免再出现「装错了包却无从判断」。
  final ApkIdentity? apkIdentity;

  /// 已下载安装包的字节数。
  final int apkBytes;

  bool get busy =>
      phase == UpdatePhase.checking || phase == UpdatePhase.downloading;
  bool get hasUpdate => switch (phase) {
        UpdatePhase.available ||
        UpdatePhase.downloading ||
        UpdatePhase.ready =>
          true,
        _ => false,
      };

  /// 能否在应用内下载安装：只有 Android 有可侧载的 APK，iOS 只能跳发布页。
  bool get canInstall => Platform.isAndroid && (release?.hasApk ?? false);

  UpdateState copyWith({
    UpdatePhase? phase,
    String? currentVersion,
    ReleaseInfo? release,
    bool clearRelease = false,
    int? progress,
    int? downloadedBytes,
    int? totalBytes,
    String? error,
    bool clearError = false,
    String? apkPath,
    ApkIdentity? apkIdentity,
    int? apkBytes,
    bool clearApk = false,
  }) {
    return UpdateState(
      phase: phase ?? this.phase,
      currentVersion: currentVersion ?? this.currentVersion,
      release: clearRelease ? null : (release ?? this.release),
      progress: progress ?? this.progress,
      downloadedBytes: downloadedBytes ?? this.downloadedBytes,
      totalBytes: totalBytes ?? this.totalBytes,
      error: clearError ? null : (error ?? this.error),
      apkPath: apkPath ?? this.apkPath,
      apkIdentity: clearApk ? null : (apkIdentity ?? this.apkIdentity),
      apkBytes: clearApk ? 0 : (apkBytes ?? this.apkBytes),
    );
  }
}

/// 更新仓库 provider，便于测试替换。
final updateRepositoryProvider = Provider<UpdateRepository>((ref) {
  return UpdateRepository();
});

/// 原生安装通道 provider，便于测试替换。
final apkInstallerProvider = Provider<ApkInstaller>((ref) => const ApkInstaller());

/// 更新状态管理器。
class UpdateNotifier extends AsyncNotifier<UpdateState> {
  @override
  Future<UpdateState> build() async {
    final version = await ref.watch(appVersionProvider.future);
    return UpdateState(currentVersion: version.displayVersion);
  }

  UpdateRepository get _repo => ref.read(updateRepositoryProvider);

  ApkInstaller get _installer => ref.read(apkInstallerProvider);

  /// 手动/自动检查更新。
  ///
  /// 版本号直接从 [appVersionProvider] 读取：build() 尚未完成时 state 为空，
  /// 用空字符串比较会把任意 release 都当成新版本。
  /// 发现新版本立即后台下载；安装仍需用户手动点击（会中断当前操作）。
  Future<void> check() async {
    final version = await ref.read(appVersionProvider.future);
    // 首帧就自动检查时 build() 可能还没完成；先拿到初始状态再改，否则 _set 会空转。
    final current = await _ensureState(version.displayVersion);
    _set((s) => s.copyWith(phase: UpdatePhase.checking, clearError: true));
    try {
      final release = await _repo.check(current.currentVersion);
      if (release == null) {
        _set((s) => s.copyWith(phase: UpdatePhase.upToDate, clearRelease: true));
      } else {
        _set((s) => s.copyWith(phase: UpdatePhase.available, release: release));
        // iOS 没有 APK 资产，download() 会直接跳过，保留提示与发布页按钮。
        await download();
      }
    } catch (error) {
      _set((s) => s.copyWith(phase: UpdatePhase.error, error: '$error'));
    }
  }

  /// 下载 APK 并校验。非 Android（如 iOS）不下载安装包，只保留发布页入口。
  Future<void> download() async {
    final state0 = state.valueOrNull;
    final release = state0?.release;
    if (release == null) return;
    if (!Platform.isAndroid) {
      _set((s) => s.copyWith(phase: UpdatePhase.available, clearError: true));
      return;
    }
    if (!release.hasApk) {
      // 有 APK 但没有清单：不能跳过校验就安装，给出去发布页的口子。
      _set((s) => s.copyWith(
            phase: UpdatePhase.available,
            error: release.apkUrl.isEmpty
                ? '该版本没有 Android 安装包，请前往发布页查看'
                : '该版本缺少校验清单，请前往发布页手动下载',
          ));
      return;
    }
    if (state0!.phase == UpdatePhase.downloading) return;
    _set((s) => s.copyWith(
          phase: UpdatePhase.downloading,
          progress: 0,
          downloadedBytes: 0,
          totalBytes: 0,
          clearError: true,
          clearApk: true,
        ));
    try {
      final sums = await _repo.fetchChecksums(release.checksumUrl);
      final expected = sums[release.apkName.toLowerCase()] ??
          (throw '校验清单缺少 APK 条目：${release.apkName}');
      final dir = await getApplicationDocumentsDirectory();
      final updatesDir = Directory('${dir.path}${Platform.pathSeparator}updates');
      await updatesDir.create(recursive: true);
      final file = File('${updatesDir.path}${Platform.pathSeparator}${release.apkName}');
      // 同名旧文件先删：断点残留的文件长度可能看着"像个 APK"，但缺尾部签名块，
      // 交给安装器只会得到一句「安装包未包含任何证书」。
      if (await file.exists()) await file.delete();
      await _repo.downloadApk(release.apkUrl, file, (downloaded, total) {
        final percent = total <= 0 ? 0 : (downloaded * 100 / total).round();
        _set((s) => s.copyWith(
              downloadedBytes: downloaded,
              totalBytes: total,
              progress: percent > 100 ? 100 : percent,
            ));
      });
      await _verifyDownloadedApk(file, release, expected);
      final identity = await _installer.inspect(file.path);
      final bytes = await file.length();
      _set((s) => s.copyWith(
            phase: UpdatePhase.ready,
            apkPath: file.path,
            apkIdentity: identity,
            apkBytes: bytes,
          ));
    } catch (error) {
      _set((s) => s.copyWith(
            phase: UpdatePhase.error,
            error: '$error',
            clearApk: true,
          ));
    }
  }

  /// 安装前把「这包到底对不对」查清楚，而不是丢给系统安装器去猜。
  ///
  /// 依次核对：字节数 → SHA-256 → 包名 → versionCode → 签名者。任何一步不符都停在
  /// 这里给一句能照做的话，因为系统对「坏包」只会统一回「安装包未包含任何证书」，
  /// 完全看不出是没下完、下错了版本，还是签名换了。
  Future<void> _verifyDownloadedApk(
    File file,
    ReleaseInfo release,
    String expectedSha256,
  ) async {
    final length = await file.length();
    if (release.apkSize > 0 && length != release.apkSize) {
      throw '安装包没有下载完整（${_mb(length)} / ${_mb(release.apkSize)} MB），请重新下载';
    }
    await _repo.verifyChecksum(file, expectedSha256);

    final identity = await _installer.inspect(file.path);
    if (identity == null) {
      throw '安装包无法解析，文件可能已损坏，请重新下载';
    }
    if (identity.packageName != qinglinkPackageName) {
      throw '安装包的应用标识不对（${identity.packageName}），已停止安装';
    }
    final expectedCode = ParsedVersion.parse(release.version).build;
    if (expectedCode > 0 && identity.versionCode != expectedCode) {
      throw '安装包版本是 ${identity.versionCode}，与发布的 $expectedCode 不一致，请重新下载';
    }
    final installed = await _installer.current();
    if (installed != null && !ApkInstaller.sameSigner(installed, identity)) {
      throw '新安装包的签名与当前已装版本不一致，无法覆盖安装；请先卸载旧版再装';
    }
  }

  static String _mb(int bytes) => (bytes / 1024 / 1024).toStringAsFixed(1);

  /// 触发安装：Android 走原生 PackageInstaller；其它平台返回 unsupported。
  ///
  /// 会等系统确认框走完才返回：成功即已装好（进程随即被替换），失败则把系统原因
  /// 写进 `state.error`，页面能直接显示，不必再对着「没有证书」猜。
  Future<InstallOutcome> install() async {
    final apkPath = state.valueOrNull?.apkPath;
    if (apkPath == null || !Platform.isAndroid) {
      return InstallOutcome.unsupported;
    }
    final result = await _installer.install(apkPath);
    switch (result.status) {
      case ApkInstallStatus.success:
        return InstallOutcome.launched;
      case ApkInstallStatus.permissionRequired:
        return InstallOutcome.permissionDenied;
      case ApkInstallStatus.aborted:
        _set((s) => s.copyWith(clearError: true));
        return InstallOutcome.aborted;
      case ApkInstallStatus.failed:
        _set((s) => s.copyWith(error: result.detailedMessage));
        return InstallOutcome.failed;
    }
  }

  /// 跳到系统「安装未知应用」授权页。
  Future<void> openInstallSettings() => _installer.openInstallSettings();

  /// 保证 state 已有值：build() 未完成时先等它；build 抛错就用兜底版本号建一个，
  /// 让页面至少能显示版本与检查结果，而不是一直卡在加载态。
  Future<UpdateState> _ensureState(String fallbackVersion) async {
    final existing = state.valueOrNull;
    if (existing != null) return existing;
    try {
      return await future;
    } catch (_) {
      final seeded = UpdateState(currentVersion: fallbackVersion);
      state = AsyncData(seeded);
      return seeded;
    }
  }

  void _set(UpdateState Function(UpdateState) update) {
    final current = state.valueOrNull;
    if (current == null) return;
    state = AsyncData(update(current));
  }
}

/// 安装动作结果。
enum InstallOutcome {
  /// 安装已完成，或系统正在替换本进程。
  launched,

  /// 还没被允许「安装未知应用」。
  permissionDenied,

  /// 用户取消了系统确认框。
  aborted,

  /// 失败，原因见 state.error。
  failed,

  /// 当前平台不支持应用内安装。
  unsupported,
}

/// 更新状态 provider。
final updateProvider =
    AsyncNotifierProvider<UpdateNotifier, UpdateState>(UpdateNotifier.new);
