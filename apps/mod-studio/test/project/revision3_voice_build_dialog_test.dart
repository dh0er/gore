import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_build_dialog.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_voice_fixture.dart';

const _projectId = '11111111111111111111111111111111';
const _bundleSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

final _head = AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': 321,
      'sha256':
          'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
    },
  }),
);

void main() {
  testWidgets(
    'keeps every output control hidden until the exact plan is ready',
    (tester) async {
      final planned = Completer<AuthoringRevision3VoiceBuildPlanResult>();

      await _openDialog(
        tester,
        plan: () => planned.future,
        pickParent: () async => null,
        build: (_) async => throw StateError('build must remain unavailable'),
        settle: false,
      );

      expect(
        find.byKey(const Key('revision3-voice-build-plan-loading')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-build-folder-name')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-voice-build-choose-parent')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-voice-build-submit')),
        findsNothing,
      );

      planned.complete(_readyPlan());
      await tester.pumpAndSettle();

      expect(find.text('Voice is ready'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-build-folder-name')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-build-choose-parent')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-build-submit')),
        findsOneWidget,
      );
    },
  );

  testWidgets('plan failure exposes retry but no output or build authority', (
    tester,
  ) async {
    var calls = 0;
    await _openDialog(
      tester,
      plan: () async {
        calls += 1;
        throw const FormatException('technical plan detail');
      },
      pickParent: () async => null,
      build: (_) async => throw StateError('build must remain unavailable'),
    );

    expect(
      find.byKey(const Key('revision3-voice-build-plan-error')),
      findsOneWidget,
    );
    expect(find.textContaining('exact current project'), findsOneWidget);
    expect(find.textContaining('technical plan detail'), findsNothing);
    expect(
      find.byKey(const Key('revision3-voice-build-folder-name')),
      findsNothing,
    );
    expect(find.byKey(const Key('revision3-voice-build-submit')), findsNothing);

    await tester.tap(find.byKey(const Key('revision3-voice-build-plan-retry')));
    await tester.pumpAndSettle();
    expect(calls, 2);
  });

  testWidgets('builds only a brand-new child and shows its sealed receipt', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_dialog_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    String? requestedOutput;

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: (output) async {
        requestedOutput = output;
        expect(
          FileSystemEntity.typeSync(output, followLinks: false),
          FileSystemEntityType.notFound,
        );
        return _built(output);
      },
    );

    expect(find.textContaining('Offline build only'), findsOneWidget);
    expect(find.textContaining('does not deploy'), findsOneWidget);
    await tester.enterText(
      find.byKey(const Key('revision3-voice-build-folder-name')),
      'asghan-voice-bundle',
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-build-choose-parent')),
    );
    await tester.pumpAndSettle();

    final expectedOutput = p.join(parent.path, 'asghan-voice-bundle');
    expect(find.text(expectedOutput), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
    await tester.pumpAndSettle();

    expect(requestedOutput, expectedOutput);
    expect(
      find.byKey(const Key('revision3-voice-build-built')),
      findsOneWidget,
    );
    expect(find.text('Sealed Voice bundle built'), findsOneWidget);
    expect(
      find.text('Offline receipt only. Deployment was not performed.'),
      findsOneWidget,
    );
    expect(find.text(_bundleSha), findsOneWidget);
    expect(find.text('Basis project revision'), findsOneWidget);
    expect(find.text('2'), findsOneWidget);
    expect(find.text('4'), findsOneWidget);
    expect(find.text('8192'), findsOneWidget);
  });

  testWidgets('shows structured blockers without claiming output authority', (
    tester,
  ) async {
    var pickCalls = 0;
    var buildCalls = 0;

    await _openDialog(
      tester,
      plan: () async => _blockedPlan(),
      pickParent: () async {
        pickCalls += 1;
        return null;
      },
      build: (_) async {
        buildCalls += 1;
        return _blocked();
      },
    );

    expect(
      find.byKey(const Key('revision3-voice-readiness-report')),
      findsOneWidget,
    );
    expect(find.text('Voice needs attention'), findsOneWidget);
    expect(find.textContaining('0 of 2 Voice slots are ready'), findsOneWidget);
    expect(find.text('Resolve this Voice target.'), findsOneWidget);
    expect(find.text('Select an approved Voice take.'), findsOneWidget);
    expect(find.text('Asghan greeting — de'), findsOneWidget);
    expect(find.text('Asghan greeting — de-x1'), findsOneWidget);
    expect(find.textContaining('No bundle was created'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-voice-build-folder-name')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-voice-build-choose-parent')),
      findsNothing,
    );
    expect(find.byKey(const Key('revision3-voice-build-submit')), findsNothing);
    expect(find.text(revision3VoiceFixtureLineId), findsNothing);
    expect(find.text('GRD_263_ASGHAN_OPEN_INFO_06_02'), findsNothing);
    expect(find.textContaining(_bundleSha), findsNothing);
    expect(pickCalls, 0);
    expect(buildCalls, 0);
  });

  testWidgets('shows the payload budget blocker without claiming a bundle', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_budget_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: (_) async => _payloadBudgetBlocked(),
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-build-choose-parent')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
    await tester.pumpAndSettle();

    expect(
      find.text(
        'The selected Voice recordings exceed the safe bundle memory budget.',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('0 of 2 Voice slots are ready'), findsOneWidget);
    expect(find.textContaining('No bundle was created'), findsOneWidget);
    expect(find.byKey(const Key('revision3-voice-build-built')), findsNothing);
  });

  testWidgets('blocker action closes the bound dialog before navigation', (
    tester,
  ) async {
    var planCalls = 0;
    String? openedLine;
    String? openedLocale;
    var dialogWasClosed = false;

    await _openDialog(
      tester,
      plan: () async {
        planCalls += 1;
        return _blockedPlan();
      },
      pickParent: () async => null,
      build: (_) async => throw StateError('build must remain unavailable'),
      onResolveVoiceTarget: ({required initialLineId, required initialLocale}) {
        dialogWasClosed = find
            .byKey(const Key('revision3-voice-build-dialog'))
            .evaluate()
            .isEmpty;
        openedLine = initialLineId;
        openedLocale = initialLocale;
      },
    );

    final action = find.byKey(
      const ValueKey('revision3-voice-readiness-blocker-action-0'),
    );
    await tester.ensureVisible(action);
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-voice-build-dialog')), findsNothing);
    expect(openedLine, revision3VoiceFixtureLineId);
    expect(openedLocale, 'de');
    expect(dialogWasClosed, isTrue);
    expect(planCalls, 1);
  });

  testWidgets(
    'failed blocker deep link reports through the stable scaffold after pop',
    (tester) async {
      const failureMessage =
          'The exact Voice workflow could not be opened from this checkpoint.';

      await _openDialog(
        tester,
        plan: () async => _blockedPlan(),
        pickParent: () async => null,
        build: (_) async => throw StateError('build must remain unavailable'),
        onResolveVoiceTarget:
            ({required initialLineId, required initialLocale}) {
              throw StateError('simulated navigation failure');
            },
        deepLinkFailureMessage: failureMessage,
      );

      final action = find.byKey(
        const ValueKey('revision3-voice-readiness-blocker-action-0'),
      );
      await tester.ensureVisible(action);
      await tester.tap(action);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-voice-build-dialog')),
        findsNothing,
      );
      expect(find.text(failureMessage), findsOneWidget);
      expect(find.byType(SnackBar), findsOneWidget);
      expect(find.textContaining('simulated navigation failure'), findsNothing);
    },
  );

  testWidgets('rejects unsafe names and an existing target', (tester) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_existing_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    Directory(p.join(parent.path, 'already-there')).createSync();
    var buildCalls = 0;

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: (output) async {
        buildCalls += 1;
        return _built(output);
      },
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-build-choose-parent')),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('revision3-voice-build-folder-name')),
      '../escape',
    );
    await tester.pump();
    expect(
      find.textContaining('without separators or reserved characters'),
      findsOneWidget,
    );
    expect(_submitButton(tester).onPressed, isNull);

    await tester.enterText(
      find.byKey(const Key('revision3-voice-build-folder-name')),
      'CON.txt',
    );
    await tester.pump();
    expect(find.textContaining('reserved by Windows'), findsOneWidget);
    expect(_submitButton(tester).onPressed, isNull);

    await tester.enterText(
      find.byKey(const Key('revision3-voice-build-folder-name')),
      'already-there',
    );
    await tester.pump();
    final submit = find.byKey(const Key('revision3-voice-build-submit'));
    await tester.ensureVisible(submit);
    final submitButton = _submitButton(tester);
    expect(submitButton.onPressed, isNotNull);
    submitButton.onPressed!();
    await tester.pumpAndSettle();

    expect(buildCalls, 0);
    expect(
      find.byKey(const Key('revision3-voice-build-error')),
      findsOneWidget,
    );
    expect(find.textContaining('target already exists'), findsOneWidget);
  });

  testWidgets('rejects a symlink child before invoking the build', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_link_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    final linkPath = p.join(parent.path, 'linked-output');
    try {
      Link(linkPath).createSync(p.join(parent.path, 'missing-target'));
    } on FileSystemException {
      // Some Windows hosts do not grant symbolic-link creation to tests.
      return;
    }
    var buildCalls = 0;

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: (output) async {
        buildCalls += 1;
        return _built(output);
      },
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-build-choose-parent')),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-voice-build-folder-name')),
      'linked-output',
    );
    await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
    await tester.pumpAndSettle();

    expect(buildCalls, 0);
    expect(find.textContaining('target path is a symlink'), findsOneWidget);
  });

  testWidgets('cannot dismiss while the picker or exact build is pending', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_pending_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    final picked = Completer<String?>();
    final built = Completer<AuthoringRevision3VoiceBuildResult>();

    await _openDialog(
      tester,
      pickParent: () => picked.future,
      build: (_) => built.future,
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-build-choose-parent')),
    );
    await tester.pump();
    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-voice-build-dialog')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<TextButton>(
            find.byKey(const Key('revision3-voice-build-close')),
          )
          .onPressed,
      isNull,
    );

    picked.complete(parent.path);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
    await tester.pump();
    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-voice-build-dialog')),
      findsOneWidget,
    );

    final output = p.join(parent.path, 'voice-bundle');
    built.complete(_built(output));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-voice-build-built')),
      findsOneWidget,
    );
  });

  testWidgets('stale and reopen failures are terminal for this build window', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_terminal_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    for (final failure in <Exception>[
      const Revision3VoiceBuildStaleCheckpointException(),
      const Revision3VoiceBuildRequiresReopenException(),
    ]) {
      await _openDialog(
        tester,
        pickParent: () async => parent.path,
        build: (_) async => throw failure,
      );
      await tester.tap(
        find.byKey(const Key('revision3-voice-build-choose-parent')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
      await tester.pumpAndSettle();

      expect(
        find.textContaining(
          failure is Revision3VoiceBuildStaleCheckpointException
              ? 'changed while this window was open'
              : 'can no longer be verified as current',
        ),
        findsOneWidget,
      );
      expect(_submitButton(tester).onPressed, isNull);
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-voice-build-choose-parent')),
            )
            .onPressed,
        isNull,
      );
      await tester.tap(find.byKey(const Key('revision3-voice-build-close')));
      await tester.pumpAndSettle();
    }
  });

  testWidgets('maps native build error codes to actionable guidance', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_native_error_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    const cases = <(String, String)>[
      (
        'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH',
        'Re-import or retarget the managed project',
      ),
      (
        'AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS',
        'outside every game installation',
      ),
      (
        'AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED',
        'game installation changed while the bundle was being built',
      ),
      (
        'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED',
        'output parent changed while the bundle was being built',
      ),
      (
        'AUTHORING_REVISION3_VOICE_BUILD_VERIFY_FAILED',
        'Do not use that output',
      ),
      (
        'AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED',
        'A conflicting output was left untouched',
      ),
    ];

    for (final (code, guidance) in cases) {
      await _openDialog(
        tester,
        pickParent: () async => parent.path,
        build: (_) async => throw ModFfiException(
          command: 'authoring_store_build_revision3_voice_v1',
          code: code,
          message: 'technical native detail',
        ),
      );
      await tester.tap(
        find.byKey(const Key('revision3-voice-build-choose-parent')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
      await tester.pumpAndSettle();

      expect(find.textContaining(guidance), findsOneWidget);
      expect(find.textContaining('technical native detail'), findsNothing);
      await tester.tap(find.byKey(const Key('revision3-voice-build-close')));
      await tester.pumpAndSettle();
    }
  });

  testWidgets('shows bounded native cleanup detail only for cleanup failure', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_voice_build_cleanup_error_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    const cleanupDetail =
        r'Temporary staging cleanup failed: C:\Builds\.voice-bundle.gore-staging-1234';

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: (_) async => throw const ModFfiException(
        command: 'authoring_store_build_revision3_voice_v1',
        code: 'AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED',
        message: cleanupDetail,
      ),
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-build-choose-parent')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining('temporary staging folder could not be removed'),
      findsOneWidget,
    );
    expect(find.textContaining(cleanupDetail), findsOneWidget);
  });

  testWidgets(
    'unconfirmed publication preserves detail and disables every retry control',
    (tester) async {
      final parent = Directory.systemTemp.createTempSync(
        'gore_voice_build_publication_unconfirmed_',
      );
      addTearDown(() => parent.deleteSync(recursive: true));
      const detail =
          r'Atomic publication status unknown: C:\Builds\voice-bundle';

      await _openDialog(
        tester,
        pickParent: () async => parent.path,
        build: (_) async => throw const ModFfiException(
          command: 'authoring_store_build_revision3_voice_v1',
          code: 'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED',
          message: detail,
        ),
      );
      await tester.tap(
        find.byKey(const Key('revision3-voice-build-choose-parent')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
      await tester.pumpAndSettle();

      expect(
        find.textContaining('atomic publication may have succeeded'),
        findsOneWidget,
      );
      expect(find.textContaining(detail), findsOneWidget);
      expect(_submitButton(tester).onPressed, isNull);
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-voice-build-choose-parent')),
            )
            .onPressed,
        isNull,
      );
    },
  );
}

