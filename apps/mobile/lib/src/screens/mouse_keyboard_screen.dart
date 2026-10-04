import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/input_controller.dart';
import '../providers/feature_facades.dart';
import '../providers/system_controller.dart';
import '../providers/auth_provider.dart';
import '../providers/settings_provider.dart';
import '../services/touchpad_input_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/active_window.dart';
import '../widgets/app_ui.dart';
import 'mouse_keyboard/connection_status_bar.dart';
import 'mouse_keyboard/keyboard_entry_button.dart';
import 'mouse_keyboard/keyboard_ime_bridge.dart';
import 'mouse_keyboard/keyboard_input_sheet.dart';
import 'mouse_keyboard/mouse_command_queue.dart';
import 'mouse_keyboard/mouse_settings_sheet.dart';
import 'mouse_keyboard/touchpad_surface.dart';

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
  late final TouchpadInputController _touchpad;
  late final MouseCommandQueue _mouseCommands;
  late final KeyboardImeBridge _imeBridge;
  ProviderSubscription<ConnectionConfig>? _settingsSubscription;
  bool _keyboardSheetOpen = false;
  bool _imeVisible = false;
  bool _connecting = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
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
      onSelectionDragStart: _mouseCommands.pressLeft,
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
    _settingsSubscription?.close();
    _mouseCommands.releaseAll();
    _mouseCommands.dispose();
    _touchpad.dispose();
    _keyboardFocusNode.dispose();
    _textController.dispose();
    super.dispose();
  }

  @override
  void didChangeMetrics() {
    final views = WidgetsBinding.instance.platformDispatcher.views;
    if (views.isEmpty) return;
    final imeVisible = views.first.viewInsets.bottom > 0;
    final wasVisible = _imeVisible;
    _imeVisible = imeVisible;
    if (!shouldCloseKeyboardSheet(
          sheetOpen: _keyboardSheetOpen,
          imeWasVisible: wasVisible,
          imeVisible: imeVisible,
        ) ||
        !mounted) {
      return;
    }
    // Navigator changes are deferred until the metrics notification finishes.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_keyboardSheetOpen) return;
      Navigator.of(context).pop();
    });
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

  void _changeSensitivity(double value) {
    setState(() => _touchpad.sensitivity = value);
  }

  Future<void> _showKeyboardSheet() async {
    if (!_isReady) return;
    _keyboardSheetOpen = true;
    _imeVisible = false;
    try {
      await showModalBottomSheet<void>(
        context: context,
        isScrollControlled: true,
        useSafeArea: true,
        builder: (sheetContext) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (sheetContext.mounted) _keyboardFocusNode.requestFocus();
          });
          final settings = ref.read(settingsProvider);
          final preset = settings.shortcutPresets.firstWhere(
            (item) => item.id == settings.activeShortcutPresetId,
            orElse: () => defaultShortcutPresets.first,
          );
          return KeyboardInputSheet(
            textController: _textController,
            focusNode: _keyboardFocusNode,
            enabled: _isReady,
            shortcuts: preset.shortcuts,
            onShortcut: _sendShortcut,
            onChanged: _handleKeyboardChanged,
            onSubmitted: () => unawaited(_submitKeyboardDraft()),
            onBackspace: () => _sendKey('backspace'),
            onDelete: () => _sendKey('delete'),
            onSendEnter: () => unawaited(_sendEnterAndClear()),
          );
        },
      );
    } finally {
      _keyboardSheetOpen = false;
      _imeVisible = false;
      if (mounted) _keyboardFocusNode.unfocus();
      _imeBridge.clearDraft();
    }
  }

  Future<void> _showMouseSettings() async {
    final sensitivity = _touchpad.sensitivity;
    final reverseScroll = ref.read(settingsProvider).reversePhoneScrollBar;

    await showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (_) => MouseSettingsSheet(
        sensitivity: sensitivity,
        reverseScroll: reverseScroll,
        onSensitivityChanged: _changeSensitivity,
      ),
    );
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
          padding: EdgeInsets.fromLTRB(14, widget.embedded ? 12 : 4, 14, 10),
          child: Column(
            children: [
              Expanded(
                child: AppCard(
                  radius: AppRadius.lg,
                  padding: const EdgeInsets.all(8),
                  child: Column(
                    children: [
                      ConnectionStatusBar(
                        statusText: statusText,
                        statusColor: statusColor,
                        connected: isReady,
                        onOpenSettings: _showMouseSettings,
                      ),
                      if (_error != null) ErrorTextLine(message: _error!),
                      const SizedBox(height: 4),
                      Expanded(
                        child: TouchpadSection(
                          controller: _touchpad,
                          enabled: isReady,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 8),
              const ActiveWindowSection(),
              const SizedBox(height: 8),
              KeyboardEntryButton(
                enabled: isReady,
                onPressed: _showKeyboardSheet,
              ),
            ],
          ),
        ),
      ),
    );
  }

  PreferredSizeWidget _buildStandaloneAppBar() {
    return AppBar(
      title: const Text('控制'),
      actions: [
        IconButton(
          tooltip: '重新连接',
          onPressed: _connecting ? null : _connect,
          icon: const Icon(Icons.refresh_rounded),
        ),
      ],
    );
  }
}
