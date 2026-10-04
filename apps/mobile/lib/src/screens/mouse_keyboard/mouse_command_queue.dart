import 'dart:async';

import 'package:flutter/services.dart';

import '../../providers/input_controller.dart';

/// 鼠标指令串行队列。
///
/// 触控板手势会连续产生移动、滚轮与按键指令；本类把它们排队后串行发送，
/// 保证到达 PC 的顺序与手势顺序一致，并跟踪仍处于按下状态的鼠标按键，
/// 便于断开连接或页面销毁时统一释放。
class MouseCommandQueue {
  MouseCommandQueue({
    required this._runCommand,
    required this._input,
  });

  final Future<bool> Function(Future<void> Function() command) _runCommand;
  final InputController Function() _input;

  Future<void> _tail = Future<void>.value();
  final Set<String> _pressedButtons = <String>{};
  bool _disposed = false;

  /// 发送相对位移。
  void move(int dx, int dy) {
    _enqueue(() => _runCommand(() => _input().sendMouseMoveOrdered(dx, dy)));
  }

  /// 发送滚轮增量。
  void wheel(int delta) {
    _enqueue(() => _runCommand(() => _input().mouseWheel(delta)));
  }

  /// 左键单击。
  void clickLeft() {
    _enqueue(() => _clickWithFeedback(() => _input().mouseButton('left')));
  }

  /// 右键单击。
  void clickRight() {
    _enqueue(() => _clickWithFeedback(() => _input().mouseButton('right')));
  }

  /// 按下左键（拖选开始）。
  void pressLeft() {
    _enqueue(() async {
      final sent = await _runCommand(
        () => _input().mouseButtonState('left', pressed: true),
      );
      if (sent) {
        _pressedButtons.add('left');
        HapticFeedback.selectionClick();
      }
    });
  }

  /// 抬起左键（拖选取消）。
  void releaseLeft() {
    _enqueue(() => _releaseButton('left'));
  }

  /// 拖选结束：先抬起左键，再自动右键呼出上下文菜单。
  void endSelectionDrag() {
    _enqueue(() async {
      await _releaseButton('left');
      await _clickWithFeedback(() => _input().mouseButton('right'));
    });
  }

  /// 释放仍处于按下状态的所有按键（连接断开、页面销毁）。
  void releaseAll() {
    _enqueue(() async {
      await _releaseButton('left');
      await _releaseButton('right');
      _pressedButtons.clear();
    });
  }

  /// 标记队列已废弃，之后不再接受新指令。
  void dispose() {
    _disposed = true;
  }

  void _enqueue(Future<void> Function() command) {
    if (_disposed) return;
    final next = _tail.then((_) => command());
    _tail = next.then<void>(
      (_) {},
      onError: (Object error, StackTrace stack) {},
    );
  }

  Future<void> _clickWithFeedback(Future<void> Function() command) async {
    final sent = await _runCommand(command);
    if (sent) HapticFeedback.selectionClick();
  }

  Future<void> _releaseButton(String button) async {
    final sent = await _runCommand(
      () => _input().mouseButtonState(button, pressed: false),
    );
    if (sent) _pressedButtons.remove(button);
  }
}
