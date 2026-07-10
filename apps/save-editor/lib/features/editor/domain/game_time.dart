/// Model + pure conversions for the world game clock.
///
/// The clock is a single typed `DoubleProperty` at
/// `m_GenericData{GameTime} › CurrentTime › TotalSeconds`, holding the total
/// elapsed in-game seconds. The game counts days from 0 (a fresh game starts on
/// day 0), so a [day] of 15 is shown to the player as "day 15".
library;

const int secondsPerMinute = 60;
const int secondsPerHour = 60 * secondsPerMinute; // 3600
const int secondsPerDay = 24 * secondsPerHour; // 86400

/// The world clock leaf: its total seconds plus the `private.typed.setValue`
/// path that addresses it. Returned by `EditorNotifier.loadGameTime`.
class GameTime {
  const GameTime({required this.totalSeconds, required this.path});

  final double totalSeconds;
  final List<String> path;
}

/// Day / hour / minute / second decomposition of a whole-second game time.
/// Days are 0-based; hours are 0–23 and minute/second 0–59 after a successful
/// [fromTotalSeconds]. The constructor itself does not range-check — the editor
/// validates user input before calling [toTotalSeconds].
class GameTimeParts {
  const GameTimeParts({
    required this.day,
    required this.hour,
    required this.minute,
    required this.second,
  });

  /// Decompose [totalSeconds], truncating any sub-second fraction (game time is
  /// always non-negative, so flooring is a plain truncation toward zero).
  factory GameTimeParts.fromTotalSeconds(double totalSeconds) {
    final whole = totalSeconds.floor();
    return GameTimeParts(
      day: whole ~/ secondsPerDay,
      hour: (whole % secondsPerDay) ~/ secondsPerHour,
      minute: (whole % secondsPerHour) ~/ secondsPerMinute,
      second: whole % secondsPerMinute,
    );
  }

  final int day;
  final int hour;
  final int minute;
  final int second;

  /// Recompose to whole seconds. The caller is responsible for range-checking
  /// the fields (day ≥ 0, hour 0–23, minute/second 0–59) beforehand.
  int toTotalSeconds() =>
      day * secondsPerDay +
      hour * secondsPerHour +
      minute * secondsPerMinute +
      second;
}
