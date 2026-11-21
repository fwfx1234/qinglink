import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../providers/input_controller.dart';
import '../providers/feature_facades.dart';
import '../providers/remote_command_bus.dart';
import '../providers/remote_input_state.dart';
import '../providers/system_controller.dart';
import '../providers/auth_provider.dart';
import '../providers/settings_provider.dart';
import '../services/touchpad_input_controller.dart';
import '../theme/app_theme.dart';
import '../utils/snackbar_extension.dart';
import '../widgets/active_window.dart';
import '../widgets/app_ui.dart';
import 'mouse_keyboard/clipboard_action_buttons.dart';
import 'mouse_keyboard/connection_status_bar.dart';
import 'mouse_keyboard/keyboard_entry_button.dart';
import 'mouse_keyboard/keyboard_ime_bridge.dart';
import 'mouse_keyboard/keyboard_sheet_presenter.dart';
import 'mouse_keyboard/mouse_command_queue.dart';
import 'mouse_keyboard/touchpad_surface.dart';
import '../app/routes.dart';

@visibleForTesting
double phoneScrollDirection(bool reverse) => reverse ? 1.0 : -1.0;

@visibleForTesting
bool shouldCloseKeyboardSheet({
  required bool sheetOpen,
  required bool imeWasVisible,
  required bool imeVisible,
}) => sheetOpen && imeWasVisible && !imeVisible;

@visibleForTesting
bool shouldForwardEmptyImeBackspace({
  required bool guardWasDeleted,
  required String previousDraft,
  required String committedDraft,
}) => guardWasDeleted && previousDraft.isEmpty && committedDraft.isEmpty;

class MouseKeyboardScreen extends ConsumerStatefulWidget {
  const MouseKeyboardScreen({super.key, this.embedded = false});

  /// 主导航内使用共享顶栏；从其他页面推入时保留独立 AppBar。
  final bool embedded;

  @override
  ConsumerState<MouseKeyboardScreen> createState() =>
      _MouseKeyboardScreenState();
}

