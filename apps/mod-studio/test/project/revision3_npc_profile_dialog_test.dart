import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_managed_compiler_check_panel.dart';
import 'package:gore_mod/project/revision3_npc_profile_dialog.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state_provider.dart';

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

  testWidgets(
    'cannot close during compiler check and records exact acceptance',
    (tester) async {
      final projectJson = revision3NpcInspectionProjectJson();
      final inspection = revision3NpcInspectionResult(
        head: revision3NpcFixtureHead(projectJson),
        projectJson: projectJson,
      );
      final safety = ScriptCompileInstallSafetyController(
        (_) async => _safeInstall(),
        gameRoot: r'C:\Game',
        autoRefresh: false,
      );
      await safety.refresh();
      final pending = Completer<ManagedRevision3CompilerCheckReceipt>();
      var checkStarted = false;

      await _openNpcProfile(
        tester,
        inspect: ({required npcId}) async => inspection,
        gameRoot: r'C:\Game',
        safety: safety,
        checkCompiler: () {
          checkStarted = true;
          return pending.future;
        },
      );
      final result = find.byKey(const Key('revision3-npc-profile-result'));
      final scrollable = find
          .descendant(of: result, matching: find.byType(Scrollable))
          .first;
      final run = find.byKey(const Key('revision3-managed-compiler-check-run'));
      await tester.scrollUntilVisible(run, 180, scrollable: scrollable);
      await Scrollable.ensureVisible(tester.element(run), alignment: 0.5);
      await tester.pumpAndSettle();
      await tester.tap(run.hitTestable());
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-managed-compiler-confirm')),
      );
      for (var attempt = 0; attempt < 10 && !checkStarted; attempt++) {
        await tester.pump();
      }

      expect(checkStarted, isTrue);
      await tester.pump();
      final close = find.byKey(const Key('revision3-npc-profile-dialog-close'));
      expect(tester.widget<TextButton>(close).onPressed, isNull);
      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-npc-profile-dialog')),
        findsOneWidget,
      );

      final receipt = _acceptedNpcCompilerReceipt(projectJson);
      expect(receipt.acceptedAtExactCurrent, isTrue);
      pending.complete(receipt);
      await tester.pumpAndSettle();
      await tester.pump();

      expect(tester.widget<TextButton>(close).onPressed, isNotNull);
      expect(
        find.byKey(const Key('revision3-managed-compiler-check-error')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-managed-compiler-check-result')),
        findsOneWidget,
      );
      await tester.scrollUntilVisible(
        find.text('Build readiness — 3 blockers'),
        180,
        scrollable: scrollable,
      );
      expect(find.text('Build readiness — 3 blockers'), findsOneWidget);
      expect(find.text('Compiler not run'), findsNothing);
      expect(
        find.text('Exact source accepted by the compiler'),
        findsOneWidget,
      );
    },
  );

  testWidgets('stale compiler evidence grants no NPC readiness authority', (
    tester,
  ) async {
    final projectJson = revision3NpcInspectionProjectJson();
    final inspection = revision3NpcInspectionResult(
      head: revision3NpcFixtureHead(projectJson),
      projectJson: projectJson,
    );
    final safety = ScriptCompileInstallSafetyController(
      (_) async => _safeInstall(),
      gameRoot: r'C:\Game',
      autoRefresh: false,
    );
    await safety.refresh();

    await _openNpcProfile(
      tester,
      inspect: ({required npcId}) async => inspection,
      gameRoot: r'C:\Game',
      safety: safety,
      checkCompiler: () async =>
          _acceptedNpcCompilerReceipt(projectJson, exactCurrent: false),
    );
    final result = find.byKey(const Key('revision3-npc-profile-result'));
    final scrollable = find
        .descendant(of: result, matching: find.byType(Scrollable))
        .first;
    final run = find.byKey(const Key('revision3-managed-compiler-check-run'));
    await tester.scrollUntilVisible(run, 180, scrollable: scrollable);
    await Scrollable.ensureVisible(tester.element(run), alignment: 0.5);
    await tester.pumpAndSettle();
    await tester.tap(run.hitTestable());
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-managed-compiler-confirm')),
    );
    await tester.pumpAndSettle();

    expect(find.text('Compiler result is no longer current'), findsOneWidget);
    await tester.scrollUntilVisible(
      find.textContaining('4 blockers'),
      180,
      scrollable: scrollable,
    );
    expect(find.textContaining('4 blockers'), findsOneWidget);
    expect(find.text('Compiler not run'), findsOneWidget);
    expect(find.text('Exact source accepted by the compiler'), findsNothing);
  });
}

Future<void> _openNpcProfile(
  WidgetTester tester, {
  required Revision3NpcSourceInspectionLoader inspect,
  bool settle = true,
  String? gameRoot,
  Revision3ManagedCompilerChecker? checkCompiler,
  ScriptCompileInstallSafetyController? safety,
}) async {
  Widget app = MaterialApp(
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
              gameRoot: gameRoot,
              checkCompiler: checkCompiler,
            ),
          ),
          child: const Text('Open NPC profile'),
        ),
      ),
    ),
  );
  if (safety != null) {
    app = ProviderScope(
      overrides: [
        scriptCompileInstallSafetyProvider.overrideWith((ref) => safety),
      ],
      child: app,
    );
  }
  await tester.pumpWidget(app);
  await tester.tap(find.byKey(const Key('open-revision3-npc-profile')));
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

ScriptCompileInstallState _safeInstall() =>
    ScriptCompileInstallState.fromJson(<String, Object?>{
      'ok': true,
      'disposition': 'safe_to_compile',
      'safe_to_compile': true,
      'game_process': 'not_running',
      'artifacts': <Object?>[],
      'issues': <Object?>[],
    });

ManagedRevision3CompilerCheckReceipt _acceptedNpcCompilerReceipt(
  String projectJson, {
  bool exactCurrent = true,
}) {
  final head = revision3NpcFixtureHead(projectJson);
  final result = AuthoringRevision3ManagedCompilerCheckResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'compiler_check_only',
      'exact_current': exactCurrent,
      'head_json': head.canonicalJson,
      'project': <String, Object?>{
        'id': revision3NpcInspectionProjectId,
        'revision': 7,
        'seal': <String, Object?>{
          'byte_len': head.snapshotByteLength,
          'sha256': head.snapshotSha256,
        },
      },
      'entity': <String, Object?>{
        'kind': 'npc_draft',
        'id': revision3NpcInspectionNpcId,
        'revision': 2,
      },
      'module': <String, Object?>{
        'id': revision3NpcInspectionModuleId,
        'revision': 3,
        'namespace': revision3NpcInspectionModuleNamespace,
        'relative_path':
            '${revision3NpcInspectionModuleNamespace.replaceAll('.', '/')}.as',
        'source_sha256':
            'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      },
      'compiler': <String, Object?>{
        'outcome': 'compiled_evidence_only',
        'compile_error': null,
        'compiler_diagnostics': <String, Object?>{
          'capture': 'captured',
          'messages': <Object?>[],
          'omitted': 0,
        },
        'install_restore': 'restored_exact',
        'recovery_required': false,
        'output_discarded': true,
      },
      'scope': 'compiler_check_only',
      'build_status': 'blocked',
      'deploy_status': 'not_supported',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
    requestedEntityId: revision3NpcInspectionNpcId,
    expectedKind: AuthoringRevision3ManagedCompilerEntityKind.npcDraft,
  );
  return ManagedRevision3CompilerCheckReceipt(
    result: result,
    storeStillExactCurrent: true,
  );
}