Future<void> _openDialog(
  WidgetTester tester, {
  required Revision3VoiceBuildParentDirectoryPicker pickParent,
  required Revision3VoiceExactBuild build,
  Future<AuthoringRevision3VoiceBuildPlanResult> Function()? plan,
  FutureOr<void> Function({
    required String initialLineId,
    required String initialLocale,
  })?
  onResolveVoiceTarget,
  FutureOr<void> Function({
    required String initialLineId,
    required String initialLocale,
  })?
  onManageVoiceTakes,
  String? deepLinkFailureMessage,
  bool settle = true,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) {
            final messenger = ScaffoldMessenger.of(context);
            return FilledButton(
              key: const Key('open-voice-build'),
              onPressed: () => showDialog<AuthoringRevision3VoiceBuildResult>(
                context: context,
                builder: (_) => Revision3VoiceBuildDialog(
                  plan: plan ?? () async => _readyPlan(),
                  build: build,
                  pickExistingParentDirectory: pickParent,
                  onResolveVoiceTarget: onResolveVoiceTarget,
                  onManageVoiceTakes: onManageVoiceTakes,
                  onDeepLinkFailure: () {
                    if (!messenger.mounted) return;
                    messenger.showSnackBar(
                      SnackBar(
                        content: Text(
                          deepLinkFailureMessage ??
                              'The selected Voice workflow could not be opened.',
                        ),
                      ),
                    );
                  },
                ),
              ),
              child: const Text('Open'),
            );
          },
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-voice-build')));
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

FilledButton _submitButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-voice-build-submit')),
);

