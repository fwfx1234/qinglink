import 'dart:async';

import 'package:flutter/material.dart';

import '../../providers/input_controller.dart';

/// 判断「IME 删掉了守卫字符、且草稿前后都为空」时是否要转发一次退格。
///
/// 规则实现在 `mouse_keyboard_screen.dart`（既有测试入口保持在该库），
/// 这里只声明依赖注入用的函数类型。
typedef ForwardEmptyImeBackspaceRule =
    bool Function({
      required bool guardWasDeleted,
      required String previousDraft,
      required String committedDraft,
    });

/// 软键盘 IME 桥。
///
/// 原生输入框的内容变化并不等于已提交文本（拼音等在 composing 区间内），
/// 本类把输入框差异翻译成远程按键指令：删除的字符发退格，新增的文本发文本。
class KeyboardImeBridge {
  KeyboardImeBridge({
    required this._textController,
    required this._runCommand,
    required this._input,
    required this._shouldForwardEmptyBackspace,
    required this._isMounted,
  });

  // Keep one invisible character in the native text field. Android IMEs do
  // not report Backspace when a field is empty, so deleting this guard lets us
  // forward Backspace even when the remote field is the only field with text.
  static const String draftGuard = '\u200B';

  final TextEditingController _textController;
  final Future<bool> Function(Future<void> Function() command) _runCommand;
  final InputController Function() _input;
  final ForwardEmptyImeBackspaceRule _shouldForwardEmptyBackspace;
  final bool Function() _isMounted;

  String _bridgedText = '';
  bool _suppressIme = false;
  Future<void> _imeQueue = Future<void>.value();

  /// 处理输入框内容变化，必要时排队发送远程按键。
  void handleChanged(String value) {
    if (_suppressIme) return;
    final composing = _textController.value.composing;
    final guardWasDeleted = !value.contains(draftGuard);
    // Do not forward uncommitted pinyin/IME composition. Once a candidate is
    // selected, the composing range collapses and the committed text is sent.
    final committedWithGuard = composing.isValid && !composing.isCollapsed
        ? value.replaceRange(composing.start, composing.end, '')
        : value;
    final committed = committedWithGuard.replaceAll(draftGuard, '');
    final forwardEmptyBackspace = _shouldForwardEmptyBackspace(
      guardWasDeleted: guardWasDeleted,
      previousDraft: _bridgedText,
      committedDraft: committed,
    );
    if (guardWasDeleted) _restoreDraftGuard();
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
        await _runCommand(() => _input().pressKey('backspace'));
      }
      for (var i = 0; i < removed; i++) {
        await _runCommand(() => _input().pressKey('backspace'));
      }
      if (added.isNotEmpty) await _sendText(added);
    });
  }

  /// 清空草稿并把输入框恢复到只含守卫字符的状态。
  void clearDraft() {
    if (!_isMounted()) {
      _bridgedText = '';
      return;
    }
    _suppressIme = true;
    _textController.value = const TextEditingValue(
      text: draftGuard,
      selection: TextSelection.collapsed(offset: 1),
    );
    _bridgedText = '';
    _suppressIme = false;
  }

  /// 等待排队中的按键发完，再发送回车；返回是否发送成功。
  Future<bool> sendEnterAndClear() async {
    await _imeQueue;
    final sent = await _runCommand(() => _input().pressKey('enter'));
    if (sent) clearDraft();
    return sent;
  }

  Future<void> _sendText(String text) async {
    if (text.isEmpty) return;
    await _runCommand(() => _input().sendText(text));
  }

  void _restoreDraftGuard() {
    final value = _textController.value;
    if (value.text.contains(draftGuard)) return;
    int shiftOffset(int offset) => offset < 0 ? offset : offset + 1;
    _suppressIme = true;
    _textController.value = value.copyWith(
      text: '$draftGuard${value.text}',
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
}
