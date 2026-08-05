import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_voice_build_readiness_panel.dart';

import '../support/revision3_voice_fixture.dart';

const _projectId = '11111111111111111111111111111111';
const _otherProjectId = '22222222222222222222222222222222';
const _projectRoot = r'C:\mods\voice-readiness';

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

final _otherHead = AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': 322,
      'sha256':
          'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
    },
  }),
);

void main() {
  testWidgets('loads exact readiness and explains unavailable build action', (
    tester,
  ) async {
    final planned = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    var planCalls = 0;
    Future<AuthoringRevision3VoiceBuildPlanResult> plan() {
      planCalls += 1;
      return planCalls == 1 ? planned.future : Future.value(_readyPlan());
    }

    await _pumpPanel(tester, plan: plan);

    expect(
      find.byKey(const Key('revision3-voice-readiness-loading')),
      findsOneWidget,
    );
    expect(
      find.textContaining('existing Voice slots pass this bundle plan'),
      findsNothing,
    );

    planned.complete(_readyPlan());
    await tester.pumpAndSettle();

    expect(find.text('Voice bundle plan checked'), findsOneWidget);
    expect(
      find.text('2 of 2 existing Voice slots pass this bundle plan.'),
      findsOneWidget,
    );
    expect(find.textContaining('configured game installation'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-voice-readiness-build')),
      findsNothing,
    );

    await tester.tap(
      find.byKey(const Key('revision3-voice-readiness-refresh')),
    );
    await tester.pumpAndSettle();
    expect(planCalls, 2);
  });

  testWidgets('keeps Validate blockers collapsed and routes friendly actions', (
    tester,
  ) async {
    var planCalls = 0;
    final opened = <(String, String, String)>[];
    await _pumpPanel(
      tester,
      plan: () async {
        planCalls += 1;
        return _blockedPlan();
      },
      onResolveVoiceTarget: ({required initialLineId, required initialLocale}) {
        opened.add(('resolve', initialLineId, initialLocale));
      },
      onManageVoiceTakes: ({required initialLineId, required initialLocale}) {
        opened.add(('manage', initialLineId, initialLocale));
      },
    );
    await tester.pumpAndSettle();

    expect(find.text('Voice bundle plan needs attention'), findsOneWidget);
    expect(
      find.text('0 of 2 existing Voice slots pass this bundle plan.'),
      findsOneWidget,
    );
    expect(find.text('Show 2 blockers'), findsOneWidget);
    expect(find.text('Resolve this Voice target.'), findsNothing);

    await tester.tap(
      find.byKey(const Key('revision3-voice-readiness-toggle-blockers')),
    );
    await tester.pumpAndSettle();

    expect(find.text('Resolve this Voice target.'), findsOneWidget);
    expect(find.text('Select an approved Voice take.'), findsOneWidget);
    expect(find.text('Asghan greeting — de'), findsOneWidget);
    expect(find.text('Asghan greeting — de-x1'), findsOneWidget);
    expect(find.text(revision3VoiceFixtureLineId), findsNothing);
    expect(find.text('GRD_263_ASGHAN_OPEN_INFO_06_02'), findsNothing);

    await tester.tap(
      find.byKey(const ValueKey('revision3-voice-readiness-blocker-action-0')),
    );
    await tester.pumpAndSettle();
    expect(opened, <(String, String, String)>[
      ('resolve', revision3VoiceFixtureLineId, 'de'),
    ]);
    expect(planCalls, 2);

    await tester.tap(
      find.byKey(const Key('revision3-voice-readiness-toggle-blockers')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('revision3-voice-readiness-blocker-action-1')),
    );
    await tester.pumpAndSettle();
    expect(opened.last, ('manage', revision3VoiceFixtureLineId, 'de-x1'));
    expect(planCalls, 3);
  });

  testWidgets('same-head parent rebuild preserves a pending action guard', (
    tester,
  ) async {
    final action = Completer<void>();
    var actionCalls = 0;
    var planCalls = 0;
    var hostRevision = 0;
    late StateSetter updateHost;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              updateHost = setState;
              return Column(
                children: [
                  Revision3VoiceBuildReadinessPanel(
                    projectRoot: _projectRoot,
                    projectId: _projectId,
                    projectRevision: 7,
                    checkpointIdentity: _head.canonicalJson,
                    plan: () async {
                      planCalls += 1;
                      return _blockedPlan();
                    },
                    onResolveVoiceTarget:
                        ({required initialLineId, required initialLocale}) {
                          actionCalls += 1;
                          return action.future;
                        },
                  ),
                  SizedBox(width: hostRevision.toDouble()),
                ],
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-voice-readiness-toggle-blockers')),
    );
    await tester.pumpAndSettle();
    final blockerAction = find.byKey(
      const ValueKey('revision3-voice-readiness-blocker-action-0'),
    );
    await tester.tap(blockerAction);
    await tester.pump();
    expect(actionCalls, 1);
    expect(tester.widget<TextButton>(blockerAction).onPressed, isNull);

    updateHost(() => hostRevision += 1);
    await tester.pump();
    expect(tester.widget<TextButton>(blockerAction).onPressed, isNull);
    expect(actionCalls, 1);

    action.complete();
    await tester.pumpAndSettle();
    expect(planCalls, 2);
    expect(
      find.byKey(const Key('revision3-voice-readiness-action-error')),
      findsNothing,
    );
  });

  testWidgets('shows global blockers without inventing a deep link', (
    tester,
  ) async {
    await _pumpPanel(tester, plan: () async => _noSlotsPlan());
    await tester.pumpAndSettle();

    expect(find.text('Show 1 blocker'), findsOneWidget);
    await tester.tap(
      find.byKey(const Key('revision3-voice-readiness-toggle-blockers')),
    );
    await tester.pumpAndSettle();

    expect(find.text('No Voice setups exist in this project.'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('revision3-voice-readiness-blocker-action-0')),
      findsNothing,
    );
  });

  testWidgets('offers Build only when game and callback are available', (
    tester,
  ) async {
    var buildCalls = 0;

    await _pumpPanel(
      tester,
      plan: () async => _readyPlan(),
      gameConfigured: true,
    );
    await tester.pumpAndSettle();
    expect(
      find.textContaining(
        'creating the offline Voice bundle remains a separate action',
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-voice-readiness-build')),
      findsNothing,
    );

    await _pumpPanel(
      tester,
      plan: () async => _readyPlan(),
      gameConfigured: true,
      onBuild: () => buildCalls += 1,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-voice-readiness-build')));
    await tester.pumpAndSettle();
    expect(buildCalls, 1);
  });

  testWidgets('tuple changes reload and stale completions are ignored', (
    tester,
  ) async {
    final first = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    final second = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    var calls = 0;
    var projectId = _projectId;
    var projectRevision = 7;
    late StateSetter updateHost;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              updateHost = setState;
              return Revision3VoiceBuildReadinessPanel(
                projectRoot: _projectRoot,
                projectId: projectId,
                projectRevision: projectRevision,
                checkpointIdentity: _head.canonicalJson,
                plan: () {
                  calls += 1;
                  return calls == 1 ? first.future : second.future;
                },
              );
            },
          ),
        ),
      ),
    );
    await tester.pump();

    updateHost(() {
      projectId = _otherProjectId;
      projectRevision = 8;
    });
    await tester.pump();
    expect(calls, 2);

    first.complete(_readyPlan());
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-voice-readiness-loading')),
      findsOneWidget,
    );

    second.complete(_readyPlan(projectId: _otherProjectId, revision: 8));
    await tester.pumpAndSettle();
    expect(find.text('Exact project revision 8'), findsOneWidget);
    expect(find.text('Exact project revision 7'), findsNothing);
  });

  testWidgets('rejects a plan for a different project tuple', (tester) async {
    await _pumpPanel(
      tester,
      projectId: _otherProjectId,
      projectRevision: 8,
      plan: () async => _readyPlan(),
      gameConfigured: true,
      onBuild: () {},
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-voice-readiness-error')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-voice-readiness-build')),
      findsNothing,
    );
  });

  testWidgets('same-revision canonical-head change reloads exact readiness', (
    tester,
  ) async {
    final controller = Revision3VoiceBuildReadinessController();
    final first = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    final second = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    var calls = 0;
    var checkpointIdentity = _head.canonicalJson;
    late StateSetter updateHost;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              updateHost = setState;
              return Revision3VoiceBuildReadinessPanel(
                projectRoot: _projectRoot,
                projectId: _projectId,
                projectRevision: 7,
                checkpointIdentity: checkpointIdentity,
                controller: controller,
                plan: () {
                  calls += 1;
                  return calls == 1 ? first.future : second.future;
                },
              );
            },
          ),
        ),
      ),
    );
    await tester.pump();

    updateHost(() => checkpointIdentity = _otherHead.canonicalJson);
    await tester.pump();
    expect(calls, 2);
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.loading,
    );
    expect(controller.snapshot.planOutcome, isNull);
    expect(
      controller.snapshot.checkpoint?.checkpointIdentity,
      _otherHead.canonicalJson,
    );

    first.complete(_readyPlan());
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-voice-readiness-loading')),
      findsOneWidget,
    );
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.loading,
    );

    second.complete(_readyPlan(head: _otherHead));
    await tester.pumpAndSettle();
    expect(find.text('Voice bundle plan checked'), findsOneWidget);
    expect(
      controller.snapshot.planOutcome,
      AuthoringRevision3VoiceBuildPlanOutcome.ready,
    );
    expect(
      find.byKey(const Key('revision3-voice-readiness-error')),
      findsNothing,
    );
  });

  testWidgets('same-revision head drift invalidates a pending old action', (
    tester,
  ) async {
    final action = Completer<void>();
    final replacement = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    var calls = 0;
    var checkpointIdentity = _head.canonicalJson;
    late StateSetter updateHost;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              updateHost = setState;
              return Revision3VoiceBuildReadinessPanel(
                projectRoot: _projectRoot,
                projectId: _projectId,
                projectRevision: 7,
                checkpointIdentity: checkpointIdentity,
                plan: () {
                  calls += 1;
                  if (calls == 1) return Future.value(_blockedPlan());
                  return replacement.future;
                },
                onResolveVoiceTarget:
                    ({required initialLineId, required initialLocale}) =>
                        action.future,
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-voice-readiness-toggle-blockers')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('revision3-voice-readiness-blocker-action-0')),
    );
    await tester.pump();

    updateHost(() => checkpointIdentity = _otherHead.canonicalJson);
    await tester.pump();
    expect(calls, 2);

    action.complete();
    await tester.pump();
    expect(calls, 2);
    expect(
      find.byKey(const Key('revision3-voice-readiness-action-error')),
      findsNothing,
    );

    replacement.complete(_readyPlan(head: _otherHead));
    await tester.pumpAndSettle();
    expect(find.text('Voice bundle plan checked'), findsOneWidget);
  });

  testWidgets('rejects a plan from another canonical head', (tester) async {
    final controller = Revision3VoiceBuildReadinessController();
    await _pumpPanel(
      tester,
      checkpointIdentity: _otherHead.canonicalJson,
      plan: () async => _readyPlan(),
      controller: controller,
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-voice-readiness-error')),
      findsOneWidget,
    );
    expect(find.text('Voice bundle plan checked'), findsNothing);
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.unavailable,
    );
    expect(controller.snapshot.planOutcome, isNull);
  });

  testWidgets(
    'controller observes one panel plan and ignores game or build affordances',
    (tester) async {
      final controller = Revision3VoiceBuildReadinessController();
      final planned = Completer<AuthoringRevision3VoiceBuildPlanResult>();
      var planCalls = 0;

      Future<AuthoringRevision3VoiceBuildPlanResult> plan() {
        planCalls += 1;
        return planned.future;
      }

      await _pumpPanel(tester, plan: plan, controller: controller);

      expect(planCalls, 1);
      expect(
        controller.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.loading,
      );
      expect(
        controller.snapshot.checkpoint,
        Revision3VoiceBuildReadinessCheckpoint(
          projectRoot: _projectRoot,
          projectId: _projectId,
          projectRevision: 7,
          checkpointIdentity: _head.canonicalJson,
        ),
      );

      planned.complete(_readyPlan());
      await tester.pumpAndSettle();
      expect(
        controller.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.ready,
      );
      expect(
        controller.snapshot.planOutcome,
        AuthoringRevision3VoiceBuildPlanOutcome.ready,
      );

      await _pumpPanel(
        tester,
        plan: plan,
        controller: controller,
        gameConfigured: true,
        onBuild: () {},
      );
      await tester.pump();

      expect(planCalls, 1);
      expect(
        controller.snapshot.planOutcome,
        AuthoringRevision3VoiceBuildPlanOutcome.ready,
      );
    },
  );

  testWidgets('refresh clears evidence before failure and retry', (
    tester,
  ) async {
    final controller = Revision3VoiceBuildReadinessController();
    final refresh = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    final retry = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    var calls = 0;

    await _pumpPanel(
      tester,
      controller: controller,
      plan: () {
        calls += 1;
        return switch (calls) {
          1 => Future.value(_readyPlan()),
          2 => refresh.future,
          _ => retry.future,
        };
      },
    );
    await tester.pumpAndSettle();
    expect(
      controller.snapshot.planOutcome,
      AuthoringRevision3VoiceBuildPlanOutcome.ready,
    );

    await tester.tap(
      find.byKey(const Key('revision3-voice-readiness-refresh')),
    );
    await tester.pump();
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.loading,
    );
    expect(controller.snapshot.planOutcome, isNull);

    refresh.completeError(StateError('private failure detail'));
    await tester.pumpAndSettle();
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.unavailable,
    );
    expect(controller.snapshot.planOutcome, isNull);

    await tester.tap(find.byKey(const Key('revision3-voice-readiness-retry')));
    await tester.pump();
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.loading,
    );

    retry.complete(_blockedPlan());
    await tester.pumpAndSettle();
    expect(
      controller.snapshot.planOutcome,
      AuthoringRevision3VoiceBuildPlanOutcome.blocked,
    );
  });

  testWidgets('requires-reopen clears evidence and recovery replans', (
    tester,
  ) async {
    final controller = Revision3VoiceBuildReadinessController();
    final recovered = Completer<AuthoringRevision3VoiceBuildPlanResult>();
    var requiresReopen = false;
    var calls = 0;
    late StateSetter updateHost;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              updateHost = setState;
              return Revision3VoiceBuildReadinessPanel(
                projectRoot: _projectRoot,
                projectId: _projectId,
                projectRevision: 7,
                checkpointIdentity: _head.canonicalJson,
                controller: controller,
                requiresReopen: requiresReopen,
                plan: () {
                  calls += 1;
                  return calls == 1
                      ? Future.value(_readyPlan())
                      : recovered.future;
                },
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      controller.snapshot.planOutcome,
      AuthoringRevision3VoiceBuildPlanOutcome.ready,
    );
    expect(calls, 1);

    updateHost(() => requiresReopen = true);
    await tester.pump();
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.unavailable,
    );
    expect(controller.snapshot.planOutcome, isNull);
    expect(calls, 1);
    expect(
      find.byKey(const Key('revision3-voice-readiness-error')),
      findsOneWidget,
    );

    updateHost(() => requiresReopen = false);
    await tester.pump();
    expect(calls, 2);
    expect(
      controller.snapshot.state,
      Revision3VoiceBuildReadinessLoadState.loading,
    );

    recovered.complete(_readyPlan());
    await tester.pumpAndSettle();
    expect(
      controller.snapshot.planOutcome,
      AuthoringRevision3VoiceBuildPlanOutcome.ready,
    );
  });

  testWidgets(
    'controller replacement, root drift, disposal, and late loads fail closed',
    (tester) async {
      final firstController = Revision3VoiceBuildReadinessController();
      final secondController = Revision3VoiceBuildReadinessController();
      final first = Completer<AuthoringRevision3VoiceBuildPlanResult>();
      final second = Completer<AuthoringRevision3VoiceBuildPlanResult>();
      final afterDispose = Completer<AuthoringRevision3VoiceBuildPlanResult>();
      var controller = firstController;
      var projectRoot = _projectRoot;
      var calls = 0;
      late StateSetter updateHost;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                updateHost = setState;
                return Revision3VoiceBuildReadinessPanel(
                  projectRoot: projectRoot,
                  projectId: _projectId,
                  projectRevision: 7,
                  checkpointIdentity: _head.canonicalJson,
                  controller: controller,
                  plan: () {
                    calls += 1;
                    return switch (calls) {
                      1 => first.future,
                      2 => second.future,
                      _ => afterDispose.future,
                    };
                  },
                );
              },
            ),
          ),
        ),
      );
      await tester.pump();
      expect(
        firstController.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.loading,
      );

      updateHost(() => controller = secondController);
      await tester.pump();
      expect(
        firstController.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.detached,
      );
      expect(
        secondController.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.loading,
      );
      expect(calls, 1);

      updateHost(() => projectRoot = r'C:\mods\voice-readiness-replaced');
      await tester.pump();
      expect(calls, 2);
      expect(
        secondController.snapshot.checkpoint?.projectRoot,
        r'C:\mods\voice-readiness-replaced',
      );

      first.complete(_readyPlan());
      await tester.pump();
      expect(
        secondController.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.loading,
      );
      expect(secondController.snapshot.planOutcome, isNull);

      second.complete(_readyPlan());
      await tester.pumpAndSettle();
      expect(
        secondController.snapshot.planOutcome,
        AuthoringRevision3VoiceBuildPlanOutcome.ready,
      );

      await tester.tap(
        find.byKey(const Key('revision3-voice-readiness-refresh')),
      );
      await tester.pump();
      expect(
        secondController.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.loading,
      );

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      expect(
        secondController.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.detached,
      );
      expect(secondController.snapshot.planOutcome, isNull);

      afterDispose.complete(_readyPlan());
      await tester.pumpAndSettle();
      expect(
        secondController.snapshot.state,
        Revision3VoiceBuildReadinessLoadState.detached,
      );
      firstController.dispose();
      secondController.dispose();
    },
  );
}

