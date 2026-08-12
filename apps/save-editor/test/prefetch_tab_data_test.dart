import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';

/// Records every core request and answers the shell commands the editor needs
/// to reach a loaded save. Everything else returns a benign empty payload, so a
/// prefetch step that fails is indistinguishable from one that succeeds — which
/// is exactly what the "prefetch never surfaces an error" tests need.
class _RecordingCore implements GoresaveCoreService {
  final requests = <({String command, Map<String, Object?> payload})>[];

  /// Completes for each in-flight command, so a test can hold the core mid-step
  /// and observe what the prefetch does while a request is outstanding.
  final Duration delay;

  /// One command held far longer than the rest, so a test can arrange for a
  /// warm-up step to still be in flight when something else finishes.
  final String? slowCommand;
  static const _slowDelay = Duration(milliseconds: 120);

  _RecordingCore({this.delay = Duration.zero, this.slowCommand});

  List<String> get commands => [for (final r in requests) r.command];

  int commandCount(String command) =>
      requests.where((request) => request.command == command).length;

  Map<String, Object?>? payloadFor(String command) => requests
      .where((request) => request.command == command)
      .map((request) => request.payload)
      .firstOrNull;

  @override
  String get description => 'prefetch-recording-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    requests.add((command: command, payload: Map<String, Object?>.from(payload)));
    final wait = command == slowCommand ? _slowDelay : delay;
    if (wait > Duration.zero) await Future<void>.delayed(wait);
    switch (command) {
      case 'scan_save_dir':
        return {
          'ok': true,
          'data': {
            'saveRoot': r'C:\tmp\saves',
            'saves': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav',
                'slot': 'G1R-001',
                'format': 'GSAV',
                'fileSize': 1,
                'sha1': 'abc',
                'status': 'ok',
              },
            ],
            'profiles': <Object?>[],
          },
        };
      case 'inspect_save':
        return {
          'ok': true,
          'data': {
            'format': 'GSAV',
            'path': payload['path'],
            'slot': 'G1R-001',
            'size': 1,
            'sha1': 'abc',
            'private': {
              'status': 'decoded',
              'preview': false,
              'decompressedSize': 9,
              'typedParse': {'status': 'ok', 'propertyCount': 1, 'maxDepth': 1},
              'player': {'playerName': 'Hero', 'attributes': <Object?>[]},
            },
          },
        };
      case 'list_backups':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'backups': <Object?>[],
            'companionBackups': <Object?>[],
          },
        };
      case 'private.characters.list':
        return {
          'ok': true,
          'data': {
            'characters': [
              {'uniqueName': 'Hero', 'globalId': 'hero-global-id'},
            ],
            'total': 1,
          },
        };
      default:
        return {'ok': true, 'data': <String, Object?>{}};
    }
  }
}

/// Wait until no warm-up is left running. A run that was cut short re-arms
/// itself, which replaces `prefetchInFlight`, so awaiting it once is not enough.
Future<void> _settledPrefetch(EditorNotifier notifier) async {
  for (var i = 0; i < 20; i++) {
    final inFlight = notifier.prefetchInFlight;
    await inFlight;
    await pumpEventQueue();
    if (identical(notifier.prefetchInFlight, inFlight)) return;
  }
  fail('the warm-up never settled');
}

Future<EditorNotifier> _loadedEditor(_RecordingCore core) async {
  final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
  await pumpEventQueue();
  await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
  await pumpEventQueue();
  core.requests.clear();
  return notifier;
}

