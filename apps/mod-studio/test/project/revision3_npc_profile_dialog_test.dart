import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/revision3_npc_profile_dialog.dart';

import '../support/revision3_npc_fixture.dart';

void main() {
  testWidgets(
    'loads one exact NPC and shows only saved evidence plus four blockers',
    (tester) async {
      final projectJson = revision3NpcInspectionProjectJson();
      final head = revision3NpcFixtureHead(projectJson);
      final inspection = revision3NpcInspectionResult(
        head: head,
        projectJson: projectJson,
      );
      final pending = Completer<AuthoringRevision3NpcSourceInspectionResult>();
      String? requestedNpc;

      await _openNpcProfile(
        tester,
        settle: false,
        inspect: ({required npcId}) {
          requestedNpc = npcId;
          return pending.future;
        },
      );

      expect(requestedNpc, revision3NpcInspectionNpcId);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(
        find.text('Verifying the saved NPC Draft and generated source…'),
        findsOneWidget,
      );

      pending.complete(inspection);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-npc-profile-result')),
        findsOneWidget,
      );
      expect(find.text('Draft only'), findsOneWidget);
      expect(find.text('Build blocked'), findsOneWidget);
      expect(find.text('Not spawned'), findsOneWidget);
      expect(
        find.text('Based on Asghan (saved parent evidence)'),
        findsOneWidget,
      );
      expect(find.text('Saved source verified'), findsOneWidget);
      expect(find.text('Saved parent evidence verified'), findsOneWidget);
      expect(find.text('Exact project version checked'), findsOneWidget);
      final result = find.byKey(const Key('revision3-npc-profile-result'));
      final scrollable = find
          .descendant(of: result, matching: find.byType(Scrollable))
          .first;
      await tester.scrollUntilVisible(
        find.text('Build readiness — 4 blockers'),
        180,
        scrollable: scrollable,
      );
      expect(find.text('Build readiness — 4 blockers'), findsOneWidget);
      for (final title in <String>[
        'Compiler not run',
        'Production build unavailable',
        'In-game residence unqualified',
        'Spawn mechanism unavailable',
      ]) {
        await tester.scrollUntilVisible(
          find.text(title),
          120,
          scrollable: scrollable,
        );
        expect(find.text(title), findsOneWidget);
      }
      expect(find.text('Build ready'), findsNothing);
      expect(find.text('Runtime qualified'), findsNothing);
      expect(find.text('Spawned'), findsNothing);
      expect(find.text('Published'), findsNothing);
    },
  );

  testWidgets('Advanced exposes exact source and copies only that source', (
    tester,
  ) async {
    final projectJson = revision3NpcInspectionProjectJson();
    final head = revision3NpcFixtureHead(projectJson);
    final inspection = revision3NpcInspectionResult(
      head: head,
      projectJson: projectJson,
    );
    String? clipboardText;
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
      if (call.method == 'Clipboard.setData') {
        clipboardText = (call.arguments as Map)['text'] as String?;
      }
      return null;
    });
    addTearDown(
      () => messenger.setMockMethodCallHandler(SystemChannels.platform, null),
    );

    await _openNpcProfile(
      tester,
      inspect: ({required npcId}) async => inspection,
    );
    final result = find.byKey(const Key('revision3-npc-profile-result'));
    final advanced = find.byKey(const Key('revision3-npc-profile-advanced'));
    await tester.scrollUntilVisible(
      advanced,
      180,
      scrollable: find
          .descendant(of: result, matching: find.byType(Scrollable))
          .first,
    );
    await tester.tap(advanced);
    await tester.pumpAndSettle();

    expect(find.text(revision3NpcInspectionNpcId), findsOneWidget);
    expect(find.text(revision3NpcInspectionModuleId), findsOneWidget);
    expect(find.text(revision3NpcInspectionModuleNamespace), findsOneWidget);
    final copy = find.byKey(const Key('revision3-npc-source-copy'));
    await Scrollable.ensureVisible(tester.element(copy), alignment: 0.5);
    await tester.pumpAndSettle();
    await tester.tap(copy.hitTestable());
    await tester.pump();

    expect(clipboardText, inspection.plan.generatedSource);
    expect(find.text('Generated source copied'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-npc-generated-source')),
      findsOneWidget,
    );
  });

  testWidgets('retries a transient domain failure in the same dialog', (
    tester,
  ) async {
    final projectJson = revision3NpcInspectionProjectJson();
    final inspection = revision3NpcInspectionResult(
      head: revision3NpcFixtureHead(projectJson),
      projectJson: projectJson,
    );
    var calls = 0;

    await _openNpcProfile(
      tester,
      inspect: ({required npcId}) async {
        calls++;
        if (calls == 1) {
          throw const ModFfiException(
            command: 'authoring_store_inspect_revision3_npc_source_v1',
            code: 'AUTHORING_REVISION3_NPC_INSPECTION_NPC_INVALID',
            message: 'the selected NPC cannot be inspected yet',
          );
        }
        return inspection;
      },
    );

    final retry = find.byKey(const Key('revision3-npc-profile-retry'));
    expect(retry, findsOneWidget);
    expect(
      find.text('the selected NPC cannot be inspected yet'),
      findsOneWidget,
    );
    await tester.tap(retry);
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.text('Saved source verified'), findsOneWidget);
  });

  testWidgets('stale and requires-reopen errors close instead of retrying', (
    tester,
  ) async {
    for (final error in <Object>[
      const Revision3NpcSourceInspectionStaleCheckpointException(),
      const Revision3NpcSourceInspectionRequiresReopenException(),
    ]) {
      var calls = 0;
      await _openNpcProfile(
        tester,
        inspect: ({required npcId}) async {
          calls++;
          throw error;
        },
      );

      expect(
        find.byKey(const Key('revision3-npc-profile-retry')),
        findsNothing,
      );
      final close = find.byKey(const Key('revision3-npc-profile-close'));
      expect(close, findsOneWidget);
      expect(find.text('Close and refresh'), findsOneWidget);
      if (error is Revision3NpcSourceInspectionStaleCheckpointException) {
        expect(
          find.textContaining('project changed after this NPC was selected'),
          findsOneWidget,
        );
      } else {
        expect(find.textContaining('must be reopened'), findsOneWidget);
      }

      await tester.tap(close);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-npc-profile-dialog')),
        findsNothing,
      );
      expect(calls, 1);
    }
  });
}

Future<void> _openNpcProfile(
  WidgetTester tester, {
  required Revision3NpcSourceInspectionLoader inspect,
  bool settle = true,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            key: const Key('open-revision3-npc-profile'),
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => Revision3NpcProfileDialog(
                npcTitle: 'Inspection Guard',
                npcId: revision3NpcInspectionNpcId,
                inspect: inspect,
              ),
            ),
            child: const Text('Open NPC profile'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-revision3-npc-profile')));
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}
