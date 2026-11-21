import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'clipboard_action_buttons.dart';
import 'keyboard_ime_bridge.dart';
import 'keyboard_input_sheet.dart';
import '../../widgets/app_sheet.dart';

/// 一次键盘弹层的生命周期。
///
/// 只负责"这一次弹层"：打开路由、入场动画结束后请求焦点、系统输入法没弹起来时的补发、
/// 迟到文本的回填与丢弃，以及给宿主用的 `isOpen` / `isClosing` 状态。
///
/// 焦点为什么必须等动画结束：`EditableText` 建立输入连接要求新鲜的 keyboard token，
/// 而在弹层滑入的第一帧请求焦点会让 Android 在转场期间收到 `showSoftInput` 并静默丢弃它，
/// Flutter 不会重试——表现就是"有光标、没键盘"。用户再点一次输入框走的是
/// `requestKeyboard()` 的已聚焦分支，所以一定能弹出来。
class KeyboardSheetPresenter {
  KeyboardSheetPresenter({
    required this.textController,
    required this.focusNode,
    required this.imeBridge,
    required this.hint,
    required this.shortcuts,
    required this.enabled,
    required this.readFocusText,
    required this.onShortcut,
    required this.onChanged,
    required this.onSubmitted,
    required this.onBackspace,
    required this.onDelete,
    required this.onSendEnter,
    required this.onClipboardAction,
  });

  /// 焦点请求之后每隔多久确认一次输入法是否真的起来了。
  static const _imeProbeInterval = Duration(milliseconds: 400);

  /// 最多补发几次 `TextInput.show`：连接已建立时它等价于"再点一下输入框"。
  static const _maxImeShowRetries = 2;

  final TextEditingController textController;
  final FocusNode focusNode;
  final KeyboardImeBridge imeBridge;
  final ValueNotifier<String?> hint;
  final List<String> shortcuts;
  final bool enabled;
  final Future<String> Function() readFocusText;
  final ValueChanged<String> onShortcut;
  final ValueChanged<String> onChanged;
  final Future<void> Function() onSubmitted;
  final VoidCallback onBackspace;
  final VoidCallback onDelete;
  final VoidCallback onSendEnter;
  final ValueChanged<ClipboardAction> onClipboardAction;

  bool _disposed = false;
  bool _userInteracted = false;
  bool _imeCloseArmed = false;
  int _imeShowRetries = 0;
  Timer? _imeProbe;
  ModalRoute<dynamic>? _route;

  /// 本次面板是否还开着。
  bool isOpen = false;

  /// 退场动画是否已经开始（用户下拉 / 点外部 / 输入法收起）。
  bool isClosing = false;

  /// 最近一次观测到的输入法可见状态。
  bool imeVisible = false;

  /// 只有本次会话请求过焦点之后，才允许"输入法收起 = 关面板"。
  ///
  /// 否则新面板入场时，上一次会话残留的输入法正在收起，会被误判成本次键盘被收起。
  bool get imeCloseArmed => _imeCloseArmed;

  /// 面板是否仍是栈顶路由；被别的弹层盖住时不要 pop。
  bool get isTopRoute => _route?.isCurrent ?? false;

  /// 同步置位后立刻开始滑出，网络往返放到后台——绝不阻塞在 `readFocusText()` 上。
  Future<void> present(BuildContext context) {
    isOpen = true;
    isClosing = false;
    imeVisible = false;
    _imeCloseArmed = false;
    _userInteracted = false;
    _imeShowRetries = 0;
    hint.value = null;
    imeBridge.clearDraft();
    final closed = showAppSheet<void>(
      context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: _buildSheet,
    );
    unawaited(_adoptRemoteTextLater());
    return closed;
  }

  /// 入场动画结束：此时请求焦点，`showSoftInput` 不会再撞上转场。
  void requestFieldFocus() {
    if (_disposed || !isOpen || isClosing) return;
    focusNode.requestFocus();
    imeVisible = _readImeInset();
    _imeCloseArmed = true;
    _scheduleImeProbe();
  }

  /// 路由重新落位（下拉回弹）：撤销"正在关闭"，必要时补一次焦点。
  void noteRouteSettled() {
    if (_disposed || !isOpen) return;
    isClosing = false;
    if (!focusNode.hasFocus) requestFieldFocus();
  }

  /// 开始关闭：同步置位，阻止再次进入并取消输入法探测。
  void markClosing() {
    if (isClosing) return;
    isClosing = true;
    _imeProbe?.cancel();
  }

  /// 记录输入法可见状态；确认输入法已经起来后就不必再补发。
  void noteImeVisible(bool visible) {
    imeVisible = visible;
    if (visible) _imeProbe?.cancel();
  }

  void dispose() {
    _disposed = true;
    isOpen = false;
    isClosing = false;
    _imeProbe?.cancel();
  }

  Widget _buildSheet(BuildContext sheetContext) {
    _route = ModalRoute.of(sheetContext);
    return KeyboardSheetRouteWatcher(
      onEnterCompleted: noteRouteSettled,
      onDismissStarted: markClosing,
      child: KeyboardInputSheet(
        textController: textController,
        focusNode: focusNode,
        enabled: enabled,
        shortcuts: shortcuts,
        onShortcut: onShortcut,
        onChanged: _handleUserChanged,
        onSubmitted: () => unawaited(_submit()),
        onBackspace: onBackspace,
        onDelete: onDelete,
        onSendEnter: onSendEnter,
        onClipboardAction: onClipboardAction,
        focusHint: hint,
      ),
    );
  }

  /// 用户动过输入框之后，迟到的 PC 原文一律丢弃，不覆盖已经敲进去的字。
  void _handleUserChanged(String value) {
    _userInteracted = true;
    onChanged(value);
  }

  Future<void> _submit() async {
    _userInteracted = true;
    await onSubmitted();
  }

  /// 读取 PC 焦点输入框的已有文本并回填；面板先滑出，文本后到。
  Future<void> _adoptRemoteTextLater() async {
    String text;
    try {
      text = await readFocusText();
    } catch (error) {
      debugPrint('readFocusText failed: $error');
      text = '';
    }
    if (_disposed || !isOpen || isClosing || _userInteracted) return;
    imeBridge.adoptRemoteText(text);
    hint.value = text.isEmpty ? '未读取到 PC 输入框内容，将按空内容输入' : null;
  }

  void _scheduleImeProbe() {
    _imeProbe?.cancel();
    if (_disposed || isClosing || imeVisible) return;
    if (_imeShowRetries > _maxImeShowRetries) return;
    _imeProbe = Timer(_imeProbeInterval, _probeIme);
  }

  void _probeIme() {
    if (_disposed || !isOpen || isClosing || imeVisible) return;
    if (!focusNode.hasFocus) return;
    if (_readImeInset()) {
      imeVisible = true;
      return;
    }
    // 连接没建立时补 show 没有意义，只有 Android 需要这条兜底。
    if (defaultTargetPlatform != TargetPlatform.android) return;
    if (_imeShowRetries >= _maxImeShowRetries) return;
    _imeShowRetries++;
    unawaited(SystemChannels.textInput.invokeMethod<void>('TextInput.show'));
    _scheduleImeProbe();
  }

  bool _readImeInset() {
    final views = WidgetsBinding.instance.platformDispatcher.views;
    return views.isNotEmpty && views.first.viewInsets.bottom > 0;
  }
}
