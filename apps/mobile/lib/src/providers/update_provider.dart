/// 更新流程状态管理：检查 → 下载 → 安装。
///
/// 用 Riverpod 的 `AsyncNotifier` 承载状态；版本号来自 [appVersionProvider]，避免写死。
/// 下载在后台 async 进行，进度由 repository 回调写回状态，界面 watch 本 provider 即可。
library;

import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:open_filex/open_filex.dart';
import 'package:path_provider/path_provider.dart';

import '../data/repositories/update_repository.dart';
import '../models/update_info.dart';
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
  });

  final UpdatePhase phase;
  final String currentVersion;
  final ReleaseInfo? release;
  final int progress;
  final int downloadedBytes;
  final int totalBytes;
  final String? error;
  final String? apkPath;

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
    );
  }
}

/// 更新仓库 provider，便于测试替换。
final updateRepositoryProvider = Provider<UpdateRepository>((ref) {
  return UpdateRepository();
});

/// 更新状态管理器。
class UpdateNotifier extends AsyncNotifier<UpdateState> {
  @override
  Future<UpdateState> build() async {
    final version = await ref.watch(appVersionProvider.future);
    return UpdateState(currentVersion: version.displayVersion);
  }

  UpdateRepository get _repo => ref.read(updateRepositoryProvider);

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
        ));
    try {
      final sums = await _repo.fetchChecksums(release.checksumUrl);
      final expected = sums[release.apkName.toLowerCase()] ??
          (throw '校验清单缺少 APK 条目：${release.apkName}');
      final dir = await getApplicationDocumentsDirectory();
      final updatesDir = Directory('${dir.path}${Platform.pathSeparator}updates');
      await updatesDir.create(recursive: true);
      final file = File('${updatesDir.path}${Platform.pathSeparator}${release.apkName}');
      await _repo.downloadApk(release.apkUrl, file, (downloaded, total) {
        final percent = total <= 0 ? 0 : (downloaded * 100 / total).round();
        _set((s) => s.copyWith(
              downloadedBytes: downloaded,
              totalBytes: total,
              progress: percent > 100 ? 100 : percent,
            ));
      });
      await _repo.verifyChecksum(file, expected);
      _set((s) => s.copyWith(phase: UpdatePhase.ready, apkPath: file.path));
    } catch (error) {
      _set((s) => s.copyWith(phase: UpdatePhase.error, error: '$error'));
    }
  }

  /// 触发安装：Android 打开 APK；其它平台返回 unsupported，由界面打开发布页。
  Future<InstallOutcome> install() async {
    final apkPath = state.valueOrNull?.apkPath;
    if (apkPath == null || !Platform.isAndroid) {
      return InstallOutcome.unsupported;
    }
    final result = await OpenFilex.open(apkPath);
    return switch (result.type) {
      ResultType.done => InstallOutcome.launched,
      ResultType.permissionDenied => InstallOutcome.permissionDenied,
      _ => InstallOutcome.failed,
    };
  }

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
enum InstallOutcome { launched, permissionDenied, failed, unsupported }

/// 更新状态 provider。
final updateProvider =
    AsyncNotifierProvider<UpdateNotifier, UpdateState>(UpdateNotifier.new);
