import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/game_time.dart';

void main() {
  group('GameTimeParts.fromTotalSeconds', () {
    test('decomposes a real cumulative value (day 0-based)', () {
      // Save 021: 1374712.47 s -> day 15, 21:51:52 (game counts days from 0).
      final parts = GameTimeParts.fromTotalSeconds(1374712.4669570476);
      expect(parts.day, 15);
      expect(parts.hour, 21);
      expect(parts.minute, 51);
      expect(parts.second, 52);
    });

    test('zero is day 0, 00:00:00', () {
      final parts = GameTimeParts.fromTotalSeconds(0);
      expect(parts.day, 0);
      expect(parts.hour, 0);
      expect(parts.minute, 0);
      expect(parts.second, 0);
    });

    test('truncates the sub-second fraction (does not round up)', () {
      final parts = GameTimeParts.fromTotalSeconds(59.99);
      expect(parts.day, 0);
      expect(parts.hour, 0);
      expect(parts.minute, 0);
      expect(parts.second, 59);
    });

    test('exact day boundary rolls the clock to 00:00:00', () {
      final parts = GameTimeParts.fromTotalSeconds(86400);
      expect(parts.day, 1);
      expect(parts.hour, 0);
      expect(parts.minute, 0);
      expect(parts.second, 0);
    });
  });

  group('GameTimeParts.toTotalSeconds', () {
    test('recomposes to whole seconds', () {
      const parts = GameTimeParts(day: 15, hour: 21, minute: 51, second: 52);
      expect(parts.toTotalSeconds(), 1374712);
    });

    test('round-trips a truncated value', () {
      const original = 1374712.4669570476;
      final back =
          GameTimeParts.fromTotalSeconds(original).toTotalSeconds();
      expect(back, original.floor());
    });
  });
}
