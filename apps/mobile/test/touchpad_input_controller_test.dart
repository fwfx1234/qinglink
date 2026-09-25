import 'package:flutter_test/flutter_test.dart';
import 'package:qinglink_mobile/src/services/touchpad_input_controller.dart';

TouchpadInputController _controller({
  void Function(int dx, int dy)? onMove,
  void Function(int delta)? onScroll,
  void Function()? onTap,
  void Function()? onRightTap,
  void Function()? onLongPressRecognized,
  void Function()? onSelectionDragStart,
  void Function()? onSelectionDragEnd,
  void Function()? onSelectionDragCancel,
  Duration Function()? elapsed,
  Duration longPressDelay = TouchpadInputController.longPressDuration,
}) {
  return TouchpadInputController(
    onMove: onMove ?? (_, _) {},
    onScroll: onScroll ?? (_) {},
    onTap: onTap ?? () {},
    onRightTap: onRightTap ?? () {},
    onLongPressRecognized: onLongPressRecognized ?? () {},
    onSelectionDragStart: onSelectionDragStart ?? () {},
    onSelectionDragEnd: onSelectionDragEnd ?? () {},
    onSelectionDragCancel: onSelectionDragCancel ?? () {},
    elapsed: elapsed,
    longPressDelay: longPressDelay,
  );
}