class _MouseKeyboardScreenState extends ConsumerState<MouseKeyboardScreen>
    with WidgetsBindingObserver {
  final _textController = TextEditingController(
    text: KeyboardImeBridge.draftGuard,
  );
  final _keyboardFocusNode = FocusNode();
  final _focusHint = ValueNotifier<String?>(null);
  late final TouchpadInputController _touchpad;
  late final MouseCommandQueue _mouseCommands;
  late final KeyboardImeBridge _imeBridge;
  ProviderSubscription<ConnectionConfig>? _settingsSubscription;
  StreamSubscription<Map<String, dynamic>>? _serverEventSubscription;

  /// 当前这一次键盘弹层；非 null 就代表面板开着（或正在打开/关闭）。
  KeyboardSheetPresenter? _keyboardSheet;
  bool _connecting = true;

  /// 页面是否在前台：手机在兜里或锁屏时，PC 上点输入框不该让键盘在后台弹出来。
  bool _appInForeground = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _appInForeground =
        WidgetsBinding.instance.lifecycleState == null ||
        WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed;
    _mouseCommands = MouseCommandQueue(
      runCommand: _runCommand,
      input: () => ref.read(inputControllerProvider),
    );
    _imeBridge = KeyboardImeBridge(
      textController: _textController,
      runCommand: _runCommand,
      input: () => ref.read(inputControllerProvider),
      shouldForwardEmptyBackspace: shouldForwardEmptyImeBackspace,
      isMounted: () => mounted,
    );
    _touchpad = TouchpadInputController(
      onMove: _mouseCommands.move,
      onScroll: _mouseCommands.wheel,
      onTap: _mouseCommands.clickLeft,
      onRightTap: _mouseCommands.clickRight,
      onLongPressRecognized: () => HapticFeedback.selectionClick(),
      onSelectionDragStart: _startSelectionDrag,
      onSelectionDragEnd: _mouseCommands.endSelectionDrag,
      onSelectionDragCancel: _mouseCommands.releaseLeft,
    );
    _touchpad.sensitivity = ref.read(settingsProvider).mouseSensitivity;
    _touchpad.reverseScrollBar = ref
        .read(settingsProvider)
        .reversePhoneScrollBar;
    _settingsSubscription = ref.listenManual<ConnectionConfig>(
      settingsProvider,
      (_, next) {
        final sensitivity = ConnectionConfig.normalizeMouseSensitivity(
          next.mouseSensitivity,
        );
        _touchpad.reverseScrollBar = next.reversePhoneScrollBar;
        if (_touchpad.sensitivity == sensitivity) {
          if (mounted) setState(() {});
          return;
        }
        _touchpad.sensitivity = sensitivity;
        if (mounted) setState(() {});
      },
    );
    ref.listenManual<bool>(connectionReadyProvider, (_, next) {
      if (next != true) _mouseCommands.releaseAll();
    });
    // PC 上的焦点第一次落进输入框时，自动把键盘面板叫起来——手机当输入工具用。
    _serverEventSubscription = ref
        .read(remoteCommandBusProvider)
        .events
        .listen(_handleServerEvent);
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await _connect();
      if (mounted) {
        await ref.read(systemControllerProvider.notifier).refreshForeground();
        await ref.read(systemControllerProvider.notifier).refreshUnlock();
        await ref.read(systemControllerProvider.notifier).refreshLockInput();
      }
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _serverEventSubscription?.cancel();
    _settingsSubscription?.close();
    _mouseCommands.releaseAll();
    _mouseCommands.dispose();
    _touchpad.dispose();
    _keyboardFocusNode.dispose();
    _textController.dispose();
    _focusHint.dispose();
    super.dispose();
  }

  @override
  void didChangeMetrics() {
    final sheet = _keyboardSheet;
    if (sheet == null) return;
    final views = WidgetsBinding.instance.platformDispatcher.views;
    if (views.isEmpty) return;
    final imeVisible = views.first.viewInsets.bottom > 0;
    final wasVisible = sheet.imeVisible;
    sheet.noteImeVisible(imeVisible);
    if (!shouldCloseKeyboardSheet(
          sheetOpen: sheet.isOpen && sheet.imeCloseArmed,
          imeWasVisible: wasVisible,
          imeVisible: imeVisible,
        ) ||
        !mounted) {
      return;
    }
    // 用户已经在下拉/点外部关闭时，退场动画自己会完成，不能再 pop 一次。
    if (sheet.isClosing) return;
    sheet.markClosing();
    // Navigator changes are deferred until the metrics notification finishes.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !identical(_keyboardSheet, sheet)) return;
      if (!sheet.isOpen || !sheet.isTopRoute) return;
      Navigator.of(context).pop();
    });
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _appInForeground = state == AppLifecycleState.resumed;
  }

  Future<void> _connect() async {
    if (mounted) {
      setState(() {
        _connecting = true;
        _error = null;
      });
    }
    final connected = await ref.read(connectionProvider.notifier).waitUntilReady();
    if (!mounted) return;
    setState(() {
      _connecting = false;
      _error = connected ? null : '无法连接到 PC';
    });
  }

  bool get _isReady => ref.read(connectionReadyProvider);

  Future<bool> _runCommand(Future<void> Function() command) async {
    if (!_isReady) return false;
    try {
      await command();
      if (mounted && _error != null) setState(() => _error = null);
      return true;
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
      return false;
    }
  }

  /// 转发一次远程按键（退格/删除等）。
  void _sendKey(String key) {
    unawaited(
      _runCommand(() => ref.read(inputControllerProvider).pressKey(key)),
    );
  }

  /// 发送快捷键面板中的一个快捷键。
  void _sendShortcut(String label) {
    unawaited(
      _runCommand(() => ref.read(inputControllerProvider).sendShortcut(label)),
    );
  }

  /// 进入拖拽：PC 侧左键已经按住（物理按压或上一次拖拽未松开）时不再重复下发按下。
  void _startSelectionDrag() {
    if (ref.read(remoteInputStateProvider).left) return;
    _mouseCommands.pressLeft();
  }

  /// 发送剪贴板动作（复制/粘贴/全选），并给出明确的成功或失败反馈。
  Future<void> _sendClipboardAction(ClipboardAction action) async {
    final sent = await _runCommand(
      () => ref.read(inputControllerProvider).sendShortcut(action.shortcut),
    );
    if (!mounted) return;
    if (sent) {
      unawaited(HapticFeedback.selectionClick());
      context.showSuccessSnackBar(action.doneText);
    } else {
      context.showErrorSnackBar('${action.label}失败：未连接或连接异常');
    }
  }

  void _handleKeyboardChanged(String value) {
    _imeBridge.handleChanged(value);
    if (mounted) setState(() {});
  }

  Future<void> _sendEnterAndClear() async {
    final sent = await _imeBridge.sendEnterAndClear();
    if (sent && mounted) setState(() {});
  }

  /// 输入法回车提交：发送回车后保持输入框焦点，便于连续输入。
  Future<void> _submitKeyboardDraft() async {
    await _sendEnterAndClear();
    if (mounted) _keyboardFocusNode.requestFocus();
  }

  /// 服务端推送：焦点第一次进入 PC 上的输入框时自动打开键盘面板。
  ///
  /// 服务端只在"没焦点 → 有焦点"的那一刻推一次，所以这里不需要任何去重或冷却：
  /// 面板已经开着就不再处理，焦点一直停在同一个输入框里也不会再收到事件。
  /// 自动弹出会抢走用户当前的触控操作，因此设置里可以关掉；关掉后事件照旧
  /// 被丢弃，键盘仍可用控制页的键盘按钮手动打开。
  void _handleServerEvent(Map<String, dynamic> event) {
    if (event['type'] != 'focus_input') return;
    if (!ref.read(settingsProvider).autoOpenKeyboardOnFocus) return;
    if (_keyboardSheet != null || !_appInForeground || !mounted) return;
    // 控制页不在最上层时不弹：用户可能正在看设置或其他页面。
    if (!(ModalRoute.of(context)?.isCurrent ?? true)) return;
    unawaited(_showKeyboardSheet());
  }

  Future<void> _showKeyboardSheet() async {
    // 同步闸门：这里早于任何 await，双击按钮与 focus_input 事件都穿透不了。
    if (!_isReady || _keyboardSheet != null) return;
    final settings = ref.read(settingsProvider);
    final preset = settings.shortcutPresets.firstWhere(
      (item) => item.id == settings.activeShortcutPresetId,
      orElse: () => defaultShortcutPresets.first,
    );
    final sheet = KeyboardSheetPresenter(
      textController: _textController,
      focusNode: _keyboardFocusNode,
      imeBridge: _imeBridge,
      hint: _focusHint,
      shortcuts: preset.shortcuts,
      enabled: _isReady,
      readFocusText: () =>
          ref.read(systemControllerProvider.notifier).readFocusText(),
      onShortcut: _sendShortcut,
      onChanged: _handleKeyboardChanged,
      onSubmitted: _submitKeyboardDraft,
      onBackspace: () => _sendKey('backspace'),
      onDelete: () => _sendKey('delete'),
      onSendEnter: () => unawaited(_sendEnterAndClear()),
      onClipboardAction: (action) => unawaited(_sendClipboardAction(action)),
    );
    _keyboardSheet = sheet;
    if (mounted) setState(() {});
    try {
      await sheet.present(context);
    } finally {
      if (identical(_keyboardSheet, sheet)) {
        _keyboardSheet = null;
        if (mounted) setState(() {});
      }
      sheet.dispose();
      _focusHint.value = null;
      if (mounted) _imeBridge.clearDraft();
    }
  }

  /// 顶栏齿轮：直接进入统一设置页。
  ///
  /// 这里以前弹的是只含「鼠标速度 + 自然滚动」的局部面板，和设置页各占一半，
  /// 用户找不到自动弹出键盘这类开关；现在所有偏好只有设置页一处。
  void _openSettings() {
    context.push(AppRoutes.settings);
  }

  @override
  Widget build(BuildContext context) {
    final isReady = ref.watch(connectionReadyProvider);
    final statusText = isReady
        ? '已连接'
        : _connecting
        ? '正在连接'
        : '未连接';
    final statusColor = isReady ? AppColors.success : AppColors.warning;

    return Scaffold(
      appBar: widget.embedded ? null : _buildStandaloneAppBar(),
      body: SafeArea(
        top: false,
        child: Padding(
          padding: EdgeInsets.fromLTRB(
            AppMetrics.pagePadding,
            widget.embedded ? AppSpacing.md : AppSpacing.sm,
            AppMetrics.pagePadding,
            AppSpacing.md,
          ),
          child: Column(
            children: [
              ConnectionStatusBar(
                statusText: statusText,
                statusColor: statusColor,
                connected: isReady,
                onOpenSettings: _openSettings,
              ),
              if (_error != null) ErrorTextLine(message: _error!),
              const SizedBox(height: AppSpacing.md),
              // 触控区撑满剩余空间，窗口卡与底部三键固定在拇指区。
              Expanded(
                child: TouchpadSection(
                  controller: _touchpad,
                  enabled: isReady,
                  pressedLabel: isReady
                      ? ref.watch(remoteInputStateProvider).label
                      : '',
                ),
              ),
              const SizedBox(height: AppSpacing.md),
              const ActiveWindowSection(),
              const SizedBox(height: AppSpacing.md),
              Row(
                children: [
                  KeyboardEntryButton(
                    compact: true,
                    enabled: isReady && _keyboardSheet == null,
                    onPressed: _showKeyboardSheet,
                  ),
                  const SizedBox(width: AppSpacing.sm),
                  Expanded(
                    child: ClipboardActionButton(
                      action: ClipboardAction.copy,
                      enabled: isReady,
                      onPressed: () =>
                          unawaited(_sendClipboardAction(ClipboardAction.copy)),
                    ),
                  ),
                  const SizedBox(width: AppSpacing.sm),
                  Expanded(
                    child: ClipboardActionButton(
                      action: ClipboardAction.paste,
                      enabled: isReady,
                      onPressed: () => unawaited(
                        _sendClipboardAction(ClipboardAction.paste),
                      ),
                    ),
                  ),
                  const SizedBox(width: AppSpacing.sm),
                  Expanded(
                    flex: 2,
                    child: KeyboardEntryButton(
                      enabled: isReady && _keyboardSheet == null,
                      onPressed: _showKeyboardSheet,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  PreferredSizeWidget _buildStandaloneAppBar() {
    final palette = context.palette;
    return AppBar(
      toolbarHeight: AppMetrics.navHeight,
      centerTitle: true,
      backgroundColor: palette.background,
      surfaceTintColor: Colors.transparent,
      scrolledUnderElevation: 0,
      title: Text(
        '控制',
        style: AppText.sectionTitle.copyWith(color: palette.textPrimary),
      ),
      leadingWidth: AppMetrics.minTap + 12,
      leading: Padding(
        padding: const EdgeInsets.only(left: AppSpacing.md),
        child: IconBtn(
          icon: Icons.chevron_left_rounded,
          tooltip: '返回',
          tonal: true,
          iconSize: 22,
          onPressed: () => Navigator.of(context).maybePop(),
        ),
      ),
      actions: [
        Padding(
          padding: const EdgeInsets.only(right: AppSpacing.md),
          child: IconBtn(
            icon: Icons.refresh_rounded,
            tooltip: '重新连接',
            tonal: true,
            onPressed: _connecting ? null : _connect,
          ),
        ),
      ],
    );
  }
}