AuthoringRevision3VoiceBuildResult _built(String output) =>
    AuthoringRevision3VoiceBuildResult.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'built',
        'basis_head_json': _head.canonicalJson,
        'project_id': _projectId,
        'project_revision': 7,
        'output': output,
        'edit_count': 2,
        'file_count': 4,
        'bundle_bytes': 8192,
        'bundle_sha256': _bundleSha,
        'build_authority': 'generation_sealed_existing_member_bundle_v1',
        'deployment_status': 'not_performed',
      },
      expectedHead: _head,
      expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
        slotCount: 2,
        projectId: _projectId,
      ),
      expectedOutput: output,
    );

AuthoringRevision3VoiceBuildPlanResult _readyPlan() =>
    AuthoringRevision3VoiceBuildPlanResult.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'ready',
        'basis_head_json': _head.canonicalJson,
        'project_id': _projectId,
        'project_revision': 7,
        'total_slots': 2,
        'ready_slots': 2,
        'blockers': const <Object?>[],
        'plan_authority': 'read_only_voice_build_plan_v1',
        'build_authority': 'not_granted',
        'deployment_status': 'not_performed',
      },
      expectedHead: _head,
      expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
        slotCount: 2,
        projectId: _projectId,
      ),
    );

AuthoringRevision3VoiceBuildPlanResult _blockedPlan() =>
    AuthoringRevision3VoiceBuildPlanResult.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'blocked',
        'basis_head_json': _head.canonicalJson,
        'project_id': _projectId,
        'project_revision': 7,
        'total_slots': 2,
        'ready_slots': 0,
        'blockers': _mixedBlockersJson(),
        'plan_authority': 'read_only_voice_build_plan_v1',
        'build_authority': 'not_granted',
        'deployment_status': 'not_performed',
      },
      expectedHead: _head,
      expectedProjectJson: _mixedBlockedProjectJson(),
    );

