import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';
import 'package:gore_mod/voice/ui/voice_line_editor.dart';
import 'package:path/path.dart' as p;

const _locId = 'info_viper_gore_01';
const _digest =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('unique installed match stages a sealed replacement', (
    tester,
  ) async {
    final fixture = _VoiceFixture.create();
    addTearDown(fixture.dispose);
    final ogg = fixture.ogg('replacement.ogg');
    var pickerCalls = 0;
    final container = await _pumpEditor(
      tester,
      gameRoot: fixture.root.path,
      matcher: ({required archive, required locId}) async => _match(
        archive: archive,
        locId: locId,
        matches: [_entry(7, 'dialog/$locId.ogg', size: 4321, crc32: 1234)],
      ),
      picker: () async {
        pickerCalls++;
        return ogg;
      },
    );

    await tester.tap(find.byKey(const ValueKey('voice-choose-ogg')));
    await tester.pumpAndSettle();

    expect(pickerCalls, 1);
    final edit = container.read(voiceEditsProvider).entries.single;
    expect(edit.locId, _locId);
    expect(edit.locale, 'de');
    expect(edit.archive, 'german_new.zip');
    expect(edit.operation, VoicePatchOperation.replace);
    expect(edit.archivePath, 'dialog/$_locId.ogg');
    expect(edit.oggPath, ogg);
    expect(edit.observation.archiveSize, 9999);
    expect(edit.observation.archiveSha256, _digest);
    expect(edit.observation.memberProof.state, VoiceMemberProofState.present);
    expect(edit.observation.memberProof.uncompressedSize, 4321);
    expect(edit.observation.memberProof.crc32, 1234);
    expect(find.byKey(const ValueKey('voice-staged')), findsOneWidget);
  });

  testWidgets('zero matches explains the limitation and never opens picker', (
    tester,
  ) async {
    final fixture = _VoiceFixture.create();
    addTearDown(fixture.dispose);
    var pickerCalls = 0;
    final container = await _pumpEditor(
      tester,
      gameRoot: fixture.root.path,
      matcher: ({required archive, required locId}) async =>
          _match(archive: archive, locId: locId, matches: const []),
      picker: () async {
        pickerCalls++;
        return fixture.ogg('must-not-be-picked.ogg');
      },
    );

    await tester.tap(find.byKey(const ValueKey('voice-choose-ogg')));
    await tester.pumpAndSettle();

    expect(pickerCalls, 0);
    expect(container.read(voiceEditsProvider).entries, isEmpty);
    expect(
      find.textContaining('Creating new voiced lines is not qualified yet'),
      findsOneWidget,
    );
  });

  testWidgets('mismatched inspection identity is rejected without staging', (
    tester,
  ) async {
    final fixture = _VoiceFixture.create();
    addTearDown(fixture.dispose);
    var pickerCalls = 0;
    final container = await _pumpEditor(
      tester,
      gameRoot: fixture.root.path,
      matcher: ({required archive, required locId}) async => _match(
        archive: p.join(p.dirname(archive), 'different.zip'),
        locId: 'info_different_line',
        matches: [_entry(1, 'speech/info_different_line.ogg')],
      ),
      picker: () async {
        pickerCalls++;
        return fixture.ogg('must-not-be-picked.ogg');
      },
    );

    await tester.tap(find.byKey(const ValueKey('voice-choose-ogg')));
    await tester.pumpAndSettle();

    expect(pickerCalls, 0);
    expect(container.read(voiceEditsProvider).entries, isEmpty);
    expect(
      find.textContaining('returned a different line or archive'),
      findsOneWidget,
    );
  });

  testWidgets('ambiguous match requires an explicit candidate selection', (
    tester,
  ) async {
    final fixture = _VoiceFixture.create();
    addTearDown(fixture.dispose);
    final ogg = fixture.ogg('chosen.ogg');
    var pickerCalls = 0;
    final container = await _pumpEditor(
      tester,
      gameRoot: fixture.root.path,
      matcher: ({required archive, required locId}) async => _match(
        archive: archive,
        locId: locId,
        matches: [
          _entry(1, 'first/$locId.ogg'),
          _entry(2, 'second/$locId.ogg'),
        ],
      ),
      picker: () async {
        pickerCalls++;
        return ogg;
      },
    );

    await tester.tap(find.byKey(const ValueKey('voice-choose-ogg')));
    await tester.pumpAndSettle();
    expect(pickerCalls, 0);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('voice-use-selected')),
          )
          .onPressed,
      isNull,
    );

    await tester.tap(find.byKey(const ValueKey('voice-candidate-2')));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('voice-use-selected')));
    await tester.pumpAndSettle();

    expect(pickerCalls, 1);
    expect(
      container.read(voiceEditsProvider).entries.single.archivePath,
      'second/$_locId.ogg',
    );
  });

  testWidgets('voice edits stay isolated by canonical locale', (tester) async {
    final fixture = _VoiceFixture.create(withEnglish: true);
    addTearDown(fixture.dispose);
    final german = fixture.ogg('german-take.ogg');
    final english = fixture.ogg('english-take.ogg');
    final picks = <String>[german, english];
    final container = await _pumpEditor(
      tester,
      gameRoot: fixture.root.path,
      matcher: ({required archive, required locId}) async => _match(
        archive: archive,
        locId: locId,
        matches: [_entry(1, 'speech/$locId.ogg')],
      ),
      picker: () async => picks.removeAt(0),
    );

    await tester.tap(find.byKey(const ValueKey('voice-choose-ogg')));
    await tester.pumpAndSettle();
    await _chooseLocale(tester, 'English');
    expect(find.text('german-take.ogg'), findsNothing);
    await tester.tap(find.byKey(const ValueKey('voice-choose-ogg')));
    await tester.pumpAndSettle();

    final state = container.read(voiceEditsProvider);
    expect(state.count, 2);
    expect(state.items[(_locId, 'de')]?.oggPath, german);
    expect(state.items[(_locId, 'en')]?.oggPath, english);
    expect(find.text('english-take.ogg'), findsOneWidget);

    await _chooseLocale(tester, 'Deutsch');
    expect(find.text('german-take.ogg'), findsOneWidget);
    expect(find.text('english-take.ogg'), findsNothing);
  });

  testWidgets('a locale change invalidates an in-flight archive request', (
    tester,
  ) async {
    final fixture = _VoiceFixture.create(withEnglish: true);
    addTearDown(fixture.dispose);
    final result = Completer<VoiceArchiveMatchLineResult>();
    var pickerCalls = 0;
    final container = await _pumpEditor(
      tester,
      gameRoot: fixture.root.path,
      matcher: ({required archive, required locId}) => result.future,
      picker: () async {
        pickerCalls++;
        return fixture.ogg('stale.ogg');
      },
    );

    await tester.tap(find.byKey(const ValueKey('voice-choose-ogg')));
    await tester.pump();
    await _chooseLocale(tester, 'English');
    result.complete(
      _match(
        archive: fixture.germanArchive,
        locId: _locId,
        matches: [_entry(1, 'speech/$_locId.ogg')],
      ),
    );
    await tester.pumpAndSettle();

    expect(pickerCalls, 0);
    expect(container.read(voiceEditsProvider).entries, isEmpty);
    expect(find.textContaining('Replacement staged'), findsNothing);
  });

  testWidgets('an existing staged take can be removed', (tester) async {
    final fixture = _VoiceFixture.create();
    addTearDown(fixture.dispose);
    final ogg = fixture.ogg('existing.ogg');
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(localeProvider.notifier).setLocale('de');
    container.read(gameExePathProvider.notifier).set(fixture.root.path);
    container
        .read(voiceEditsProvider.notifier)
        .setEdit(_edit(oggPath: ogg, archivePath: 'speech/$_locId.ogg'));
    await _pumpWithContainer(
      tester,
      container,
      matcher: ({required archive, required locId}) async =>
          _match(archive: archive, locId: locId, matches: const []),
      picker: () async => null,
    );

    expect(find.text('existing.ogg'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('voice-remove')));
    await tester.pumpAndSettle();

    expect(container.read(voiceEditsProvider).entries, isEmpty);
    expect(find.byKey(const ValueKey('voice-staged')), findsNothing);
  });

  testWidgets('missing game root and missing locale archive are actionable', (
    tester,
  ) async {
    final fixture = _VoiceFixture.create(createGermanArchive: false);
    addTearDown(fixture.dispose);
    final container = await _pumpEditor(
      tester,
      matcher: ({required archive, required locId}) =>
          throw StateError('must not inspect'),
      picker: () => throw StateError('must not pick'),
    );

    expect(
      find.textContaining('Choose your Gothic installation in Settings'),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('voice-choose-ogg')), findsNothing);

    container.read(gameExePathProvider.notifier).set(fixture.root.path);
    await tester.pumpAndSettle();
    expect(
      find.textContaining('No voice archive was found for Deutsch'),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('voice-choose-ogg')), findsNothing);
  });
}

