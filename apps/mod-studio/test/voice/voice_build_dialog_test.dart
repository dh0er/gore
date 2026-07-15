import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/export/ui/build_deploy_dialog.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';
import 'package:path/path.dart' as p;

const _replaceEdit = VoiceArchiveEdit(
  locId: 'INFO_ASGHAN_HELLO',
  locale: 'de',
  archive: 'german_new.zip',
  operation: VoicePatchOperation.replace,
  archivePath: 'NPC/Asghan/info_asghan_hello.ogg',
  oggPath: 'asghan.ogg',
  observation: VoiceArchiveObservation(
    archiveSize: 2000,
    archiveSha256:
        'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
    memberProof: VoiceMemberProof.present(uncompressedSize: 200, crc32: 9),
  ),
);

const _addEdit = VoiceArchiveEdit(
  locId: 'INFO_ASGHAN_NEW',
  locale: 'de',
  archive: 'german_new.zip',
  operation: VoicePatchOperation.add,
  archivePath: 'NPC/Asghan/INFO_ASGHAN_NEW.ogg',
  oggPath: 'asghan-new.ogg',
  observation: VoiceArchiveObservation(
    archiveSize: 2000,
    archiveSha256:
        'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
    memberProof: VoiceMemberProof.absent(),
  ),
);

void main() {
  late Directory gameRoot;

  setUp(() {
    gameRoot = Directory.systemTemp.createTempSync('gore_voice_build_test_');
    Directory(p.join(gameRoot.path, 'G1R')).createSync();
  });

  tearDown(() {
    gameRoot.deleteSync(recursive: true);
  });

  testWidgets('replace-only voice project is visible and buildable', (
    tester,
  ) async {
    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(_safeCore())],
    );
    addTearDown(container.dispose);
    container.read(gameExePathProvider.notifier).set(gameRoot.path);
    container.read(voiceEditsProvider.notifier).setEdit(_replaceEdit);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('1 dialog voice edit'), findsOneWidget);
    final build = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, 'Build to folder…'),
    );
    expect(build.onPressed, isNotNull);
    final deploy = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Deploy to game'),
    );
    expect(deploy.onPressed, isNotNull);
    expect(find.textContaining('Draft-only'), findsNothing);
  });

  testWidgets('Draft voice add blocks Build and Deploy in a short dialog', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(700, 400));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(_safeCore())],
    );
    addTearDown(container.dispose);
    container.read(gameExePathProvider.notifier).set(gameRoot.path);
    container.read(voiceEditsProvider.notifier).setEdit(_addEdit);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('1 dialog voice edit'), findsOneWidget);
    expect(
      find.textContaining(
        'New voice archive members are Draft-only and not runtime-qualified yet.',
      ),
      findsOneWidget,
    );
    final build = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, 'Build to folder…'),
    );
    expect(build.onPressed, isNull);
    final deploy = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Deploy to game'),
    );
    expect(deploy.onPressed, isNull);
    expect(tester.takeException(), isNull);
  });

  testWidgets('unsafe install blocks Deploy but keeps Build and Undeploy', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = FakeGoreCoreFfiService(
      responses: {'script_compile_install_state_v1': _recoveryProbe()},
    );
    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    container.read(gameExePathProvider.notifier).set(gameRoot.path);
    container.read(voiceEditsProvider.notifier).setEdit(_replaceEdit);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('script-compile-install-state-banner')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<OutlinedButton>(
            find.widgetWithText(OutlinedButton, 'Build to folder…'),
          )
          .onPressed,
      isNotNull,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, 'Deploy to game'),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, 'Undeploy'))
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('failed deploy rechecks and immediately blocks recovery state', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FailingDeployCore();
    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    container.read(gameExePathProvider.notifier).set(gameRoot.path);
    container.read(voiceEditsProvider.notifier).setEdit(_replaceEdit);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
      ),
    );
    await tester.pumpAndSettle();
    final deployFinder = find.widgetWithText(FilledButton, 'Deploy to game');
    expect(tester.widget<FilledButton>(deployFinder).onPressed, isNotNull);

    await tester.tap(deployFinder);
    await tester.pump();
    for (
      var attempt = 0;
      attempt < 100 && (core.deployCalls == 0 || core.probeCalls < 3);
      attempt++
    ) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 10)),
      );
      await tester.pump(const Duration(milliseconds: 10));
    }
    await tester.pump(const Duration(milliseconds: 100));

    expect(core.deployCalls, 1);
    expect(core.probeCalls, greaterThanOrEqualTo(3));
    expect(
      find.byKey(const Key('script-compile-install-state-banner')),
      findsOneWidget,
    );
    expect(find.text('Compiler recovery files are present'), findsOneWidget);
    expect(tester.widget<FilledButton>(deployFinder).onPressed, isNull);
  });
}

FakeGoreCoreFfiService _safeCore() => FakeGoreCoreFfiService(
  responses: {'script_compile_install_state_v1': _safeProbe()},
);

Map<String, Object?> _safeProbe() => <String, Object?>{
  'ok': true,
  'disposition': 'safe_to_compile',
  'safe_to_compile': true,
  'game_process': 'not_running',
  'artifacts': <Object?>[],
  'issues': <Object?>[],
};

Map<String, Object?> _recoveryProbe() => <String, Object?>{
  'ok': true,
  'disposition': 'recovery_artifacts_present',
  'safe_to_compile': false,
  'game_process': 'not_running',
  'artifacts': <Object?>[
    <String, Object?>{
      'kind': 'deploy_recovery_record',
      'display_path': r'C:\Game\.gore-deploy-recovery.json',
      'path_truncated': false,
    },
  ],
  'issues': <Object?>[],
};

final class _FailingDeployCore implements GoreCoreFfiService {
  int probeCalls = 0;
  int deployCalls = 0;

  @override
  String get description => 'failing deploy fixture';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    switch (command) {
      case 'script_compile_install_state_v1':
        probeCalls++;
        return probeCalls <= 2 ? _safeProbe() : _recoveryProbe();
      case 'mod_build':
        return <String, Object?>{
          'ok': true,
          'bundle_dir': payload['out_dir'] as String,
        };
      case 'mod_deploy':
        deployCalls++;
        return <String, Object?>{
          'ok': false,
          'error': <String, Object?>{
            'code': 'DEPLOY_FAILED',
            'message': 'deploy failed after recovery record creation',
          },
        };
      default:
        return <String, Object?>{
          'ok': false,
          'error': <String, Object?>{
            'code': 'UNKNOWN_COMMAND',
            'message': 'unexpected command $command',
          },
        };
    }
  }
}