AuthoringRevision3VoiceBuildResult _blocked() =>
    AuthoringRevision3VoiceBuildResult.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'blocked',
        'basis_head_json': _head.canonicalJson,
        'project_id': _projectId,
        'project_revision': 7,
        'report': <String, Object?>{
          'project_id': _projectId,
          'project_revision': 7,
          'total_slots': 2,
          'ready_slots': 0,
          'blockers': _mixedBlockersJson(),
        },
        'build_authority': 'not_granted',
        'deployment_status': 'not_performed',
      },
      expectedHead: _head,
      expectedProjectJson: _mixedBlockedProjectJson(),
      expectedOutput: 'unused-for-blocked-response',
    );

List<Object?> _mixedBlockersJson() => <Object?>[
  <String, Object?>{
    'slot_id': '00000000000000000000000000100000',
    'line_id': revision3VoiceFixtureLineId,
    'line_label': 'Asghan greeting',
    'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
    'locale': 'de',
    'reason': 'unresolved_target',
  },
  <String, Object?>{
    'slot_id': '00000000000000000000000000100001',
    'line_id': revision3VoiceFixtureLineId,
    'line_label': 'Asghan greeting',
    'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
    'locale': 'de-x1',
    'reason': 'missing_selected_take',
  },
];

AuthoringRevision3VoiceBuildResult _payloadBudgetBlocked() =>
    AuthoringRevision3VoiceBuildResult.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'blocked',
        'basis_head_json': _head.canonicalJson,
        'project_id': _projectId,
        'project_revision': 7,
        'report': <String, Object?>{
          'project_id': _projectId,
          'project_revision': 7,
          'total_slots': 2,
          'ready_slots': 0,
          'blockers': <Object?>[
            <String, Object?>{'reason': 'voice_payload_budget_exceeded'},
          ],
        },
        'build_authority': 'not_granted',
        'deployment_status': 'not_performed',
      },
      expectedHead: _head,
      expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
        slotCount: 2,
        assetBytes: 256 * 1024 * 1024 + 1,
        projectId: _projectId,
      ),
      expectedOutput: 'unused-for-blocked-response',
    );

String _mixedBlockedProjectJson() {
  final project =
      (jsonDecode(
                revision3VoiceFixtureBuildReadyProjectJson(
                  slotCount: 2,
                  projectId: _projectId,
                ),
              )
              as Map)
          .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();

  Map<String, Object?> slotData(String id) {
    final entity = (entities[id]! as Map).cast<String, Object?>();
    final payload = (entity['payload']! as Map).cast<String, Object?>();
    return (payload['data']! as Map).cast<String, Object?>();
  }

  slotData('00000000000000000000000000100000')['target_resolution'] =
      <String, Object?>{'state': 'unresolved'};
  slotData('00000000000000000000000000100001').remove('selected');
  return jsonEncode(project);
}
