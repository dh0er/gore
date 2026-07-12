import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_controller.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';

const edit = VoiceArchiveEdit(
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

void main() {
  testWidgets(
    'voice participates in dirty, gather, apply, saved baseline, and new',
    (tester) async {
      late WidgetRef ref;
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Consumer(
              builder: (context, widgetRef, child) {
                ref = widgetRef;
                return Text('dirty=${projectIsDirty(widgetRef)}');
              },
            ),
          ),
        ),
      );

      expect(find.text('dirty=false'), findsOneWidget);
      expect(hasUnsavedChanges(ref), isFalse);

      ref.read(voiceEditsProvider.notifier).setEdit(edit);
      await tester.pump();
      expect(find.text('dirty=true'), findsOneWidget);
      expect(hasUnsavedChanges(ref), isTrue);
      expect(gatherProject(ref).voice.single.locId, edit.locId);

      markProjectSaved(ref);
      expect(hasUnsavedChanges(ref), isFalse);
      ref
          .read(voiceEditsProvider.notifier)
          .setEdit(edit.withOggPath('changed.ogg'));
      expect(hasUnsavedChanges(ref), isTrue);

      applyProject(ref, ModProject(name: 'Loaded', voice: const [edit]));
      expect(ref.read(voiceEditsProvider).entries.single.oggPath, 'asghan.ogg');
      expect(gatherProject(ref).name, 'Loaded');
      markProjectSaved(ref);
      expect(hasUnsavedChanges(ref), isFalse);

      newProject(ref);
      await tester.pump();
      expect(ref.read(voiceEditsProvider).count, 0);
      expect(find.text('dirty=false'), findsOneWidget);
      expect(hasUnsavedChanges(ref), isFalse);
    },
  );

  testWidgets('apply preflights voice identities before mutating providers', (
    tester,
  ) async {
    late WidgetRef ref;
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Consumer(
            builder: (context, widgetRef, child) {
              ref = widgetRef;
              return const SizedBox();
            },
          ),
        ),
      ),
    );
    ref.read(modNameProvider.notifier).state = 'Unchanged';
    ref.read(voiceEditsProvider.notifier).setEdit(edit);

    final duplicate = VoiceArchiveEdit(
      locId: edit.locId.toLowerCase(),
      locale: edit.locale,
      archive: edit.archive,
      operation: edit.operation,
      archivePath: 'Other/${edit.locId.toLowerCase()}.ogg',
      oggPath: 'duplicate.ogg',
      observation: edit.observation,
    );
    expect(
      () => applyProject(
        ref,
        ModProject(name: 'MustNotApply', voice: [edit, duplicate]),
      ),
      throwsFormatException,
    );
    expect(ref.read(modNameProvider), 'Unchanged');
    expect(ref.read(voiceEditsProvider).entries.single.oggPath, 'asghan.ogg');
  });
}
