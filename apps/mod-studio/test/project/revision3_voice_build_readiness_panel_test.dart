import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_voice_build_readiness_panel.dart';

import '../support/revision3_voice_fixture.dart';

const _projectId = '11111111111111111111111111111111';
const _otherProjectId = '22222222222222222222222222222222';

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
    expect(find.textContaining('Voice slots are ready'), findsNothing);

    planned.complete(_readyPlan());
    await tester.pumpAndSettle();

    expect(find.text('Voice is ready'), findsOneWidget);
    expect(find.text('2 of 2 Voice slots are ready.'), findsOneWidget);
    expect(
      find.textContaining('Configure the game installation'),
      findsOneWidget,
    );
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

    expect(find.text('Voice needs attention'), findsOneWidget);
    expect(find.text('0 of 2 Voice slots are ready.'), findsOneWidget);
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
    expect(find.textContaining('Open Build & Release'), findsOneWidget);
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
                projectId: projectId,
                projectRevision: projectRevision,
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
}

Future<void> _pumpPanel(
  WidgetTester tester, {
  required Future<AuthoringRevision3VoiceBuildPlanResult> Function() plan,
  String projectId = _projectId,
  int projectRevision = 7,
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
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3VoiceBuildReadinessPanel(
        projectId: projectId,
        projectRevision: projectRevision,
        plan: plan,
        onResolveVoiceTarget: onResolveVoiceTarget,
        onManageVoiceTakes: onManageVoiceTakes,
        onBuild: onBuild,
        gameConfigured: gameConfigured,
      ),
    ),
  ),
);

AuthoringRevision3VoiceBuildPlanResult _readyPlan({
  String projectId = _projectId,
  int revision = 7,
}) => AuthoringRevision3VoiceBuildPlanResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'ready',
    'basis_head_json': _head.canonicalJson,
    'project_id': projectId,
    'project_revision': revision,
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
