import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/audio_replacements_notifier.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/project_controller.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:path/path.dart' as p;

class _ThrowingDialogTopicsNotifier extends DialogTopicsNotifier {
  @override
  void loadAll(List<DialogTopicDefinition> topics) {
    throw StateError('injected apply failure');
  }
}

void main() {
  test('metadata-only edits are dirty before the first baseline', () {
    final container = ProviderContainer.test();
    final session = container.read(projectSessionProvider);

    expect(session.hasUnsavedChanges, isFalse);
    container.read(modVersionProvider.notifier).state = '1.0';
    expect(session.hasUnsavedChanges, isTrue);
  });

  test(
    'save records the captured signature, not edits made during I/O',
    () async {
      final started = Completer<void>();
      final finish = Completer<void>();
      ModProject? written;
      final container = ProviderContainer.test(
        overrides: [
          projectSaverProvider.overrideWithValue((project, path) async {
            written = project;
            started.complete();
            await finish.future;
          }),
        ],
      );
      final session = container.read(projectSessionProvider);
      container.read(modNameProvider.notifier).state = 'Captured';

      final saving = session.saveToPath('captured.goremod');
      await started.future;
      container.read(modNameProvider.notifier).state = 'Edited during save';
      finish.complete();
      await saving;

      expect(written!.name, 'Captured');
      expect(session.currentPath, 'captured.goremod');
      expect(
        container.read(savedProjectSignatureProvider),
        jsonEncode(written!.toJson()),
      );
      expect(session.hasUnsavedChanges, isTrue);
    },
  );

  test('save-to-current resolves the path after earlier queued Open', () async {
    final loadStarted = Completer<void>();
    final finishLoad = Completer<void>();
    final writes = <(String, String)>[];
    final container = ProviderContainer.test(
      overrides: [
        projectLoaderProvider.overrideWithValue((path) async {
          loadStarted.complete();
          await finishLoad.future;
          return LoadedProject(
            project: ModProject(name: 'Opened'),
            workspace: null,
          );
        }),
        projectSaverProvider.overrideWithValue((project, path) async {
          writes.add((path, project.name));
        }),
      ],
    );
    final session = container.read(projectSessionProvider);
    container.read(currentProjectPathProvider.notifier).state = 'old.goremod';

    final opening = session.openFromPath('opened.goremod');
    await loadStarted.future;
    final saving = session.saveToCurrentPath();
    finishLoad.complete();
    await opening;
    await saving;

    expect(writes, [('opened.goremod', 'Opened')]);
    expect(session.currentPath, 'opened.goremod');
  });

  test('Save, Open, New, and Save run in invocation order', () async {
    final firstSaveStarted = Completer<void>();
    final finishFirstSave = Completer<void>();
    final events = <String>[];
    final container = ProviderContainer.test(
      overrides: [
        projectSaverProvider.overrideWithValue((project, path) async {
          events.add('save:$path:${project.name}');
          if (path == 'first.goremod') {
            firstSaveStarted.complete();
            await finishFirstSave.future;
          }
        }),
        projectLoaderProvider.overrideWithValue((path) async {
          events.add('open:$path');
          return LoadedProject(
            project: ModProject(name: 'Loaded'),
            workspace: null,
          );
        }),
      ],
    );
    final session = container.read(projectSessionProvider);
    container.read(modNameProvider.notifier).state = 'First';

    final firstSave = session.saveToPath('first.goremod');
    await firstSaveStarted.future;
    final opening = session.openFromPath('opened.goremod');
    final creating = session.newProject();
    final secondSave = session.saveToPath('second.goremod');
    finishFirstSave.complete();

    await firstSave;
    await opening;
    await creating;
    await secondSave;
    expect(events, [
      'save:first.goremod:First',
      'open:opened.goremod',
      'save:second.goremod:MyMod',
    ]);
    expect(container.read(modNameProvider), 'MyMod');
    expect(session.currentPath, 'second.goremod');
    expect(session.hasUnsavedChanges, isFalse);
  });

  test('successful Open and New release the previous workspace', () async {
    final fixture = await Directory.systemTemp.createTemp(
      'goremod_session_swap_',
    );
    addTearDown(() => fixture.delete(recursive: true));
    final withAssets = await _writeProjectWithAudio(fixture, 'assets');
    final withoutAssets = p.join(fixture.path, 'plain.goremod');
    await saveProject(ModProject(name: 'Plain'), withoutAssets);

    final loadedRoots = <String>[];
    final container = ProviderContainer.test(
      overrides: [
        projectLoaderProvider.overrideWithValue((path) async {
          final loaded = await loadProject(path);
          if (loaded.workspace != null) loadedRoots.add(loaded.workspace!.path);
          return loaded;
        }),
      ],
    );
    final session = container.read(projectSessionProvider);

    await session.openFromPath(withAssets);
    final firstRoot = Directory(loadedRoots.single);
    expect(await firstRoot.exists(), isTrue);
    await session.openFromPath(withoutAssets);
    expect(await firstRoot.exists(), isFalse);

    await session.openFromPath(withAssets);
    final secondRoot = Directory(loadedRoots.last);
    expect(await secondRoot.exists(), isTrue);
    await session.newProject();
    expect(await secondRoot.exists(), isFalse);
    expect(session.currentPath, isNull);
  });

  test(
    'old-workspace cleanup failure does not release the adopted candidate',
    () async {
      final fixture = await Directory.systemTemp.createTemp(
        'goremod_session_cleanup_failure_',
      );
      addTearDown(() => fixture.delete(recursive: true));
      final firstArchive = await _writeProjectWithAudio(fixture, 'first');
      final secondArchive = await _writeProjectWithAudio(fixture, 'second');
      final roots = <String, String>{};
      final container = ProviderContainer.test(
        overrides: [
          projectLoaderProvider.overrideWithValue((path) async {
            final loaded = await loadProject(path);
            roots[path] = loaded.workspace!.path;
            return loaded;
          }),
        ],
      );
      final session = container.read(projectSessionProvider);

      await session.openFromPath(firstArchive);
      final firstRoot = Directory(roots[firstArchive]!);
      await firstRoot.delete(recursive: true);
      final tamperedRoot = File(firstRoot.path);
      await tamperedRoot.writeAsString('preserve');

      await expectLater(
        session.openFromPath(secondArchive),
        completion(secondArchive),
      );

      final secondRoot = Directory(roots[secondArchive]!);
      expect(session.currentPath, secondArchive);
      expect(container.read(modNameProvider), 'second');
      expect(await secondRoot.exists(), isTrue);
      expect(session.retainedWorkspacePaths, [firstRoot.path]);
      expect(
        await File(
          container.read(audioReplacementsProvider).entries.single.wavPath,
        ).exists(),
        isTrue,
      );

      await tamperedRoot.delete();
      await session.newProject();
      expect(await secondRoot.exists(), isFalse);
    },
  );

  test('old-workspace cleanup failure does not fail New', () async {
    final fixture = await Directory.systemTemp.createTemp(
      'goremod_session_new_cleanup_failure_',
    );
    addTearDown(() => fixture.delete(recursive: true));
    final archive = await _writeProjectWithAudio(fixture, 'before-new');
    String? rootPath;
    final container = ProviderContainer.test(
      overrides: [
        projectLoaderProvider.overrideWithValue((path) async {
          final loaded = await loadProject(path);
          rootPath = loaded.workspace!.path;
          return loaded;
        }),
      ],
    );
    final session = container.read(projectSessionProvider);

    await session.openFromPath(archive);
    final root = Directory(rootPath!);
    await root.delete(recursive: true);
    final tamperedRoot = File(root.path);
    await tamperedRoot.writeAsString('preserve');

    await expectLater(session.newProject(), completes);

    expect(container.read(modNameProvider), 'MyMod');
    expect(container.read(audioReplacementsProvider).count, 0);
    expect(session.currentPath, isNull);
    expect(session.hasUnsavedChanges, isFalse);
    expect(session.retainedWorkspacePaths, [root.path]);

    await tamperedRoot.delete();
  });

  test('an apply failure releases only the candidate workspace', () async {
    final fixture = await Directory.systemTemp.createTemp(
      'goremod_session_apply_failure_',
    );
    addTearDown(() => fixture.delete(recursive: true));
    final archive = await _writeProjectWithAudio(fixture, 'candidate');
    String? candidateRoot;
    final container = ProviderContainer.test(
      overrides: [
        dialogTopicsProvider.overrideWith(
          (ref) => _ThrowingDialogTopicsNotifier(),
        ),
        projectLoaderProvider.overrideWithValue((path) async {
          final loaded = await loadProject(path);
          candidateRoot = loaded.workspace!.path;
          return loaded;
        }),
      ],
    );
    final session = container.read(projectSessionProvider);
    container.read(modNameProvider.notifier).state = 'Existing';
    container.read(currentProjectPathProvider.notifier).state = 'old.goremod';

    await expectLater(
      session.openFromPath(archive),
      throwsA(isA<StateError>()),
    );

    expect(container.read(modNameProvider), 'Existing');
    expect(session.currentPath, 'old.goremod');
    expect(await Directory(candidateRoot!).exists(), isFalse);
  });
}

Future<String> _writeProjectWithAudio(Directory fixture, String stem) async {
  final wav = File(p.join(fixture.path, '$stem.wav'));
  await wav.writeAsBytes(<int>[
    ...ascii.encode('RIFF'),
    ...List<int>.filled(32, 7),
  ]);
  final archive = p.join(fixture.path, '$stem.goremod');
  await saveProject(
    ModProject(
      name: stem,
      audio: [
        AudioReplacement(
          bank: '$stem.bank',
          sample: '$stem-sample',
          wavPath: wav.path,
        ),
      ],
    ),
    archive,
  );
  return archive;
}