void main() {
  test('uses the default 4x sensitivity and clamps configured values', () {
    final controller = _controller();
    addTearDown(controller.dispose);

    expect(controller.sensitivity, 4);
    controller.sensitivity = 0;
    expect(controller.sensitivity, 1);
    controller.sensitivity = 10;
    expect(controller.sensitivity, 8);
  });

  test('scales one-finger movement and preserves overflow past 500', () {
    var elapsed = Duration.zero;
    final moves = <(int, int)>[];
    final controller = _controller(
      onMove: (dx, dy) => moves.add((dx, dy)),
      elapsed: () => elapsed,
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    elapsed += const Duration(milliseconds: 17);
    controller.pointerMove(1, const Offset(200, 0));
    controller.flushNow();

    expect(moves, [(500, 0), (300, 0)]);
  });

  test(
    'does not emit cursor movement while switching to or from two fingers',
    () {
      var elapsed = Duration.zero;
      final moves = <(int, int)>[];
      final controller = _controller(
        onMove: (dx, dy) => moves.add((dx, dy)),
        elapsed: () => elapsed,
      );
      addTearDown(controller.dispose);

      controller.pointerDown(1, Offset.zero);
      elapsed += const Duration(milliseconds: 17);
      controller.pointerMove(1, const Offset(10, 0));
      controller.pointerDown(2, const Offset(10, 0));
      controller.pointerMove(1, const Offset(15, 0));
      controller.pointerUp(2);
      elapsed += const Duration(milliseconds: 17);
      controller.pointerMove(1, const Offset(20, 0));

      expect(moves, [(40, 0), (20, 0)]);
    },
  );

  test('turns two-finger vertical movement into wheel steps', () {
    var elapsed = Duration.zero;
    final scrolls = <int>[];
    final controller = _controller(
      onScroll: scrolls.add,
      elapsed: () => elapsed,
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    controller.pointerDown(2, Offset.zero);
    elapsed += const Duration(milliseconds: 17);
    controller.pointerMove(2, const Offset(0, -48));

    expect(scrolls, [1]);
  });

  test('tapping without dragging triggers one left click', () {
    var taps = 0;
    final controller = _controller(onTap: () => taps++);
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    controller.pointerUp(1);

    expect(taps, 1);
  });

  test('two quick taps produce two left clicks (PC double click)', () {
    var taps = 0;
    var recognizes = 0;
    var dragStarts = 0;
    final controller = _controller(
      onTap: () => taps++,
      onLongPressRecognized: () => recognizes++,
      onSelectionDragStart: () => dragStarts++,
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    controller.pointerUp(1);
    controller.pointerDown(1, Offset.zero);
    controller.pointerUp(1);

    expect(taps, 2);
    expect(recognizes, 0);
    expect(dragStarts, 0);
  });

  test(
    'long-press recognition fires once and release without drag sends right click',
    () async {
      var recognizes = 0;
      var rightTaps = 0;
      var taps = 0;
      final controller = _controller(
        onLongPressRecognized: () => recognizes++,
        onRightTap: () => rightTaps++,
        onTap: () => taps++,
        longPressDelay: const Duration(milliseconds: 20),
      );
      addTearDown(controller.dispose);

      controller.pointerDown(1, Offset.zero);
      await Future<void>.delayed(const Duration(milliseconds: 60));
      expect(recognizes, 1);
      expect(rightTaps, 0);

      controller.pointerUp(1);
      expect(rightTaps, 1);
      expect(taps, 0);
      expect(recognizes, 1);
    },
  );

  test('long-press is cancelled by movement beyond slop', () async {
    var taps = 0;
    var rightTaps = 0;
    final moves = <(int, int)>[];
    final controller = _controller(
      onMove: (dx, dy) => moves.add((dx, dy)),
      onTap: () => taps++,
      onRightTap: () => rightTaps++,
      elapsed: () => Duration.zero,
      longPressDelay: const Duration(milliseconds: 20),
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    controller.pointerMove(1, const Offset(20, 0));
    controller.flushNow();
    expect(moves, [(80, 0)]);
    expect(rightTaps, 0);

    await Future<void>.delayed(const Duration(milliseconds: 60));
    expect(rightTaps, 0);

    controller.pointerUp(1);
    expect(taps, 0);
  });

  test(
    'slow drag accumulates movement and never triggers right click',
    () async {
      var rightTaps = 0;
      final moves = <(int, int)>[];
      final controller = _controller(
        onMove: (dx, dy) => moves.add((dx, dy)),
        onRightTap: () => rightTaps++,
        elapsed: () => Duration.zero,
        longPressDelay: const Duration(milliseconds: 20),
      );
      addTearDown(controller.dispose);

      controller.pointerDown(1, Offset.zero);
      controller.pointerMove(1, const Offset(3, 0));
      controller.pointerMove(1, const Offset(6, 0));
      controller.pointerMove(1, const Offset(9, 0));
      controller.flushNow();
      expect(rightTaps, 0);

      await Future<void>.delayed(const Duration(milliseconds: 60));
      expect(rightTaps, 0);

      controller.pointerUp(1);
      expect(rightTaps, 0);
    },
  );

  test(
    'slight drift within slop still allows long-press right click',
    () async {
      var recognizes = 0;
      var rightTaps = 0;
      final controller = _controller(
        onLongPressRecognized: () => recognizes++,
        onRightTap: () => rightTaps++,
        longPressDelay: const Duration(milliseconds: 20),
      );
      addTearDown(controller.dispose);

      controller.pointerDown(1, Offset.zero);
      controller.pointerMove(1, const Offset(3, 0));
      await Future<void>.delayed(const Duration(milliseconds: 60));

      expect(recognizes, 1);
      expect(rightTaps, 0);

      controller.pointerUp(1);
      expect(rightTaps, 1);
    },
  );

  test('early release still triggers a left click', () async {
    var taps = 0;
    var rightTaps = 0;
    final controller = _controller(
      onTap: () => taps++,
      onRightTap: () => rightTaps++,
      longPressDelay: const Duration(milliseconds: 200),
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    await Future<void>.delayed(const Duration(milliseconds: 40));
    controller.pointerUp(1);

    expect(taps, 1);
    expect(rightTaps, 0);
  });

  test('a second finger cancels the pending long-press', () async {
    var recognizes = 0;
    var rightTaps = 0;
    final controller = _controller(
      onLongPressRecognized: () => recognizes++,
      onRightTap: () => rightTaps++,
      longPressDelay: const Duration(milliseconds: 20),
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    controller.pointerDown(2, const Offset(10, 0));
    await Future<void>.delayed(const Duration(milliseconds: 60));

    expect(recognizes, 0);
    expect(rightTaps, 0);

    controller.pointerUp(1);
    controller.pointerUp(2);
  });

  test(
    'long-press then drag starts selection, moves and ends with drag end',
    () async {
      var recognizes = 0;
      var dragStarts = 0;
      var dragEnds = 0;
      var rightTaps = 0;
      final moves = <(int, int)>[];
      final controller = _controller(
        onMove: (dx, dy) => moves.add((dx, dy)),
        onLongPressRecognized: () => recognizes++,
        onSelectionDragStart: () => dragStarts++,
        onSelectionDragEnd: () => dragEnds++,
        onRightTap: () => rightTaps++,
        longPressDelay: const Duration(milliseconds: 20),
      );
      addTearDown(controller.dispose);

      controller.pointerDown(1, Offset.zero);
      await Future<void>.delayed(const Duration(milliseconds: 60));
      expect(recognizes, 1);
      expect(dragStarts, 0);

      controller.pointerMove(1, const Offset(10, 0));
      controller.flushNow();
      expect(dragStarts, 1);
      expect(moves, [(40, 0)]);

      controller.pointerMove(1, const Offset(20, 0));
      controller.flushNow();
      expect(moves, [(40, 0), (40, 0)]);

      controller.pointerUp(1);
      expect(dragEnds, 1);
      expect(rightTaps, 0);
      expect(moves, [(40, 0), (40, 0)]);
    },
  );

  test('long-press then drag then second finger cancels selection', () async {
    var dragStarts = 0;
    var dragCancels = 0;
    var dragEnds = 0;
    var rightTaps = 0;
    final controller = _controller(
      onLongPressRecognized: () {},
      onSelectionDragStart: () => dragStarts++,
      onSelectionDragCancel: () => dragCancels++,
      onSelectionDragEnd: () => dragEnds++,
      onRightTap: () => rightTaps++,
      longPressDelay: const Duration(milliseconds: 20),
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    await Future<void>.delayed(const Duration(milliseconds: 60));
    controller.pointerMove(1, const Offset(12, 0));
    expect(dragStarts, 1);

    controller.pointerDown(2, const Offset(5, 5));
    expect(dragCancels, 1);

    controller.pointerUp(1);
    controller.pointerUp(2);
    expect(dragEnds, 0);
    expect(rightTaps, 0);
  });

  test('pointer cancel during pending long-press sends no click', () async {
    var recognizes = 0;
    var rightTaps = 0;
    var dragCancels = 0;
    final controller = _controller(
      onLongPressRecognized: () => recognizes++,
      onRightTap: () => rightTaps++,
      onSelectionDragCancel: () => dragCancels++,
      longPressDelay: const Duration(milliseconds: 20),
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    await Future<void>.delayed(const Duration(milliseconds: 60));
    expect(recognizes, 1);

    controller.pointerCancel(1);
    expect(rightTaps, 0);
    expect(dragCancels, 0);
  });

  test('pointer cancel during selection drag sends drag cancel', () async {
    var dragStarts = 0;
    var dragCancels = 0;
    var dragEnds = 0;
    final controller = _controller(
      onLongPressRecognized: () {},
      onSelectionDragStart: () => dragStarts++,
      onSelectionDragCancel: () => dragCancels++,
      onSelectionDragEnd: () => dragEnds++,
      longPressDelay: const Duration(milliseconds: 20),
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero);
    await Future<void>.delayed(const Duration(milliseconds: 60));
    controller.pointerMove(1, const Offset(12, 0));
    expect(dragStarts, 1);

    controller.pointerCancel(1);
    expect(dragCancels, 1);
    expect(dragEnds, 0);
  });

  test(
    'single-finger drag in the scroll zone scrolls without moving cursor',
    () {
      var elapsed = Duration.zero;
      final scrolls = <int>[];
      final moves = <(int, int)>[];
      final controller = _controller(
        onMove: (dx, dy) => moves.add((dx, dy)),
        onScroll: scrolls.add,
        elapsed: () => elapsed,
      );
      addTearDown(controller.dispose);

      controller.pointerDown(1, Offset.zero, inScrollZone: true);
      elapsed += const Duration(milliseconds: 17);
      controller.pointerMove(1, const Offset(0, -24));
      controller.flushNow();

      expect(scrolls, [1]);
      expect(moves, isEmpty);
    },
  );

  test('horizontal drag inside the scroll zone is ignored', () {
    var elapsed = Duration.zero;
    final scrolls = <int>[];
    final moves = <(int, int)>[];
    final controller = _controller(
      onMove: (dx, dy) => moves.add((dx, dy)),
      onScroll: scrolls.add,
      elapsed: () => elapsed,
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero, inScrollZone: true);
    elapsed += const Duration(milliseconds: 17);
    controller.pointerMove(1, const Offset(50, 0));
    controller.flushNow();

    expect(scrolls, isEmpty);
    expect(moves, isEmpty);
  });

  test('single-finger drag outside the scroll zone moves the cursor', () {
    var elapsed = Duration.zero;
    final scrolls = <int>[];
    final moves = <(int, int)>[];
    final controller = _controller(
      onMove: (dx, dy) => moves.add((dx, dy)),
      onScroll: scrolls.add,
      elapsed: () => elapsed,
    );
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero, inScrollZone: false);
    elapsed += const Duration(milliseconds: 17);
    controller.pointerMove(1, const Offset(10, 0));
    controller.flushNow();

    expect(moves, [(40, 0)]);
    expect(scrolls, isEmpty);
  });

  test('tapping inside the scroll zone still triggers a left click', () {
    var taps = 0;
    final controller = _controller(onTap: () => taps++);
    addTearDown(controller.dispose);

    controller.pointerDown(1, Offset.zero, inScrollZone: true);
    controller.pointerUp(1);

    expect(taps, 1);
  });

  test(
    'scroll zone hold never recognizes long press and release still left clicks',
    () async {
      var recognizes = 0;
      var rightTaps = 0;
      var taps = 0;
      final controller = _controller(
        onLongPressRecognized: () => recognizes++,
        onRightTap: () => rightTaps++,
        onTap: () => taps++,
        longPressDelay: const Duration(milliseconds: 20),
      );
      addTearDown(controller.dispose);

      controller.pointerDown(1, Offset.zero, inScrollZone: true);
      await Future<void>.delayed(const Duration(milliseconds: 60));

      expect(recognizes, 0);
      expect(rightTaps, 0);

      controller.pointerUp(1);
      expect(taps, 1);
      expect(rightTaps, 0);
    },
  );
}
