import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/desktop_updater.dart';

/// Records what a check did and lets a test park any step of it.
class _Recorder {
  _Recorder({this.latest = '9.9.9', this.current = '0.1.0'});

  String? latest;
  String current;

  /// Parks the feed read until completed, so a check can be held mid-flight.
  Completer<void>? parkFetch;

  /// Parks the update prompt, standing in for a dialog nobody answered.
  Completer<bool>? parkPrompt;

  bool downloadAnswer = false;
  bool openSucceeds = true;
  Object? fetchThrows;

  int fetches = 0;
  int prompts = 0;
  int opens = 0;
  final List<PortableUpdateReport> reports = [];
  final List<String> reportUrls = [];

  /// Highest number of steps observed running at once. Anything above 1 means
  /// two checks overlapped.
  int concurrent = 0;
  int _inFlight = 0;

  void _enter() {
    _inFlight++;
    if (_inFlight > concurrent) concurrent = _inFlight;
  }

  void _leave() => _inFlight--;

  PortableUpdateHooks get hooks => PortableUpdateHooks(
    fetchLatestVersion: () async {
      _enter();
      fetches++;
      try {
        // Always suspend at least once. A step that returns without yielding
        // can never be observed overlapping another, which would make the
        // serialization assertions below pass no matter what.
        await Future<void>.delayed(Duration.zero);
        if (parkFetch != null) await parkFetch!.future;
        if (fetchThrows != null) throw fetchThrows!;
        return latest;
      } finally {
        _leave();
      }
    },
    currentVersion: () async => current,
    report: (report, url) async {
      reports.add(report);
      reportUrls.add(url);
    },
    askDownload: (latest, current) async {
      _enter();
      prompts++;
      try {
        await Future<void>.delayed(Duration.zero);
        if (parkPrompt != null) return await parkPrompt!.future;
        return downloadAnswer;
      } finally {
        _leave();
      }
    },
    openReleasePage: (latest) async {
      opens++;
      return openSucceeds;
    },
  );
}

PortableUpdateChecker _checker(_Recorder rec, {Duration? interval}) =>
    PortableUpdateChecker(
      rec.hooks,
      interval: interval ?? const Duration(milliseconds: 20),
    );

void main() {
  group('isNewerVersion', () {
    test('compares components numerically and pads missing ones', () {
      expect(isNewerVersion('0.2.0', '0.1.0'), isTrue);
      expect(isNewerVersion('0.1.0', '0.2.0'), isFalse);
      expect(isNewerVersion('0.1.0', '0.1.0'), isFalse);
      expect(isNewerVersion('0.4.1', '0.4'), isTrue);
      expect(isNewerVersion('0.4', '0.4.1'), isFalse);
      // A malformed component counts as 0 rather than throwing.
      expect(isNewerVersion('1.x.0', '0.9.9'), isTrue);
    });
  });

  test('a background tick yields to a running check', () async {
    final rec = _Recorder()..parkFetch = Completer<void>();
    final checker = _checker(rec);

    final first = checker.run(background: true);
    await pumpEventQueue();
    expect(checker.isChecking, isTrue);

    // The hourly tick landing on a busy checker must do nothing at all.
    await checker.run(background: true);
    expect(rec.fetches, 1);

    rec.parkFetch!.complete();
    await first;
    expect(rec.concurrent, 1);
  });

  test(
    'a manual check waits for a running one instead of being dropped',
    () async {
      final park = Completer<void>();
      final rec = _Recorder()..parkFetch = park;
      final checker = _checker(rec);

      final background = checker.run(background: true);
      await pumpEventQueue();

      final manual = checker.run(background: false);
      await pumpEventQueue();
      // Still queued behind the background check, not silently discarded.
      expect(rec.fetches, 1);

      rec.parkFetch = null;
      park.complete();
      await Future.wait([background, manual]);

      expect(rec.fetches, 2, reason: 'the manual check still ran');
      expect(rec.concurrent, 1, reason: 'never at the same time');
    },
  );

  test('two queued manual checks do not overlap', () async {
    final park = Completer<void>();
    final rec = _Recorder()..parkFetch = park;
    final checker = _checker(rec);

    final running = checker.run(background: true);
    await pumpEventQueue();

    // Both clicks land while the first check holds the slot.
    final a = checker.run(background: false);
    final b = checker.run(background: false);
    await pumpEventQueue();

    rec.parkFetch = null;
    park.complete();
    await Future.wait([running, a, b]);

    expect(rec.fetches, 3);
    expect(rec.concurrent, 1, reason: 'queued clicks must stay serialized');
  });

  test('an unanswered prompt does not let the next tick in', () async {
    final rec = _Recorder()..parkPrompt = Completer<bool>();
    final checker = _checker(rec);

    // Manual, so the prompt is reached without arming the poll — the point
    // here is the open prompt holding the slot, not who opened it.
    final first = checker.run(background: false);
    await pumpEventQueue();
    expect(rec.prompts, 1);

    // An hour later, with the prompt still open.
    await checker.run(background: true);
    expect(rec.prompts, 1, reason: 'no second modal on top of the first');

    rec.parkPrompt!.complete(false);
    await first;
    expect(rec.concurrent, 1);
  });

  test('a failing check releases the slot', () async {
    final rec = _Recorder()..fetchThrows = StateError('feed exploded');
    final checker = _checker(rec);

    // Must not throw out of run(): callers fire it unawaited.
    await checker.run(background: false);
    expect(checker.isChecking, isFalse);

    // And the next check still works.
    rec.fetchThrows = null;
    await checker.run(background: false);
    expect(rec.fetches, 2);
  });

  test('only a manual check reports an uneventful outcome', () async {
    final rec = _Recorder(latest: '0.1.0', current: '0.1.0');
    final checker = _checker(rec);

    await checker.run(background: true);
    expect(rec.reports, isEmpty, reason: 'a silent tick stays silent');

    await checker.run(background: false);
    expect(rec.reports, [PortableUpdateReport.upToDate]);

    rec.latest = null;
    await checker.run(background: false);
    expect(rec.reports.last, PortableUpdateReport.checkFailed);
  });

  test('a cancelled poll cannot still raise a prompt', () async {
    final park = Completer<void>();
    final rec = _Recorder()..parkFetch = park;
    final checker = _checker(rec, interval: const Duration(hours: 1));

    // startPolling awaits its own first check, which is parked on the feed —
    // so hold its future rather than awaiting it here.
    final polling = checker.startPolling();
    await pumpEventQueue();
    checker.stopPolling();
    park.complete();
    await polling;

    expect(rec.prompts, 0, reason: 'auto-check was switched off mid-flight');
  });

  test('a failed download page is reported with its address', () async {
    final rec = _Recorder()
      ..downloadAnswer = true
      ..openSucceeds = false;
    final checker = _checker(rec);

    await checker.run(background: false);

    expect(rec.opens, 1);
    expect(rec.reports, [PortableUpdateReport.downloadPageFailed]);
    expect(rec.reportUrls.single, contains('gore-mod-manager-v9.9.9'));
  });

  test('a successful download page says nothing further', () async {
    final rec = _Recorder()
      ..downloadAnswer = true
      ..openSucceeds = true;
    final checker = _checker(rec);

    await checker.run(background: false);

    expect(rec.opens, 1);
    expect(rec.reports, isEmpty);
  });

  test('declining the prompt never opens anything', () async {
    final rec = _Recorder()..downloadAnswer = false;
    final checker = _checker(rec);

    await checker.run(background: false);

    expect(rec.prompts, 1);
    expect(rec.opens, 0);
    expect(rec.reports, isEmpty);
  });
}