void main() {
  test('prefetch warms every tab the loaded save can show', () async {
    final core = _RecordingCore();
    final notifier = await _loadedEditor(core);

    notifier.prefetchTabData();
    await notifier.prefetchInFlight;

    // One entry per panel that loads from the core on first paint.
    expect(
      core.commands.toSet(),
      containsAll(<String>[
        'search_typed_properties', // Overview clock + hero attributes + browser
        'private.characters.list', // Characters master list
        'private.skills.list',
        'private.npc.list',
        'query_progression', // quests, glossary, tutorials, story, knowledge
        'private.factions.list',
      ]),
    );
    // Every progression section a panel opens on.
    final sections = [
      for (final request in core.requests)
        if (request.command == 'query_progression') request.payload['section'],
    ];
    expect(
      sections.toSet(),
      containsAll(<String>[
        'knowledge',
        'events',
        'quests',
        'glossary',
        'tutorials',
        'story',
      ]),
    );
  });

  test('prefetch asks for the page sizes the panels ask for', () async {
    final core = _RecordingCore();
    final notifier = await _loadedEditor(core);

    notifier.prefetchTabData();
    await notifier.prefetchInFlight;

    Map<String, Object?> progression(String section) => core.requests
        .firstWhere(
          (request) =>
              request.command == 'query_progression' &&
              request.payload['section'] == section,
        )
        .payload;

    // The core caches one response per exact request, so a prefetch at the
    // wrong page size warms an answer no panel ever asks for.
    expect(progression('knowledge')['limit'], EditorPageSize.detail);
    expect(progression('events')['limit'], EditorPageSize.detail);
    expect(progression('quests')['limit'], EditorPageSize.fullList);
    expect(progression('story')['limit'], EditorPageSize.fullList);
    expect(progression('story')['includeUnset'], isTrue);

    // The property browser's opening request: first page, node model, private
    // source, no facet filters.
    final browse = core.requests.firstWhere(
      (request) =>
          request.command == 'search_typed_properties' &&
          request.payload['includeNodes'] == true,
    );
    expect(browse.payload['query'], '');
    expect(browse.payload['offset'], 0);
    expect(browse.payload['limit'], EditorPageSize.detail);
    expect(browse.payload['source'], 'private');
    expect(browse.payload.containsKey('kind'), isFalse);
    expect(browse.payload.containsKey('type'), isFalse);
    expect(browse.payload.containsKey('editable'), isFalse);
  });

  test('opening a save warms its tabs, driven only by state changes', () async {
    // The page does not call this at a chosen moment — it calls it on every
    // state change. The FIRST change it sees is the inspection landing, which
    // happens while `_inspect` is still fetching the backup list, so the editor
    // is still loading and no warm-up can run yet. That moment must not consume
    // the one trigger this inspection gets, or the call that arrives once
    // loading ends finds the inspection already claimed, skips it, and no tab is
    // ever warmed.
    final core = _RecordingCore();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await pumpEventQueue();
    core.requests.clear();

    // Exactly what the editor page subscribes with.
    final removeListener = notifier.addListener(
      (_) => notifier.prefetchTabData(),
      fireImmediately: false,
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await pumpEventQueue();
    await notifier.prefetchInFlight;
    removeListener();

    expect(
      core.commands,
      contains('private.characters.list'),
      reason: 'opening the save warmed nothing',
    );
    expect(core.commands, contains('private.skills.list'));
    expect(core.commands, contains('query_progression'));
  });

  test('prefetch warms the core without filling the NPC memo', () async {
    // `loadAllNpcActors` memoizes its roster for the lifetime of one inspection.
    // The warm-up must not be what fills that memo: it runs seconds before the
    // user opens an NPC panel, and a save replaced in between (the game, a cloud
    // sync) would leave the panel showing a roster fetched from bytes that are
    // no longer on disk. Warm the core instead, and let the panel's own call
    // populate the memo from the file as of that moment.
    final core = _RecordingCore();
    final notifier = await _loadedEditor(core);

    notifier.prefetchTabData();
    await notifier.prefetchInFlight;

    final warmed = core.commandCount('private.npc.list');
    expect(warmed, greaterThan(0), reason: 'the NPC roster was not warmed');

    // The panel's own call still goes to the core — proof the memo was empty —
    // and is answered from the warm cache.
    await notifier.loadAllNpcActors();
    expect(
      core.commandCount('private.npc.list'),
      greaterThan(warmed),
      reason: 'the warm-up pre-filled the NPC memo',
    );

    // And it is a real memo from then on: a second call adds no request.
    final afterPanel = core.commandCount('private.npc.list');
    await notifier.loadAllNpcActors();
    expect(core.commandCount('private.npc.list'), afterPanel);
  });

  test('prefetch runs once per inspection', () async {
    final core = _RecordingCore();
    final notifier = await _loadedEditor(core);

    notifier.prefetchTabData();
    await notifier.prefetchInFlight;
    final first = core.commands.length;
    expect(first, greaterThan(0));

    // The editor page calls this on every rebuild.
    notifier.prefetchTabData();
    notifier.prefetchTabData();
    await notifier.prefetchInFlight;

    expect(core.commands.length, first, reason: 'prefetch repeated itself');
  });

  test('an interrupted warm-up restarts itself', () async {
    // Something else taking the editor mid-warm-up — a backup rename bumps the
    // load sequence, a codec check raises the loading flag — makes the
    // remaining steps skip. The warm-up has to come back on its own: a step
    // still in flight holds the run open past the moment that operation clears
    // the loading flag, so the state change that would have restarted it
    // bounces off the one-run-at-a-time guard and never comes again.
    // The first warm-up step outlives the interruption, which is what puts the
    // state change that would restart the warm-up before the run has ended.
    final core = _RecordingCore(slowCommand: 'search_typed_properties');
    final notifier = await _loadedEditor(core);

    // The editor page's own wiring, so the restart cannot be attributed to a
    // trigger this test made by hand.
    final removeListener = notifier.addListener(
      (_) => notifier.prefetchTabData(),
      fireImmediately: false,
    );
    notifier.prefetchTabData();
    // Interrupt it: refreshBackups bumps the load sequence without producing a
    // new inspection, so the remaining steps skip. It finishes — and clears the
    // loading flag — while the first warm-up step is still outstanding.
    await notifier.refreshBackups();
    await _settledPrefetch(notifier);
    removeListener();

    // No further trigger of any kind — the warm-up must have re-armed itself.
    expect(
      core.commands,
      contains('private.characters.list'),
      reason: 'the interrupted warm-up was never resumed',
    );
    expect(core.commands, contains('query_progression'));

    // And once it does complete, it is retired: another trigger adds nothing.
    final settled = core.commands.length;
    notifier.prefetchTabData();
    await _settledPrefetch(notifier);
    expect(core.commands.length, settled);
  });

  test('a fresh inspection prefetches again', () async {
    final core = _RecordingCore();
    final notifier = await _loadedEditor(core);

    notifier.prefetchTabData();
    await notifier.prefetchInFlight;
    final first = core.commands.length;

    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await pumpEventQueue();
    notifier.prefetchTabData();
    await notifier.prefetchInFlight;

    expect(core.commands.length, greaterThan(first));
  });

  test('prefetch never turns on the loading overlay', () async {
    final core = _RecordingCore(delay: const Duration(milliseconds: 1));
    final notifier = await _loadedEditor(core);

    var sawLoading = false;
    final removeListener = notifier.addListener((state) {
      if (state.isLoading) sawLoading = true;
    }, fireImmediately: false);

    notifier.prefetchTabData();
    await notifier.prefetchInFlight;
    removeListener();

    expect(sawLoading, isFalse);
    expect(notifier.state.error, isNull);
  });

  test('a newer load stops the prefetch instead of racing it', () async {
    final core = _RecordingCore(delay: const Duration(milliseconds: 5));
    final notifier = await _loadedEditor(core);

    notifier.prefetchTabData();
    // Supersede immediately: a second inspection means the panels will be
    // rebuilt against a different inspection anyway, and every further prefetch
    // request would only make the user's own load wait behind it.
    final reinspect = notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await notifier.prefetchInFlight;
    final duringPrefetch = core.commands
        .where((command) => command != 'inspect_save' && command != 'list_backups')
        .length;
    await reinspect;

    expect(
      duringPrefetch,
      lessThan(6),
      reason: 'prefetch kept queueing work behind a newer load',
    );
  });
}