Future<ProviderContainer> _pumpEditor(
  WidgetTester tester, {
  String? gameRoot,
  required VoiceArchiveMatcher matcher,
  required VoiceOggPicker picker,
}) async {
  final container = ProviderContainer();
  addTearDown(container.dispose);
  container.read(localeProvider.notifier).setLocale('de');
  if (gameRoot != null) {
    container.read(gameExePathProvider.notifier).set(gameRoot);
  }
  await _pumpWithContainer(tester, container, matcher: matcher, picker: picker);
  return container;
}

Future<void> _pumpWithContainer(
  WidgetTester tester,
  ProviderContainer container, {
  required VoiceArchiveMatcher matcher,
  required VoiceOggPicker picker,
}) async {
  tester.view.physicalSize = const Size(1200, 1000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: VoiceLineEditor(
              locId: _locId,
              matcher: matcher,
              oggPicker: picker,
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _chooseLocale(WidgetTester tester, String endonym) async {
  await tester.tap(find.byKey(const ValueKey('voice-locale')));
  await tester.pumpAndSettle();
  await tester.tap(find.text(endonym).last);
  await tester.pumpAndSettle();
}

VoiceArchiveMatchLineResult _match({
  required String archive,
  required String locId,
  required List<VoiceArchiveEntryInfo> matches,
}) => VoiceArchiveMatchLineResult(
  archive: archive,
  archiveSize: 9999,
  archiveSha256: _digest,
  locId: locId,
  expectedBasename: '$locId.ogg',
  resolution: switch (matches.length) {
    0 => VoiceArchiveLineResolution.unresolved,
    1 => VoiceArchiveLineResolution.unique,
    _ => VoiceArchiveLineResolution.ambiguous,
  },
  matches: matches,
);

VoiceArchiveEntryInfo _entry(
  int index,
  String path, {
  int size = 100,
  int crc32 = 42,
}) => VoiceArchiveEntryInfo(
  index: index,
  path: path,
  basename: path.split('/').last,
  compressedSize: size,
  uncompressedSize: size,
  crc32: crc32,
  compression: 'stored',
  compressionCode: 0,
  lastModified: null,
  unixMode: null,
  isDirectory: false,
  isSymlink: false,
  encrypted: false,
);

VoiceArchiveEdit _edit({
  required String oggPath,
  required String archivePath,
}) => VoiceArchiveEdit(
  locId: _locId,
  locale: 'de',
  archive: 'german_new.zip',
  operation: VoicePatchOperation.replace,
  archivePath: archivePath,
  oggPath: oggPath,
  observation: const VoiceArchiveObservation(
    archiveSize: 9999,
    archiveSha256: _digest,
    memberProof: VoiceMemberProof.present(uncompressedSize: 100, crc32: 42),
  ),
);

class _VoiceFixture {
  _VoiceFixture._(this.root, this.germanArchive);

  factory _VoiceFixture.create({
    bool createGermanArchive = true,
    bool withEnglish = false,
  }) {
    final root = Directory.systemTemp.createTempSync('gore_voice_line_ui_');
    final voiceRoot = Directory(p.join(root.path, 'G1R', 'Story', 'VoiceOver'))
      ..createSync(recursive: true);
    final germanArchive = p.join(voiceRoot.path, 'german_new.zip');
    if (createGermanArchive) File(germanArchive).writeAsBytesSync([1]);
    if (withEnglish) {
      File(p.join(voiceRoot.path, 'english_newer.zip')).writeAsBytesSync([1]);
    }
    return _VoiceFixture._(root, germanArchive);
  }

  final Directory root;
  final String germanArchive;

  String ogg(String name) {
    final file = File(p.join(root.path, name))
      ..writeAsBytesSync([79, 103, 103]);
    return file.path;
  }

  void dispose() {
    if (root.existsSync()) root.deleteSync(recursive: true);
  }
}
