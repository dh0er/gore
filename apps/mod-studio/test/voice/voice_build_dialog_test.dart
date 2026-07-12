import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
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
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(gameExePathProvider.notifier).set(gameRoot.path);
    container.read(voiceEditsProvider.notifier).setEdit(_replaceEdit);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
      ),
    );

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

    final container = ProviderContainer();
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
}
