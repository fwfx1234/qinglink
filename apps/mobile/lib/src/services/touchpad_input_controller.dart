import 'dart:async';
import 'dart:ui';

class TouchpadInputController {
  static const double defaultSensitivity = 4;
  static const double minSensitivity = 1;
  static const double maxSensitivity = 8;
  static const int maxMouseDelta = 500;
  static const Duration longPressDuration = Duration(milliseconds: 400);
  static const Duration _minimumFlushInterval = Duration(milliseconds: 16);
  static const double _tapSlop = 6;
  static const double _pixelsPerScrollStep = 14;

  TouchpadInputController({
    required this.onMove,
    required this.onScroll,
    required this.onTap,
    required this.onRightTap,
    required this.onLongPressRecognized,
    required this.onSelectionDragStart,
    required this.onSelectionDragEnd,
    required this.onSelectionDragCancel,
    Duration Function()? elapsed,
    this.longPressDelay = longPressDuration,
  }) : _injectedElapsed = elapsed;

  final void Function(int dx, int dy) onMove;
  final void Function(int delta) onScroll;
  final void Function() onTap;
  final void Function() onRightTap;

  /// 长按到点（未位移）时触发一次，用于轻触感提示；
  /// 之后「继续拖动=选取、直接松手=右键」。
  final void Function() onLongPressRecognized;

  /// 拖选开始：PC 端左键按下。
  final void Function() onSelectionDragStart;

  /// 拖选结束（发生过实际拖动）：PC 端左键抬起 + 自动右键。
  final void Function() onSelectionDragEnd;

  /// 拖选被取消（第二指落下/系统取消）：仅左键抬起。
  final void Function() onSelectionDragCancel;

  final Duration longPressDelay;

  /// 测试可注入的单调时钟；默认使用 [Stopwatch]，不受系统时钟跳变影响。
  final Duration Function()? _injectedElapsed;
  final Stopwatch _stopwatch = Stopwatch()..start();
  final Map<int, Offset> _pointers = {};
  final Map<int, Offset> _pointerOrigins = {};
  final Map<int, bool> _scrollZonePointers = {};

  Timer? _flushTimer;
  Timer? _longPressTimer;
  Duration _lastFlushAt = Duration.zero;
  Offset? _lastCentroid;
  double _pendingX = 0;
  double _pendingY = 0;
  double _pendingScroll = 0;
  bool _tapEligible = false;

  /// 长按已识别（位移未超阈值）：松手=右键，继续拖动=选取。
  bool _longPressRecognized = false;

  /// 拖选进行中（PC 端左键已按下）。
  bool _selecting = false;

  double _sensitivity = defaultSensitivity;
  bool reverseScrollBar = false;

  double get sensitivity => _sensitivity;

  set sensitivity(double value) {
    _sensitivity = value.clamp(minSensitivity, maxSensitivity).toDouble();
  }

  void pointerDown(int pointer, Offset position, {bool inScrollZone = false}) {
    _pointers[pointer] = position;
    _pointerOrigins[pointer] = position;
    _scrollZonePointers[pointer] = inScrollZone;

    // 第二指落下：取消待识别长按或进行中的拖选，回到常规多指逻辑。
    if (_pointers.length >= 2) {
      _cancelLongPressTimer();
      if (_selecting) {
        _selecting = false;
        onSelectionDragCancel();
      }
      _longPressRecognized = false;
      _tapEligible = false;
      _resetGestureBaseline();
      return;
    }

    if (inScrollZone) {
      // 滚动区不启用长按/拖选；点击与按住松手仍发左键。
      _tapEligible = true;
      _resetGestureBaseline();
      return;
    }

    _tapEligible = true;
    _startLongPressTimer();
    _resetGestureBaseline();
  }

  void pointerMove(int pointer, Offset position) {
    final previous = _pointers[pointer];
    if (previous == null) return;
    _pointers[pointer] = position;

    if (_pointers.length == 1) {
      final delta = position - previous;
      final totalDelta = position - (_pointerOrigins[pointer] ?? position);

      if (_longPressRecognized) {
        // 长按已识别：位移超过阈值才真正进入拖选（左键按下）。
        if (!_selecting && totalDelta.distance > _tapSlop) {
          _selecting = true;
          onSelectionDragStart();
        }
        if (_selecting) {
          _pendingX += delta.dx * _sensitivity;
          _pendingY += delta.dy * _sensitivity;
        }
        _scheduleFlush();
        return;
      }

      if (totalDelta.distance > _tapSlop) {
        _tapEligible = false;
        _cancelLongPressTimer();
      }
      if (_scrollZonePointers[pointer] ?? false) {
        // 右缘滚动区：单指上下拖动即滚动，内容跟随手指方向。
        final direction = reverseScrollBar ? 1.0 : -1.0;
        _pendingScroll += direction * delta.dy / _pixelsPerScrollStep;
      } else {
        _pendingX += delta.dx * _sensitivity;
        _pendingY += delta.dy * _sensitivity;
      }
    } else {
      final centroid = _centroid();
      final previousCentroid = _lastCentroid;
      if (previousCentroid != null) {
        // Finger movement follows natural scrolling: content follows the drag.
        _pendingScroll -=
            (centroid.dy - previousCentroid.dy) / _pixelsPerScrollStep;
      }
      _lastCentroid = centroid;
    }
    _scheduleFlush();
  }

