import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/remote_control_provider.dart';
import '../providers/settings_provider.dart';
import '../services/touchpad_input_controller.dart';
import '../services/ws_client.dart';
import '../theme/app_theme.dart';
import '../widgets/active_window.dart';
import '../widgets/app_ui.dart';

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
  // Keep one invisible character in the native text field. Android IMEs do
  // not report Backspace when a field is empty, so deleting this guard lets us
  // forward Backspace even when the remote field is the only field with text.
  static const _keyboardDraftGuard = '\u200B';

  final _textController = TextEditingController(text: _keyboardDraftGuard);
  final _keyboardFocusNode = FocusNode();
  String _bridgedText = '';
  bool _suppressIme = false;
  bool _keyboardSheetOpen = false;
  bool _imeVisible = false;
  Future<void> _imeQueue = Future<void>.value();
  late final TouchpadInputController _touchpad;
  ProviderSubscription<ConnectionConfig>? _settingsSubscription;
  ProviderSubscription<RemoteControlState>? _connectionSubscription;
  WsConnectionState? _lastWsState;
  Future<void> _mouseCommandTail = Future<void>.value();
  final Set<String> _pressedMouseButtons = <String>{};
  bool _mouseCommandsDisposed = false;
  bool _connecting = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _touchpad = TouchpadInputController(
      onMove: _sendMouseMove,
      onScroll: _sendMouseWheel,
      onTap: _sendLeftClick,
      onRightTap: _sendRightClick,
      onLongPressRecognized: () => HapticFeedback.selectionClick(),
      onSelectionDragStart: _sendLeftDown,
      onSelectionDragEnd: _sendSelectionDragEnd,
      onSelectionDragCancel: _sendLeftUp,
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
    _lastWsState = ref.read(remoteControlProvider).wsState;
    _connectionSubscription = ref.listenManual<RemoteControlState>(
      remoteControlProvider,
      (_, next) {
        final previous = _lastWsState;
        _lastWsState = next.wsState;
        if (previous == WsConnectionState.connected &&
            next.wsState != WsConnectionState.connected) {
          _queueMouseRelease();
        }
      },
    );
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await _connect();
      if (mounted) {
        await ref.read(remoteControlProvider.notifier).refreshForeground();
        await ref.read(remoteControlProvider.notifier).getUnlockStatus();
        await ref.read(remoteControlProvider.notifier).getLockInputStatus();
      }
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _settingsSubscription?.close();
    _connectionSubscription?.close();
    _queueMouseRelease();
    _mouseCommandsDisposed = true;
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
    final connected = await ref
        .read(remoteControlProvider.notifier)
        .waitForWebSocket();
    if (!mounted) return;
    setState(() {
      _connecting = false;
      _error = connected ? null : '无法连接到 PC';
    });
  }

  bool get _isReady =>
      ref.read(remoteControlProvider).wsState == WsConnectionState.connected;

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

  void _enqueueMouseCommand(Future<void> Function() command) {
    if (_mouseCommandsDisposed) return;
    final next = _mouseCommandTail.then((_) => command());
    _mouseCommandTail = next.then<void>(
      (_) {},
      onError: (Object error, StackTrace stack) {},
    );
  }

  void _sendMouseMove(int dx, int dy) {
    _enqueueMouseCommand(
      () => _runCommand(
        () => ref
            .read(remoteControlProvider.notifier)
            .sendMouseMoveOrdered(dx, dy),
      ),
    );
  }

  void _sendMouseWheel(int delta) {
    _enqueueMouseCommand(
      () => _runCommand(
        () => ref.read(remoteControlProvider.notifier).mouseWheel(delta),
      ),
    );
  }

  void _sendLeftClick() {
    _enqueueMouseCommand(
      () => _clickWithFeedback(
        () => ref.read(remoteControlProvider.notifier).mouseButton('left'),
      ),
    );
  }

  void _sendRightClick() {
    _enqueueMouseCommand(
      () => _clickWithFeedback(
        () => ref.read(remoteControlProvider.notifier).mouseButton('right'),
      ),
    );
  }

  Future<void> _clickWithFeedback(Future<void> Function() command) async {
    final sent = await _runCommand(command);
    if (sent) HapticFeedback.selectionClick();
  }

  void _sendLeftDown() {
    _enqueueMouseCommand(() async {
      final sent = await _runCommand(
        () => ref
            .read(remoteControlProvider.notifier)
            .mouseButtonState('left', pressed: true),
      );
      if (sent) {
        _pressedMouseButtons.add('left');
        HapticFeedback.selectionClick();
      }
    });
  }

  Future<void> _releaseMouseButton(String button) async {
    final sent = await _runCommand(
      () => ref
          .read(remoteControlProvider.notifier)
          .mouseButtonState(button, pressed: false),
    );
    if (sent) _pressedMouseButtons.remove(button);
  }

  void _sendLeftUp() {
    _enqueueMouseCommand(() => _releaseMouseButton('left'));
  }

  /// 拖选结束：先抬起左键，再自动右键呼出上下文菜单。
  void _sendSelectionDragEnd() {
    _enqueueMouseCommand(() async {
      await _releaseMouseButton('left');
      await _clickWithFeedback(
        () => ref.read(remoteControlProvider.notifier).mouseButton('right'),
      );
    });
  }

  void _queueMouseRelease() {
    if (_mouseCommandsDisposed) return;
    _enqueueMouseCommand(() async {
      await _releaseMouseButton('left');
      await _releaseMouseButton('right');
      _pressedMouseButtons.clear();
    });
  }

  Future<void> _sendText(String text) async {
    if (text.isEmpty) return;
    await _runCommand(
      () => ref.read(remoteControlProvider.notifier).sendText(text),
    );
  }

  void _bridgeImeValue(String value) {
    if (_suppressIme) return;
    final composing = _textController.value.composing;
    final guardWasDeleted = !value.contains(_keyboardDraftGuard);
    // Do not forward uncommitted pinyin/IME composition. Once a candidate is
    // selected, the composing range collapses and the committed text is sent.
    final committedWithGuard = composing.isValid && !composing.isCollapsed
        ? value.replaceRange(composing.start, composing.end, '')
        : value;
    final committed = committedWithGuard.replaceAll(_keyboardDraftGuard, '');
    final forwardEmptyBackspace = shouldForwardEmptyImeBackspace(
      guardWasDeleted: guardWasDeleted,
      previousDraft: _bridgedText,
      committedDraft: committed,
    );
    if (guardWasDeleted) _restoreKeyboardDraftGuard();
    var common = 0;
    while (common < _bridgedText.length &&
        common < committed.length &&
        _bridgedText.codeUnitAt(common) == committed.codeUnitAt(common)) {
      common++;
    }
    final removed = _bridgedText.length - common;
    final added = committed.substring(common);
    _bridgedText = committed;
    if (removed == 0 && added.isEmpty && !forwardEmptyBackspace) return;
    _imeQueue = _imeQueue.then((_) async {
      if (forwardEmptyBackspace) {
        await _runCommand(
          () => ref.read(remoteControlProvider.notifier).pressKey('backspace'),
        );
      }
      for (var i = 0; i < removed; i++) {
        await _runCommand(
          () => ref.read(remoteControlProvider.notifier).pressKey('backspace'),
        );
      }
      if (added.isNotEmpty) await _sendText(added);
    });
  }

  void _restoreKeyboardDraftGuard() {
    final value = _textController.value;
    if (value.text.contains(_keyboardDraftGuard)) return;
    int shiftOffset(int offset) => offset < 0 ? offset : offset + 1;
    _suppressIme = true;
    _textController.value = value.copyWith(
      text: '$_keyboardDraftGuard${value.text}',
      selection: value.selection.copyWith(
        baseOffset: shiftOffset(value.selection.baseOffset),
        extentOffset: shiftOffset(value.selection.extentOffset),
      ),
      composing: value.composing.isValid
          ? TextRange(
              start: shiftOffset(value.composing.start),
              end: shiftOffset(value.composing.end),
            )
          : TextRange.empty,
    );
    _suppressIme = false;
  }

  void _clearKeyboardDraft() {
    if (!mounted) {
      _bridgedText = '';
      return;
    }
    _suppressIme = true;
    _textController.value = const TextEditingValue(
      text: _keyboardDraftGuard,
      selection: TextSelection.collapsed(offset: 1),
    );
    _bridgedText = '';
    _suppressIme = false;
  }

  Future<void> _sendEnterAndClear() async {
    await _imeQueue;
    final sent = await _runCommand(
      () => ref.read(remoteControlProvider.notifier).pressKey('enter'),
    );
    if (sent) {
      _clearKeyboardDraft();
      if (mounted) setState(() {});
    }
  }

  void _changeSensitivity(double value) {
    setState(() => _touchpad.sensitivity = value);
  }

  void _saveSensitivity(double value) {
    unawaited(ref.read(settingsProvider.notifier).saveMouseSensitivity(value));
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
          return Padding(
            padding: EdgeInsets.fromLTRB(
              16,
              12,
              16,
              12 + MediaQuery.viewInsetsOf(sheetContext).bottom,
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SizedBox(
                  height: 40,
                  child: ListView.separated(
                    scrollDirection: Axis.horizontal,
                    itemCount: preset.shortcuts.length,
                    separatorBuilder: (_, _) => const SizedBox(width: 6),
                    itemBuilder: (_, index) {
                      final label = preset.shortcuts[index];
                      return ActionChip(
                        label: Text(label),
                        onPressed: _isReady
                            ? () => unawaited(
                                _runCommand(
                                  () => ref
                                      .read(remoteControlProvider.notifier)
                                      .sendShortcut(label),
                                ),
                              )
                            : null,
                      );
                    },
                  ),
                ),
                const SizedBox(height: 14),
                TextField(
                  controller: _textController,
                  focusNode: _keyboardFocusNode,
                  autofocus: true,
                  minLines: 1,
                  maxLines: 3,
                  maxLength: 4096,
                  onChanged: (value) {
                    _bridgeImeValue(value);
                    if (mounted) setState(() {});
                  },
                  textInputAction: TextInputAction.send,
                  // Keep the keyboard sheet active after sending so the user
                  // can enter another message without reopening it.
                  onEditingComplete: () {},
                  onSubmitted: (_) async {
                    await _sendEnterAndClear();
                    if (mounted) _keyboardFocusNode.requestFocus();
                  },
                  decoration: const InputDecoration(
                    hintText: '输入要发送的文字',
                    counterText: '',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 10),
                // Explicit keys so text that was not entered through this IME
                // bridge (typed on the PC, pasted, previous session) can still be
                // deleted, and Enter can be sent without composing text.
                Row(
                  children: [
                    Expanded(
                      child: _SheetKeyButton(
                        icon: Icons.backspace_outlined,
                        label: '退格',
                        enabled: _isReady,
                        onPressed: () => _runCommand(
                          () => ref
                              .read(remoteControlProvider.notifier)
                              .pressKey('backspace'),
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: _SheetKeyButton(
                        icon: Icons.delete_outline_rounded,
                        label: '删除',
                        enabled: _isReady,
                        onPressed: () => _runCommand(
                          () => ref
                              .read(remoteControlProvider.notifier)
                              .pressKey('delete'),
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: _SheetKeyButton(
                        icon: Icons.keyboard_return_rounded,
                        label: '回车',
                        enabled: _isReady,
                        onPressed: () => unawaited(_sendEnterAndClear()),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          );
        },
      );
    } finally {
      _keyboardSheetOpen = false;
      _imeVisible = false;
      if (mounted) _keyboardFocusNode.unfocus();
      _clearKeyboardDraft();
    }
  }

  Future<void> _showMouseSettings() async {
    var sensitivity = _touchpad.sensitivity;
    var reverseScroll = ref.read(settingsProvider).reversePhoneScrollBar;

    await showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (sheetContext) => StatefulBuilder(
        builder: (context, setSheetState) {
          final palette = context.palette;
          return SafeArea(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(20, 4, 20, 24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      Text(
                        '鼠标速度',
                        style: AppText.body.copyWith(
                          color: palette.textPrimary,
                        ),
                      ),
                      const Spacer(),
                      StatusTag('${sensitivity.toStringAsFixed(1)}x'),
                    ],
                  ),
                  Slider(
                    value: sensitivity,
                    min: TouchpadInputController.minSensitivity,
                    max: TouchpadInputController.maxSensitivity,
                    divisions: 14,
                    onChanged: (value) {
                      setSheetState(() => sensitivity = value);
                      _changeSensitivity(value);
                    },
                    onChangeEnd: _saveSensitivity,
                  ),
                  const SizedBox(height: 4),
                  AppListRow(
                    title: '自然滚动',
                    onTap: null,
                    showChevron: false,
                    trailing: AppToggle(
                      value: reverseScroll,
                      onChanged: (value) {
                        setSheetState(() => reverseScroll = value);
                        ref
                            .read(settingsProvider.notifier)
                            .setReversePhoneScrollBar(value);
                      },
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final connectionState = ref.watch(
      remoteControlProvider.select((state) => state.wsState),
    );
    final isReady = connectionState == WsConnectionState.connected;
    final statusText = isReady
        ? '已连接'
        : _connecting || connectionState == WsConnectionState.connecting
        ? '正在连接'
        : connectionState == WsConnectionState.reconnecting
        ? '正在重连'
        : '未连接';
    final statusColor = isReady ? AppColors.success : AppColors.warning;

    return Scaffold(
      appBar: widget.embedded
          ? null
          : AppBar(
              title: const Text('控制'),
              actions: [
                IconButton(
                  tooltip: '重新连接',
                  onPressed: _connecting ? null : _connect,
                  icon: const Icon(Icons.refresh_rounded),
                ),
              ],
            ),
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
                      Padding(
                        padding: const EdgeInsets.only(left: 4, right: 2),
                        child: Row(
                          children: [
                            StatusDot(color: statusColor, glow: isReady),
                            const SizedBox(width: 7),
                            Text(
                              statusText,
                              style: AppText.label.copyWith(
                                color: statusColor,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            const Spacer(),
                            IconBtn(
                              icon: Icons.tune_rounded,
                              tooltip: '鼠标设置',
                              size: 34,
                              iconSize: 18,
                              onPressed: _showMouseSettings,
                            ),
                          ],
                        ),
                      ),
                      if (_error != null)
                        Align(
                          alignment: Alignment.centerLeft,
                          child: Padding(
                            padding: const EdgeInsets.only(left: 4, top: 2),
                            child: Text(
                              _error!,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: AppText.meta.copyWith(
                                color: AppColors.error,
                              ),
                            ),
                          ),
                        ),
                      const SizedBox(height: 4),
                      Expanded(child: _buildTouchpad(isReady)),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 8),
              const ActiveWindowSection(),
              const SizedBox(height: 8),
              SizedBox(
                width: double.infinity,
                height: 46,
                child: Tooltip(
                  message: '输入文字',
                  child: OutlinedButton.icon(
                    onPressed: isReady ? _showKeyboardSheet : null,
                    icon: const Icon(Icons.keyboard_outlined, size: 18),
                    label: const Text('输入文字'),
                    style: OutlinedButton.styleFrom(
                      foregroundColor: palette.textPrimary,
                      backgroundColor: palette.surface2,
                      side: BorderSide(color: palette.line),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(AppRadius.md),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildTouchpad(bool enabled) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final scrollZoneWidth = _scrollZoneWidthFor(constraints.maxWidth);
        return _TouchpadSurface(
          enabled: enabled,
          scrollZoneWidth: scrollZoneWidth,
          onPointerDown: (event) => _touchpad.pointerDown(
            event.pointer,
            event.localPosition,
            inScrollZone:
                event.localPosition.dx >=
                constraints.maxWidth - scrollZoneWidth,
          ),
          onPointerMove: (event) =>
              _touchpad.pointerMove(event.pointer, event.localPosition),
          onPointerUp: (event) => _touchpad.pointerUp(event.pointer),
          onPointerCancel: (event) => _touchpad.pointerCancel(event.pointer),
        );
      },
    );
  }

  double _scrollZoneWidthFor(double surfaceWidth) {
    return (surfaceWidth * 0.12).clamp(36.0, 44.0);
  }
}

class _TouchpadSurface extends StatelessWidget {
  const _TouchpadSurface({
    required this.enabled,
    required this.scrollZoneWidth,
    required this.onPointerDown,
    required this.onPointerMove,
    required this.onPointerUp,
    required this.onPointerCancel,
  });

  final bool enabled;
  final double scrollZoneWidth;
  final PointerDownEventListener onPointerDown;
  final PointerMoveEventListener onPointerMove;
  final PointerUpEventListener onPointerUp;
  final PointerCancelEventListener onPointerCancel;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Semantics(
      label: enabled ? '触控板' : '触控板，未连接',
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: enabled ? onPointerDown : null,
        onPointerMove: enabled ? onPointerMove : null,
        onPointerUp: enabled ? onPointerUp : null,
        onPointerCancel: enabled ? onPointerCancel : null,
        child: DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [palette.touchpadStart, palette.touchpadEnd],
            ),
            borderRadius: BorderRadius.circular(AppRadius.lg),
            border: Border.all(
              color: enabled ? palette.touchpadBorder : palette.line,
            ),
          ),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(AppRadius.lg),
            child: Stack(
              children: [
                Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.touch_app_rounded,
                        size: 24,
                        color: enabled ? palette.textSubtle : palette.line,
                      ),
                      const SizedBox(height: 6),
                      Text(
                        '滑动移动',
                        style: AppText.meta.copyWith(
                          fontSize: 12,
                          color: enabled ? palette.textSubtle : palette.line,
                        ),
                      ),
                    ],
                  ),
                ),
                if (enabled && scrollZoneWidth > 0)
                  Positioned(
                    top: 0,
                    bottom: 0,
                    right: 0,
                    width: scrollZoneWidth,
                    child: IgnorePointer(
                      child: Semantics(
                        label: '右侧滚动区',
                        child: Container(
                          decoration: BoxDecoration(
                            color: palette.surface2.withValues(alpha: .82),
                            border: Border(
                              left: BorderSide(color: palette.line),
                            ),
                          ),
                          child: Center(
                            child: Icon(
                              Icons.unfold_more_rounded,
                              size: 20,
                              color: palette.textSubtle,
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// A wide key button inside the remote keyboard sheet.
class _SheetKeyButton extends StatelessWidget {
  const _SheetKeyButton({
    required this.icon,
    required this.label,
    required this.enabled,
    required this.onPressed,
  });

  final IconData icon;
  final String label;
  final bool enabled;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return OutlinedButton.icon(
      onPressed: enabled ? onPressed : null,
      icon: Icon(icon, size: 18),
      label: Text(label),
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(0, 46),
        foregroundColor: palette.textPrimary,
        side: BorderSide(color: palette.line),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
    );
  }
}