Future<void> _pumpPanel(
  WidgetTester tester, {
  required Future<AuthoringRevision3VoiceBuildPlanResult> Function() plan,
  String projectId = _projectId,
  String projectRoot = _projectRoot,
  int projectRevision = 7,
  String? checkpointIdentity,
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
  FutureOr<void> Function()? onBuild,
  bool gameConfigured = false,
  bool requiresReopen = false,
  Revision3VoiceBuildReadinessController? controller,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3VoiceBuildReadinessPanel(
        projectRoot: projectRoot,
        projectId: projectId,
        projectRevision: projectRevision,
        checkpointIdentity: checkpointIdentity ?? _head.canonicalJson,
        plan: plan,
        onResolveVoiceTarget: onResolveVoiceTarget,
        onManageVoiceTakes: onManageVoiceTakes,
        onBuild: onBuild,
        gameConfigured: gameConfigured,
        requiresReopen: requiresReopen,
        controller: controller,
      ),
    ),
  ),
);

AuthoringRevision3VoiceBuildPlanResult _readyPlan({
  String projectId = _projectId,
  int revision = 7,
  AuthoringWorkingHead? head,
}) => AuthoringRevision3VoiceBuildPlanResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'ready',
    'basis_head_json': (head ?? _head).canonicalJson,
    'project_id': projectId,
    'project_revision': revision,
    'total_slots': 2,
    'ready_slots': 2,
    'blockers': const <Object?>[],
    'plan_authority': 'read_only_voice_build_plan_v1',
    'build_authority': 'not_granted',
    'deployment_status': 'not_performed',
  },
  expectedHead: head ?? _head,
  expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
    slotCount: 2,
    projectId: projectId,
    projectRevision: revision,
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

AuthoringRevision3VoiceBuildPlanResult _noSlotsPlan() =>
    AuthoringRevision3VoiceBuildPlanResult.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'blocked',
        'basis_head_json': _head.canonicalJson,
        'project_id': _projectId,
        'project_revision': 7,
        'total_slots': 0,
        'ready_slots': 0,
        'blockers': const <Object?>[
          <String, Object?>{'reason': 'no_voice_slots'},
        ],
        'plan_authority': 'read_only_voice_build_plan_v1',
        'build_authority': 'not_granted',
        'deployment_status': 'not_performed',
      },
      expectedHead: _head,
      expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
        slotCount: 0,
        projectId: _projectId,
      ),
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