  void pointerUp(int pointer) {
    final hadSinglePointer =
        _pointers.length == 1 && _pointers.containsKey(pointer);
    _pointers.remove(pointer);
    _pointerOrigins.remove(pointer);
    _scrollZonePointers.remove(pointer);
    _cancelLongPressTimer();

    if (hadSinglePointer) {
      if (_longPressRecognized && !_selecting) {
        // 长按识别后未拖动：松手提交右键。
        _longPressRecognized = false;
        onRightTap();
      } else if (_selecting) {
        // 拖选结束：先发出残留位移，再通知上层抬左键 + 自动右键。
        _selecting = false;
        _longPressRecognized = false;
        flushNow();
        onSelectionDragEnd();
      } else if (_tapEligible) {
        // 快速点击（含滚动区）：左键。
        onTap();
      }
    }

    _tapEligible = false;
    _resetGestureBaseline();
    _scheduleFlush();
    if (_pointers.isEmpty) {
      _longPressRecognized = false;
      _selecting = false;
    }
  }

  void pointerCancel(int pointer) {
    final isSingle = _pointers.length == 1 && _pointers.containsKey(pointer);
    final wasSelecting = _selecting;
    _pointers.remove(pointer);
    _pointerOrigins.remove(pointer);
    _scrollZonePointers.remove(pointer);
    _tapEligible = false;
    _cancelLongPressTimer();
    if (isSingle && wasSelecting) {
      // 拖选中被系统取消：仅抬左键。
      _selecting = false;
      _longPressRecognized = false;
      onSelectionDragCancel();
    } else {
      _longPressRecognized = false;
      _selecting = false;
    }
    _resetGestureBaseline();
    _scheduleFlush();
    if (_pointers.isEmpty) {
      _longPressRecognized = false;
      _selecting = false;
    }
  }

  void flushNow() {
    _flushTimer?.cancel();
    _flushTimer = null;
    _flush();
  }

  Duration _elapsedNow() => _injectedElapsed?.call() ?? _stopwatch.elapsed;

  void _startLongPressTimer() {
    _cancelLongPressTimer();
    _longPressTimer = Timer(longPressDelay, () {
      _longPressTimer = null;
      _longPressRecognized = true;
      _tapEligible = false;
      onLongPressRecognized();
    });
  }

  void _cancelLongPressTimer() {
    _longPressTimer?.cancel();
    _longPressTimer = null;
  }

  void _scheduleFlush() {
    if (_pendingX == 0 && _pendingY == 0 && _pendingScroll == 0) return;
    final elapsed = _elapsedNow() - _lastFlushAt;
    if (elapsed >= _minimumFlushInterval) {
      _flush();
      return;
    }
    _flushTimer ??= Timer(_minimumFlushInterval - elapsed, () {
      _flushTimer = null;
      _flush();
    });
  }

  void _flush() {
    _lastFlushAt = _elapsedNow();
    final dx = _takeMouseDelta(true);
    final dy = _takeMouseDelta(false);
    if (dx != 0 || dy != 0) onMove(dx, dy);

    final scroll = _pendingScroll.clamp(-40, 40).truncate();
    _pendingScroll -= scroll;
    if (scroll != 0) onScroll(scroll);

    if (_hasSendableOutput) {
      _scheduleFlush();
    }
  }

  bool get _hasSendableOutput =>
      _pendingX.abs() >= 1 || _pendingY.abs() >= 1 || _pendingScroll.abs() >= 1;

  int _takeMouseDelta(bool horizontal) {
    final pending = horizontal ? _pendingX : _pendingY;
    final delta = pending.clamp(-maxMouseDelta, maxMouseDelta).truncate();
    if (horizontal) {
      _pendingX -= delta;
    } else {
      _pendingY -= delta;
    }
    return delta;
  }

  void _resetGestureBaseline() {
    _lastCentroid = _pointers.length >= 2 ? _centroid() : null;
  }

  Offset _centroid() {
    var x = 0.0;
    var y = 0.0;
    for (final position in _pointers.values) {
      x += position.dx;
      y += position.dy;
    }
    return Offset(x / _pointers.length, y / _pointers.length);
  }

  void dispose() {
    _longPressTimer?.cancel();
    _flushTimer?.cancel();
  }
}
